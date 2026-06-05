# Conformance Specialization — Most-Specific-Wins for Overlapping Conformances

**Status**: Implemented (Parts A + B; full suite 3080 pass / 0 fail)
**Motivation**: [#110](https://github.com/kestrellang/kestrel/issues/110) (`Exitable`) — the
`Result[(), E]` + generic `Result[T, E]` overlap that forced a v1 scope-down
**Related memory**: `overlapping_generic_specialized_conformance_ice`

> **Two features in one subsystem.** Part A makes overlapping generic +
> specialized *conformances* dispatch to the most specific one. Part B makes
> structural (`()`, `!`) and intrinsic (`lang.*`) types `extend`-able at all.
> They share the Entity-keyed extension/conformance machinery, and Part B lets
> `Exitable` drop its `()` / `!` wrapper special-casing (future follow-up).
>
> **As implemented.** Part A needed a second fix the design missed: the witness
> selector was inert because `ConformingProtocolInstantiations` **deduped**
> overlapping conformances by `(protocol, protocol-args)`, collapsing
> `extend Box[T]: P` + `extend Box[lang.i64]: P` into one source. The dedup key
> now also includes the source's implementing-type args. Part B threads
> structural/intrinsic types through synthetic `lang` entities (`lang.()`,
> `lang.!`, and the existing `lang.iN`): name-res `ExtensionTargetEntity` +
> `resolve_lang_child`, inference `conforms_to`/`resolve_member`, and MIR
> `try_lang_primitive` (entity → `Tuple([])`/`Never`/`I64`) all map the
> structural/intrinsic type to its entity, so the Entity-keyed pipeline works
> unchanged. Direct calls AND witness dispatch both work.

## Summary

Today, declaring both a generic conformance and a partial specialization for the
same type + protocol does **not** work:

```kestrel
extend Box[T]:        Marked { func mark() -> lang.i64 { 1 } }   // generic
extend Box[lang.i64]: Marked { func mark() -> lang.i64 { 2 } }   // specialized
```

A polymorphic call `markOf(Box[lang.i64](...))` (where `func markOf[T](x: T) ->
... where T: Marked`) dispatches to the **generic** body (`1`), not the
specialized one (`2`). When the generic is guarded by a `where`-clause the
specialization is meant to fill (the `Result[(), E]` case), routing through the
generic body instead produces a hard ICE (`Callee::Witness not resolved`).

We want **most-specific-wins**: among all conformances whose pattern matches a
concrete self type, dispatch selects the strictly most specific one
(`Box[lang.i64]` over `Box[T]`, `Result[(), E]` over `Result[T, E]`), and a
genuine tie is a declaration-time coherence error.

## Motivation

`Exitable` (#110) wanted both `extend Result[T, E]: Exitable where T: Exitable`
(so `main() -> NonUnit throws E` works) and `extend Result[(), E]: Exitable` (the
common `main() throws E` ⇒ `Result[(), E]`). They overlap, and the engine ICEs,
so v1 shipped **only** the unit-specialized conformance — leaving
`main() -> NonUnit throws E` unsupported. This feature removes that limitation
and generalizes to any `extend X[Concrete]: P` overriding `extend X[T]: P`.

Five documentation tests already pin the contract
(`testdata/declarations/extensions/most_specific/`):

| Test | Selector path | Today |
|---|---|---|
| `inherent_direct_specialized_wins` | analyzer member resolution | ✅ passes |
| `two_param_specificity_ladder` | analyzer member resolution | ✅ passes |
| `protocol_direct_specialized_wins` | analyzer (conformance) | ❌ picks generic |
| `witness_dispatch_specialized_wins` | mono witness dispatch | ❌ picks generic |
| `where_clause_disjoint_specialized_wins` | mono witness dispatch | ❌ ICE |

The asymmetry is the key diagnosis: **inherent** direct calls already resolve
most-specific (analyzer `TypeMembers`); **protocol-conformance** dispatch does
not, at either the analyzer or the mono layer.

## Background — how conformance dispatch works today

A protocol method call lowers to a `Callee::Witness { protocol, method,
self_type }` (e.g. `lib/kestrel-mir-lower/src/body/expr.rs`). Monomorphization
resolves it against `MirModule.witnesses` via
`lib/kestrel-mir/src/mono/witness.rs::resolve_witness_call` →
`find_witness_with_method`.

Two facts make specialization impossible today:

1. **Witnesses are keyed by the nominal *entity* with the *generic* implementing
   type.** `lib/kestrel-mir-lower/src/items/witness_lower.rs::lower_witnesses`
   (`:22-54`) iterates per struct/enum entity, building `impl_ty =
   named(entity, [TypeParam(T)…])` — always `Box[T]`. `lower_witnesses_for_type`
   does `WitnessDef::new(*protocol, impl_ty)` (`:108`) for **every** conformance
   of that entity. The concrete `[lang.i64]` of `extend Box[lang.i64]: P` is
   **discarded**; both extends collapse into one `WitnessDef` whose
   `implementing_type` is `Box[T]`.

2. **One witness ⇒ one method binding.** That single `WitnessDef` binds one impl
   per method, chosen first-match by `find_impl_among` (`witness_lower.rs`, also
   first-match). A single binding physically cannot encode "`Box[i64].mark → 2`
   but `Box[str].mark → 1`."

So the mono selector never sees two candidates to rank. **0.15 was identical** —
`release/0.15` keys witnesses the same way (`witness_lower.rs:109`,
`WitnessDef::new(impl_ty, protocol)` with the generic `impl_ty`) and its mono
`find_witness_with_method` is explicitly first-match ("fall back to the old
first-match behavior"). Its `more_specialized_wins` / `specialized_extension_wins`
tests are `// test: diagnostics` (inherent methods) that only assert overlap is
*accepted*, never that the right impl *runs*. This is **net-new** work, not a
regression or a port.

## Design

### Part A.1 — Most-specific selection in mono (LANDED)

`find_witness_with_method` Pass 1 now **collects all** structurally-matching
witnesses and picks the most specific
(`lib/kestrel-mir/src/mono/witness.rs`):

```
witness_more_specific(a, b) :=
    match_pattern(b.implementing_type, a.implementing_type)   // a is an instance of b
    && !match_pattern(a.implementing_type, b.implementing_type)
```

`match_pattern` treats a pattern's `TypeParam`s as wildcards, so `b`'s `T`
matches `a`'s concrete `()`/`i64` but not vice-versa — giving `Result[(), E]` ⊏
`Result[T, E]` and `Box[i64]` ⊏ `Box[T]`. `select_most_specific` is greedy over
this partial order, which finds the unique global minimum of a chain
(`X[i64,i64]` ⊏ `X[T,i64]` ⊏ `X[T,U]`) regardless of candidate order.

This is correct, **regression-free** (full suite 3073/3 — the 3 reds are the
documentation tests), and necessary — but **inert until Part A.2** surfaces
distinct witnesses. Genuinely-incomparable overlaps are not yet flagged
(deterministic pick); see Part A.3.

### Part A.2 — Per-`extend` witnesses with concrete implementing types (THE REWORK)

Rework witness lowering so each `extend` declaration produces its **own**
`WitnessDef` carrying that extend's concrete implementing type and its own method
bindings:

- For `extend Box[T]: P` → `WitnessDef { implementing_type: Box[T], … }`
- For `extend Box[lang.i64]: P` → `WitnessDef { implementing_type: Box[lang.i64], … }`

Touch-points (`lib/kestrel-mir-lower/src/items/witness_lower.rs`):

1. **Iterate per conformance *source*, not per entity.**
   `ConformingProtocolInstantiations` already returns `(protocol, source,
   ast_type_args)` where `source` is the extension entity. When `source` is an
   `Extension`, resolve the implementing type from the **extension's target type
   annotation** (`extend Box[lang.i64]` → `Box[lang.i64]`) instead of the
   entity's generic `impl_ty`. Resolve via `lower_type` /
   `resolve_type_annotation` on the extension target.
2. **Bind each witness's methods from *its own* extension's members**, not the
   merged `TypeMembersByName` set — so the `Box[lang.i64]` witness binds the
   specialized `mark`, the `Box[T]` witness binds the generic `mark`.
3. **Carry `where`-constraints** (`WitnessDef.constraints` already exists but is
   unused by selection) so a generic conformance guarded by `where T: P` can be
   filtered when its bound is unsatisfiable for the concrete self — see A.3.

Once both witnesses exist with distinct `implementing_type`s, the landed Part A.1
selector ranks them and picks the specialization.

### Part A.3 — `where`-clause filtering & ambiguity coherence

- **`where`-clause filtering** (the `where_clause_disjoint` case): for
  `Wrap[lang.i64]`, the generic `extend Wrap[T]: P where T: P` has an unsatisfied
  bound (`lang.i64: P` doesn't hold). Selection should exclude a candidate whose
  `constraints` are unsatisfiable for the concrete self, leaving only the
  specialization. (Most-specific-wins alone also resolves this case once both
  witnesses exist, since the specialization is strictly more specific — but
  constraint filtering is the principled mechanism and is needed when a generic
  is the *only* match yet its bound fails.)
- **Ambiguity**: genuinely incomparable overlaps (e.g. `X[i64, U]` and `X[T,
  i64]` both matching `X[i64, i64]`, neither more specific) have no unique
  minimum. Detect at **declaration time** (a new coherence check, à la Rust's
  E0119) and emit an error, rather than silently picking one. This is the
  cheap-and-correct safety net; it can land before A.2 to turn today's ICE into a
  clean diagnostic.

### Part A.4 — Analyzer conformance selection (`protocol_direct`)

A direct call `b.mark()` on a concrete `Box[lang.i64]` is resolved in the
analyzer (`TypeMembers` / member resolution), not mono — and currently picks the
generic conformance's method (`protocol_direct_specialized_wins` → 1). The same
specificity ordering must apply when a member is contributed by a conformance
extension. Inherent extensions already rank correctly here (the two green tests),
so this is extending the existing analyzer ranking to conformance-sourced
members.

## Companion (Part B) — Extendible structural & intrinsic types

Asked alongside: *how hard to make `()`, `!`, and `lang.*` intrinsics
`extend`-able?* All three are blocked by the **Entity-keyed** extension system:
every discovery query assumes an extendable type has a nominal `Entity`
(`ConformingProtocols`/`ConformingProtocolInstantiations` —
`lib/kestrel-name-res/src/conformances.rs`; `ExtensionsFor` /
`ExtensionTargetEntity` — `extensions.rs`; `TypeMembers` —
`type_members.rs`; witness lowering iterates `module.structs`/`enums`).

### `lang.*` intrinsics — LOW effort

`lang.i64` etc. **already have entities** — seeded as `NodeKind::Struct` +
`Intrinsic` marker in `lib/kestrel-ast-builder/src/lang_module.rs`. Two blockers:

1. `lib/kestrel-analyze/src/decl/extension_validation.rs:86-96` rejects any
   target with the `Intrinsic` marker → E452. Relax this (allow intrinsic
   targets, or gate behind a capability).
2. Witness lowering enumerates `module.structs`/`enums`; intrinsic entities live
   in the seeded `lang` module and aren't in those maps. Include intrinsic
   entities that have conformances so `lower_witnesses` generates their witness
   tables. Member discovery via `ExtensionsFor`/`TypeMembers` already works
   (entity exists); mono `match_pattern` already handles the primitive `MirTy`s.

Net: remove one check + register intrinsic entities for witness gen. The stdlib
currently *wraps* intrinsics (`Int64 { var raw: lang.i64 }`) precisely to give
them an entity — this would make the wrapping optional for extension purposes.

### `()` and `!` — MEDIUM effort

These have **no entity at all** (`AstType::Unit` → `MirTy::Tuple([])`,
`AstType::Never` → `MirTy::Never`; `ExtensionTargetEntity` returns `None` at the
`AstType::Named` gate, `extensions.rs:47`). They're **singleton** types, so the
fix is self-contained:

1. **Mint synthetic nominal entities** for `Unit` and `Never` at world seeding
   (next to `lang_module.rs`), `NodeKind::Struct`-like, **without** the
   `Intrinsic` marker.
2. **Resolve** `AstType::Unit` / `AstType::Never` to those entities in
   `ResolveTypePath` and short-circuit `ExtensionTargetEntity` before the
   `Named` gate.
3. **Allow** them in `extension_validation.rs`.
4. **Register** them for witness lowering with `implementing_type = Tuple([])` /
   `Never`; mono `match_pattern` already matches `MirTy::Tuple`/`MirTy::Never`
   structurally, so dispatch works once a witness exists.

**Payoff:** `Exitable` could drop its `()` / `!` wrapper special-casing — they'd
just `extend (): Exitable` / `extend !: Exitable` like any type (see
exitable-main-return.md "Alternatives").

### General tuples / function types — HARD (out of scope)

`extend (A, B): P` or `extend (A) -> B: P` generically is structural/variadic
conformance — a much larger feature. Not proposed here; only the singleton `()`
and `!` are.

| Target | Has entity? | Blocker | Effort |
|---|---|---|---|
| `lang.i64` etc. | yes (`Intrinsic` struct) | E452 `is_intrinsic` check + witness-gen enumeration | **Low** |
| `()` / `!` | no | no entity; `ExtensionTargetEntity` `Named` gate | **Medium** |
| `(A,B)`, `(A)->B` | no | structural/variadic conformance | **Hard** |

## Tests

The five `most_specific/` tests pin the contract and flip green incrementally:
A.1 alone changes nothing; A.2 flips `witness_dispatch` and (with A.3)
`where_clause_disjoint`; A.4 flips `protocol_direct`; the two inherent tests stay
green throughout (regression guard). Per repo policy they currently assert the
**desired** outcome (no `#[ignore]`, no cajoled expectations) and document the
bug until the feature lands. Part B would add execution tests for `extend (): P`
/ `extend lang.i64: P`.

## Risks

- **Witness lowering is load-bearing** and has a history of collapse bugs
  (`witness_instantiation_collapse`, `apply_partial_thunk_mono_collapse`,
  `consuming_method_self_drop_skip_nominal`). Emitting more, finer-grained
  witnesses risks dedup/ordering regressions — lean on the full suite and the
  per-instantiation keying lessons.
- **Incremental compilation**: per-`extend` witnesses change the witness-set
  shape and query keys; ensure `ConformingProtocolInstantiations` /
  `ExtensionsFor` invalidation still covers added/removed specializations.
- **Mangling / mono keys** must distinguish the specialized vs generic witness
  function instances (cf. `mangler_panic_assoc_projection`).

## Alternatives

1. **Declaration-time coherence error only (no specialization).** Ship A.3's
   ambiguity/overlap detection, reject all overlaps (incl. generic+specialized)
   with a clean E-diagnostic instead of an ICE. Cheapest; turns the ICE into a
   real error but does **not** enable the feature. Good interim step.
2. **Keep specialized-only workarounds** (status quo): authors write a single
   most-specific conformance and avoid overlap. What `Exitable` v1 did.
3. **Wrap intrinsics/structural types** (status quo for Part B): require a
   nominal wrapper (`Int64`, `ExitCode`) rather than extending `lang.i64` / `()`.

## Status of work done

- **Part A.1 (mono selector)** — implemented in `lib/kestrel-mir/src/mono/witness.rs`,
  regression-free (3073/3). Foundation for A.2.
- Everything else — designed here, not yet implemented.
