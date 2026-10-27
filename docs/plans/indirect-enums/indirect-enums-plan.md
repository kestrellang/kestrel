# Indirect Enums Implementation Plan (0.19)

**Status:** rev 3 — all review questions (Q1–Q6) answered (see [Decisions](#decisions-from-review)). **Design inputs:** [shared-box.md](../../design/shared-box.md)
(`indirect enum` client row), [enums.md](../../language/enums.md) §Recursive Enums.
**Method:** TDD — Phase 0 baselines the test matrix red before any compiler change.

## Decision summary

`indirect enum` gets **Rust `Box` semantics**, not Swift's immutable shared box
and not CowBox:

| property | rule |
|---|---|
| storage | every **recursive** case (its payload reaches the enum by value — A6) stores its whole payload in one `OwnedBox[(f0, f1, …)]`, one allocation per node; non-recursive cases (`.Leaf(value: T)`, `.Nil`) stay inline |
| mutability | fully mutable; single owner, so `&mutating` reaches the heap payload directly with no uniqueness check or fork |
| `Copyable` | not a special rule: inferred from fields like any aggregate. A boxed case's physical field is `OwnedBox`, which is not Copyable (a bit-copy would make two owners of one heap block → double free), so the fold lands on Cloneable or not Copyable. An indirect enum with no recursive case boxes nothing and can be Copyable |
| `Cloneable` | exactly when every payload field is Cloneable (self-references assumed Cloneable, coinductively); clone is a deep copy |
| move-only payloads | allowed — the enum is then `not Copyable` and not Cloneable, like any aggregate holding a move-only field |
| surface | boxing is invisible: construction, patterns, and types are written as for a plain enum |

Why not CowBox: `CowBox[T]` requires `T: Cloneable` on the struct, which rejects
move-only payloads (`recursive_indirect_enum_deinit.ks`) and silently constrains
generic `List[T]`. Why not an immutable shared box: payloads must stay mutable.
The cost of the chosen design is that copying a list/tree (an implicit clone) is
O(n); O(1) structural sharing is deferred to an opt-in `indirect[CowBox]` (see
[Deferred](#deferred)), mirroring `any P` vs `any[CowBox] P`.

## Architecture

### A1. Box at the MIR item boundary, not in the type system

Above MIR (HIR, type inference, move checking, borrow analysis) an indirect enum
is an ordinary enum: case signatures keep their declared field types. The box
exists only from MIR down.

`enum_lower.rs` lowers each boxed case (A6) to **one physical payload field** of type
`OwnedBox[(f0, f1, …)]` and records the logical shape beside it:

```rust
pub struct EnumCaseDef {
    pub name: String,
    pub discriminant: u32,
    /// Physical fields — what layout, drop/clone shims, mono and codegen see.
    pub payload_fields: Vec<FieldDef>,
    /// Set for an `indirect` case: the declared fields and the boxed tuple.
    pub indirect: Option<IndirectPayload>,
}
pub struct IndirectPayload {
    pub logical_fields: Vec<FieldDef>,   // names + declared types
    pub tuple_ty: TyId,                  // (f0, f1, …)
    pub box_ty: TyId,                    // OwnedBox[tuple_ty]
}
```

Consequence: layout, drop shim, clone shim, mono type collection, the verifier
and both codegen backends need **no indirect-specific code** — they see a
single-field case holding an ordinary stdlib struct. `MirTy::Pointer` is already
a layout leaf (`passes/layout.rs`), so `List[T] → OwnedBox[(T, List[T])] →
Pointer[…]` is a finite type set with pointer-sized layout. The ~30
`payload_fields` consumers stay untouched; only MIR lowering of construction and
patterns translates logical ↔ physical, through one helper each (A3, A4).

Rejected: a type-level desugar (rewriting case types to `OwnedBox[…]` in HIR).
It leaks the box into inference, diagnostics, and the move checker, and every
pattern would need surface-level unwrapping.

### A2. Box binding

Resolve the box through the existing `Builtin::OwnedBox` binding, reusing
`closure_box.rs`'s `resolve_box_common` (+ `find_box_member`) to get the
specialized entity, its `init(consuming:)`, `takeValue()`, `pointeeRef()` and
`pointeeMutRef()`. No new lang item: a future `indirect[B]` names its box
explicitly. If `OwnedBox` is unbound (`stdlib: false`), see Q3.

### A3. Construction

`emit_enum_variant` is the single choke point. For an indirect case it builds
`Tuple { payload }`, calls the box `init(consuming:)`, and emits
`Enum { payload: [box] }`. The existing ref-slot / ownership handling runs on
the logical elements before they are tupled.

### A4. Pattern matching

`resolve_enum_payload_field{,_by_index}` answer in **logical** terms (name →
logical index + declared type). One new helper,
`project_indirect_payload(operand, variant, mode)`, turns the scrutinee into the
payload tuple, after which today's tuple destructuring applies:

| match mode (existing `arms_ref_binding_mode`) | scrutinee | projection |
|---|---|---|
| value mode | owned (consumed) | `EnumPayload` → box → `takeValue()` → `DestructureTuple` (moves fields out, frees block) |
| value mode | guaranteed (borrowed) | box → `pointeeRef()` → `TupleExtract` + copy/clone of bound fields (same as a borrowed plain enum today) |
| place mode `&` | any | box → `pointeeRef()` → field refs |
| place mode `&mutating` | mutable place | box → `pointeeMutRef()` → field refs; writes land in the heap payload |

`if let` / `guard let` / `while let` / destructuring `let` all route through the
same pattern lowering, so they inherit this.

### A5. Copy semantics

`NominalCopySemantics` (kestrel-semantics) must see a boxed case's fields as the
`OwnedBox` they lower to: the box contributes "not Copyable, Cloneable iff the
payload tuple is". Self-references resolve through query cycle recovery (A7) —
the initial assumption is the top of the lattice and the fixpoint descends.
This also fixes the "unbounded recursion in CopyBehavior over indirect enum"
bug noted in the old `execution_graph/enums/linked_list.ks`.

### A6. Which cases are boxed

A new kestrel-semantics query `IndirectCases { enum }` → the set of case
entities to box; the single source of truth for `enum_lower.rs` and A5. It is
the walk in `recursive_enum.rs` (`find_recursive_case`/`type_contains`) moved
into semantics, with one change: when computing the boxed set of an indirect
enum, the walk **follows other indirect enums by value** instead of stopping
at them. Otherwise mutual recursion (`indirect enum A { case X(B) }`,
`indirect enum B { case Y(A) }`) boxes neither case and the layout is
infinite. With the change both are boxed (one extra allocation, always sound).
E429 for *non*-indirect enums keeps stopping at indirect enums: any cycle
through an indirect enum's case also makes that case reach its own enum, so
the case is boxed and breaks the cycle. The analyzer calls the same query.

An `indirect enum` with no recursive case boxes nothing — warn ("`indirect` has
no effect"), new diagnostic ID.

### A7. Query cycle recovery (replaces the thread-local guards)

The query framework panics on re-entering a key, so recursive-type queries
dodge it with side stacks invisible to dependency tracking:
`COMPUTING_COPY_SEMANTICS` (lib.rs), `COMPUTING_STATICNESS` (staticness.rs),
the sizedness stack (sizedness.rs) and `OPAQUE_RESOLVE_STACK` (mir-lower
`ty.rs`). A memoized answer computed under "assume Copyable" is never
invalidated when the cycle's other members change — an incremental-compilation
bug, and indirect enums make recursion the common case.

Replace them with framework-level fixpoint recovery in `kestrel-hecs`
`query.rs`, in the shape of Salsa's cycle handling:

- `QueryFn` gains an opt-in `fn cycle_mode(&self) -> CycleMode<Self::Output>`:
  `Panic` (default, today's behavior), `Fixpoint { initial }` (iterate to a
  fixpoint), or `Fallback { value }` (every member of the cycle answers
  `value`, no iteration — for cycles that are genuinely errors).
- Re-entering an opted-in key on the stack returns its provisional value
  (initially `cycle_initial`) and marks the cycle head.
- The head re-executes until its output stops changing; members computed
  under a provisional value are memoized only once the head converges, and
  record dependencies on every member so ordinary invalidation reruns the
  whole cycle.
- Termination: outputs must be a finite lattice walked monotonically
  (copy semantics: Copyable → Cloneable → NotCopyable). Debug-assert
  monotonicity; hard iteration cap → ICE.

All four side stacks migrate in 0.19 (Q6), and each thread-local is deleted:

| guard | query | mode | initial / fallback | lattice |
|---|---|---|---|---|
| `COMPUTING_COPY_SEMANTICS` | `NominalCopySemantics` | Fixpoint | Copyable (today's self-reference assumption) | Copyable → Cloneable → NotCopyable |
| `COMPUTING_STATICNESS` | `NominalStaticness` | Fixpoint | Static (today's assumption) | Static → conditional → not Static |
| `COMPUTING_SIZEDNESS` | `NominalSizedness` | Fixpoint | `Sized` | `Sized` → `ConditionalOn(positions)` with positions only growing (bounded by the param count) |
| `OPAQUE_RESOLVE_STACK` | **new** `OpaqueConcreteTy { origin }` | Fallback | `error()` | n/a |

The opaque guard is different in kind: it guards a plain recursive function
(`lower_resolved_ty` lowering an opaque type whose concrete type mentions
itself), not query re-entry. Migrating it means lifting "concrete type of this
opaque origin, lowered" into its own query so the framework can see the cycle.
A cycle there is a user error (`func f() -> some P { f() }`), so the fallback
also emits a diagnostic instead of silently yielding `error()` — new ID, owned
by the analyzer that already reports opaque-type errors.

Migration order: copy semantics (indirect enums need it), then staticness and
sizedness (same shape, mechanical), then opaque (new query + diagnostic). Each
is its own commit with the full suite green, so one regression doesn't block
the rest.

MIR `copy_behavior_in` reads `type_info.copy`, which is fed from that query, and
the physical field is `OwnedBox`, whose own conditional `Cloneable` agrees — so
the lockstep sites (`expand.rs ty_needs_drop`, `audit.rs mono_needs_drop`,
`clone_shim.rs ty_needs_clone_shim`, `mono/mod.rs concrete_copy`) should need no
change. Phase 2 tests prove it.

## Phases

**Phase 0 — Tests (red).** Delete the existing E465 tests (all of them, per
review) and write new ones, `stdlib: true`, under
`testdata/declarations/enums/indirect/` and `testdata/execution_graph/enums/indirect/`:
construction + `print` of a list and tree; borrowed recursive `sum`/`depth`;
consuming match that rebuilds (`reverse`, `map`); in-place mutation through a
`&mutating` binder (`insert` into a BST); `mutating func` that reassigns
`self = .Node(…)`; generic `List[T]` with Copyable, Cloneable (String) and
move-only `T`; clone independence (mutate a clone, original unchanged); deinit
counts (existing `recursive_indirect_enum_deinit.ks`); mutual recursion through
a struct (`Tree` ↔ `Forest`) and between two indirect enums; non-recursive
cases staying inline (a `.Leaf(Int64)` holds no box: Copyable payload, deinit
count); an indirect enum with no recursive case (warning, Copyable); one
`stdlib: false` test for the missing-stdlib error; a 100k-node list drop
(documents the known stack overflow); polymorphic recursion
(`case Nest(Nested[[T]])`) rejected or bounded.

**Phase 1a — Cycle recovery** (A7) in `kestrel-hecs`, with framework unit tests
(direct cycle, mutual cycle, invalidation of one member reruns the cycle,
non-opted query still panics, `Fallback` answers every member). Then migrate
all four guards in the A7 order, deleting each thread-local; the full suite
must stay green after each one. Recursive-type tests for staticness and
sizedness (struct containing itself through a box, generic self-reference) and
a recursive opaque-return test land first, red where they expose today's
stale-memo behavior.

**Phase 1b — Front end.** Remove `IndirectEnumAnalyzer` and retire E465 (ID stays
reserved). Add `IndirectCases` (A6) and point E429 at it. Make A5 see boxed
fields. New diagnostics: `indirect` with no recursive case (warning); explicit
`indirect enum …: Copyable` whose boxed case makes that impossible (reuse the
existing "contains non-Copyable field" shape); `indirect enum` without a
stdlib `OwnedBox` binding ("indirect enums require the standard library").

**Phase 2 — MIR item.** `IndirectPayload` + `enum_lower.rs` (A1, A2). Verify
layout, drop and clone shims with the deinit-count and clone-independence tests
before touching bodies.

**Phase 3 — Construction** (A3).

**Phase 4 — Matching** (A4): borrowed value mode and `&` first, then consuming,
then `&mutating`.

**Phase 5 — Docs.** `enums.md` (replace the *Future* section and E465 entry),
`shared-box.md` client table (indirect enum → `OwnedBox`, not CoW), a
`docs/language` note on clone cost.

**Risks.** (1) MIR lowering calling stdlib methods on a synthesized tuple type
inside pattern lowering — mitigated by reusing `closure_box.rs`, which already
does this for closure environments. (2) The ownership verifier around
`takeValue()` in consuming arms with guards/fallthrough. (3) Polymorphic
recursion making mono non-terminating — mono has no instantiation depth limit
today.

**Estimate:** ~1–1.5 weeks for the feature (Phase 4 is half of it) plus ~1–1.5 weeks for Phase 1a (framework ~3–5 days, then four migrations; opaque is the largest because it adds a query and a diagnostic).

## Decisions from review

- **Q1. Copyable.** No `indirect`-specific rule: copy semantics fold from the
  physical fields, so a boxed case makes the enum non-Copyable (a bit-copy would
  double-own the heap block), and an enum with no boxed case stays whatever its
  fields say. No annotation is required.
- **Q2. Box only recursive cases** (A6). Per-case `indirect case` syntax stays
  deferred.
- **Q3. Delete the existing indirect tests** and write a new matrix. Without a
  stdlib `OwnedBox`, `indirect enum` is an error.
- **Q4. Deep drop recursion** ships as a known issue with a test documenting it.
- **Q5. Replace the re-entrancy guard now** with query cycle recovery (A7).
- **Q6. Migrate all four guards** (copy semantics, staticness, sizedness,
  opaque types) in 0.19.

## Deferred

- `indirect[B] enum` — caller-chosen box; `indirect[CowBox]` for O(1) sharing
  with copy-on-write mutation.
- Per-case `indirect case`.
- Iterative (non-recursive) drop of long chains.
