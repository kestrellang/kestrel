# Indirect Enums Implementation Plan (0.19)

**Status:** draft for review. **Design inputs:** [shared-box.md](../../design/shared-box.md)
(`indirect enum` client row), [enums.md](../../language/enums.md) §Recursive Enums.
**Method:** TDD — Phase 0 baselines the test matrix red before any compiler change.

## Decision summary

`indirect enum` gets **Rust `Box` semantics**, not Swift's immutable shared box
and not CowBox:

| property | rule |
|---|---|
| storage | every non-nullary case stores its whole payload in one `OwnedBox[(f0, f1, …)]` — one allocation per node; nullary cases (`.Nil`) stay inline |
| mutability | fully mutable; single owner, so `&mutating` reaches the heap payload directly with no uniqueness check or fork |
| `Copyable` | never — an indirect enum always owns a heap block |
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

`enum_lower.rs` lowers an indirect case to **one physical payload field** of type
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

`NominalCopySemantics` (kestrel-semantics) gains an `IsIndirect` rule: never
`Copyable`; `Cloneable` iff each non-self payload field is Cloneable, treating
references back to the enum being computed as Cloneable (coinduction); else
`NotCopyable`. This also fixes the "unbounded recursion in CopyBehavior over
indirect enum" bug noted in `execution_graph/enums/linked_list.ks`.

Today that query survives recursive types via a thread-local re-entrancy stack
(`COMPUTING_COPY_SEMANTICS`) the query framework can't see, which is unsafe for
incremental invalidation. Indirect enums make recursion the common case, so the
rule above must decide self-references **structurally** (by entity identity)
rather than by hitting the guard. Whether to also replace the guard is Q5.

MIR `copy_behavior_in` reads `type_info.copy`, which is fed from that query, and
the physical field is `OwnedBox`, whose own conditional `Cloneable` agrees — so
the lockstep sites (`expand.rs ty_needs_drop`, `audit.rs mono_needs_drop`,
`clone_shim.rs ty_needs_clone_shim`, `mono/mod.rs concrete_copy`) should need no
change. Phase 2 tests prove it.

## Phases

**Phase 0 — Tests (red).** New files under `testdata/declarations/enums/indirect/`
and `testdata/execution_graph/enums/`:
construction + `print` of a list and tree; borrowed recursive `sum`/`depth`;
consuming match that rebuilds (`reverse`, `map`); in-place mutation through a
`&mutating` binder (`insert` into a BST); `mutating func` that reassigns
`self = .Node(…)`; generic `List[T]` with Copyable, Cloneable (String) and
move-only `T`; clone independence (mutate a clone, original unchanged); deinit
counts (existing `recursive_indirect_enum_deinit.ks`); mutual recursion through
a struct (`Tree` ↔ `Forest`); `indirect enum X: Copyable` rejected; a
100k-node list drop (documents Q4); polymorphic recursion
(`case Nest(Nested[[T]])`) rejected or bounded.

**Phase 1 — Front end.** Remove `IndirectEnumAnalyzer` and retire E465 (ID stays
reserved). Add the A5 rule. Add a diagnostic for `indirect enum …: Copyable`
(reuse E423's shape: "indirect enums own heap storage and cannot be Copyable").

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

**Estimate:** ~1–1.5 weeks; Phase 4 is half of it.

## Open questions

- **Q1. Implicit `not Copyable`?** An indirect enum can never be Copyable. Either
  `indirect` implies it (existing tests are written this way), or users must
  write `indirect enum List[T]: not Copyable`. *Recommend: implicit.*
- **Q2. Box every non-nullary case, or only recursive ones?** Boxing only cases
  that reach the enum (e.g. leave `.Leaf(value: T)` inline) saves an allocation
  per leaf but makes the rule depend on the recursion analysis. Per-case
  `indirect case` syntax is a separate, larger question. *Recommend: every
  non-nullary case for 0.19 (Swift's rule, predictable).*
- **Q3. `stdlib: false`.** Most existing indirect tests are `stdlib: false`, but
  `OwnedBox` lives in the stdlib. Either emit "indirect enums require the
  standard library", or flip those tests to `stdlib: true` (needs your OK, since
  it changes tests). *Recommend: the diagnostic, plus new stdlib tests.*
- **Q4. Deep drop recursion.** Dropping a long list recurses once per node and
  can overflow the stack (Swift and Rust both have this). *Recommend: ship with
  the 100k-node test documenting it; iterative drop later.*
- **Q5. Re-entrancy guard.** Fix only the indirect path structurally (A5), or
  also replace the thread-local guard with a proper cycle-recovery mechanism
  now? *Recommend: indirect path only; file the guard separately.*

## Deferred

- `indirect[B] enum` — caller-chosen box; `indirect[CowBox]` for O(1) sharing
  with copy-on-write mutation.
- Per-case `indirect case`.
- Iterative (non-recursive) drop of long chains.
