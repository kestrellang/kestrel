# Stage 0.5 — Compiler architecture

Front half of `references-prototype/references-plumbing.md` (Stages 0-4).
**Nothing past HIR changes in this stage**, and no signature representation
changes at all — `ParamConvention` is untouched (there is no ref→convention
normalization; ref-typed params are rejected, not folded — `references-gaps.md`
§10.6).

## Touched, in landing order

1. **SyntaxKind** (`lib/kestrel-syntax-tree/src/lib.rs`): append `TyRef`,
   `TyMutRef` after the last variant (rowan u16 append-only rule) +
   `kind_from_raw` entries.
2. **Parser** (`lib/kestrel-parser/src/ty/mod.rs`): `TyVariant::Ref/MutRef`,
   a `ref_type` alternative ahead of `base_ty`, `emit_ref_type*` helpers
   emitting the **atomic** `TyRef` node (`syntax.md` trap). Plus the one
   allowance for the Pointer-init decl: keyword `mutating` accepted in
   argument-**label** position (`syntax.md`).
3. **AST** (`lib/kestrel-ast/src/ast_type.rs` + builder):
   `AstType::Ref/MutRef` + CST-extraction arms.
4. **HIR** (`lib/kestrel-hir/src/ty.rs`, `lib/kestrel-hir-lower/src/ty.rs`):
   `HirTy::Ref/MutRef`; arms in `lower_ast_type`, `ast_type_span`,
   `override_span`, `contains_opaque`.
5. **Negative walk**: one ref-position validator over `HirTy` (pattern:
   `contains_opaque`) called from signature, annotation, field, and
   generic-arg lowering; diagnostics per `errors.md`. Parameter position is
   permanently rejected; stage 1 carves out only the return position.
6. **Pointer intrinsics**: `init(to:)` / `init(mutating:)` in
   `lang/std/memory/pointer.ks` backed by an address-capture intrinsic —
   follow the existing `lang.ptr_*` pattern (`pointer.ks` already uses
   `lang.ptr_read` / `lang.ptr_mut_borrow`).

## Explicitly untouched

Signature lowering/normalization, `kestrel-type-infer` (no `TyKind::Ref`),
`kestrel-mir` (no `MirTy::Ref`), both codegen backends, `verify.rs`. Add a
debug assertion that no `HirTy::Ref` survives HIR lowering anywhere.

## Incremental compilation

SyntaxKind is append-only (no re-parse of cached green trees). No accepted
code changes spelling, so no existing signature hash changes — strictly
additive.
