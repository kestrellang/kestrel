# kestrel-hir-lower — design notes

## Protocol `Self` is `HirTy::SelfType`

`lower_self_hir_ty` (`ty.rs`) turns `Self` into the right `HirTy` for its scope:

- `extend Box[T] { … Self … }` and `struct Box[T] { … Self … }` → `Box[T]`, with
  the scope's own type args. Without those args, `-> Self` unifies as the
  unparameterized type and fails with "expected Box got Box[i64]" at call sites.
- `extend Box[lang.i64] { … Self … }` → `Box[i64]`.
- `protocol P { … Self … }` and `extend P { … Self … }` →
  `HirTy::SelfType(P)`. This is the abstract implementing type. It carries the
  owning protocol, so every lowering site can substitute it without relying on
  ambient context. It becomes `MirTy::SelfType` and is resolved at
  monomorphization.

A bare associated type inside a protocol, like `Item` in `extend Iterator`,
means `Self.Item`.

**History, so it is not reintroduced.** Protocol `Self` used to lower to
`HirTy::Protocol(P)`, the protocol entity itself. Codegen then could not map
the projection base back to the concrete type, and layouts silently defaulted
to 8 bytes, so sub-`i64` items (`UInt8`, `Char`, `Grapheme`) read back as
garbage. That was fixed with `HirTy::SelfType`. The MIR workaround for the
`Protocol(P)` base and codegen's `resolve_assoc_type_substs` fallback are gone.
Never lower protocol `Self` to a nominal or protocol type again.

## Where a bare protocol associated type is re-rooted

One place remains that turns a bare `TypeAlias` back into a projection:
`kestrel-mir-lower/src/ty.rs::lower_named_type`. When type inference hands MIR
a bare associated type whose parent is a protocol, that function emits
`AssociatedProjection { base: SelfType(parent), … }`, rooted at **its own
protocol's** `Self`.

That is only correct for the innermost alias. Type inference must keep the
base of anything projected *off* such an alias (G26, `5242beb0`:
`solve_associated` keeps `Item.Sub` as a projection off `Item`). If it
collapses a deeper projection to a bare alias, `lower_named_type` roots it at
the wrong protocol's `Self`, and monomorphization fails with an ICE in
post-mono verify or a mangler panic. Fix the collapse in type inference; do not
add a second re-rooting special case here.

## A type written in expression position is `HirExpr::TypeRef`

`B.Item` in `B.Item.zero()` lowers to `HirExpr::TypeRef { ty }`, carrying the
full `HirTy::AssocProjection { base: Param(B), assoc }`. It is built by
`lower_type_receiver_def` / `lower_type_receiver_path` through the normal
type-path lowering. **Never** build `HirExpr::Def(alias)` for an associated type
that name resolution returned with `container: Some(_)`. `Def` has room for an
entity only, so the base is dropped. That shape caused the G17 S5 miscompile,
where mono ran `Int64.zero` for a `W` result, and the G26 crash. A `TypeRef` is
only valid as a receiver, callee or field base. See
`lib/kestrel-type-infer/AGENTS.md`.

## When adding a new `HirTy` variant

- `kestrel-mir-lower/src/ty.rs::lower_type`: lower it.
- **`HirTy::same_type`** (`kestrel-hir/src/ty.rs`). It ends in `_ => false`, so
  a variant without its own arm silently compares unequal to itself, and
  equality-clause entailment (G25 step 4) then rejects clauses it should
  accept. Add an arm that compares every field except the span.
- Every `HirTy` walker. Search for `HirTy::` match sites (name-res,
  type-infer, the analyzers, the LSP's `semantic::hir_ty_entity_at`).
