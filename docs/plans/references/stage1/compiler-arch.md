# Stage 1 — Compiler architecture

Base map: `references-prototype/references-plumbing.md` Stages 5-22, with
the corrections below (the checklist predates the syntax decisions and the
LLVM backend landing). Anchor re-verification as of 2026-06-09:
`references-gaps.md` §1.

## Corrections to the plumbing checklist

1. **`TyKind::Ref` flows out of calls only.** Params were normalized away in
   stage 0.5; Ref/MutRef appear in return positions
   (`LowerCallableReturnType`, `MemberResolution.return_type`) and as
   expression types of ref-returning calls. No `T → &T` argument coercion
   exists anywhere.
2. **Coercion arms** (absent from the checklist): `MutRef{T} → Ref{T}` and
   the `Ref{T} → T` copy-out (Q8(a)) in `solve_coerce` (solver.rs:1295);
   copy-out rides the existing CopyValue→clone mono-expand machinery.
3. **`FunctionRetConvention` single-source query**: `ret_borrow` is derived
   in exactly one place and consumed by MIR signature lowering,
   `MonoFunction` creation, member resolution, and (stage 3) witness
   lowering — never re-derived per site.
4. **LLVM twins** (the checklist has none): `kestrel-codegen-llvm`
   `ty.rs:173` classify · `abi.rs:40, 61-88` return_mode + build_signature ·
   `func.rs:63-80` resolve_scalar guard · `terminator.rs:147, 151` return.
   Twin table: `references-gaps.md` §6.

## The escape checker (build FIRST)

- `root_provenance` on `ValueDef` (or a parallel side table), stamped at
  creation, copied through `StructExtract` / `TupleExtract` / `EnumPayload`
  / `BeginBorrowAddr` — O(1) per projection, never walked at verify time.
- Return-site check in `verify.rs` (the Return arm is currently a no-op for
  `@guaranteed` — verify.rs:984-990): when `ret_borrow`, assert the root per
  the root rule, plus the mutable-root predicate for MutRef.
- The `.value` / `.mutatingValue` intrinsics stamp `PointerDerived`.

## The dangerous surgery (only after the checker)

The 6-site `ret_borrow` change (`references.md` §6), gated per-function:
the copy guards (`mir-lower/body/mod.rs:479-486`, `expr.rs:260-269`); the
`set_terminator` carve-out (mod.rs:1854-1882 — exactly one value);
`alloc_guaranteed` call-result registration; the `return_mode` /
`resolve_scalar` branches ×2 backends. **The copy-guard gate is the only
non-additive change in the whole stage** — land everything else additively
first.

## Transparent place (Q8 decided: (a) — `semantics.md`)

Four sites, in order:

1. **Fix `codegen_byref_scalar_deref` first** — the known @guaranteed-scalar
   double-deref becomes load-bearing once refs travel as expression types.
2. **The peel**: `solve_member` extracts the receiver `TyKind` at one point
   (solver.rs:~2554) before nominal lookup — peel `Ref`/`MutRef` there.
   Covers field/method/subscript/operators/for-in/compound-assign/
   interpolation at once (all funnel through `solve_member`,
   solver.rs:2535).
3. **Coercion arms**: correction #2 above.
4. **`classify_mutability`** (kestrel-analyze/src/body/access_mode.rs:249):
   consult the expression's inferred type before the syntactic walk —
   `MutRef` → Mutable (the #106 `is_mut_borrow_param` carve-out is the
   precedent), `Ref` → new shared-ref class (E-REF-20). Today a ref-typed
   call result falls to `_ => Temporary`. This classifier is currently
   purely syntactic; the type consult is the one new thread.

No new MIR lowering: a `&T` expr at MIR is a `@guaranteed` value, and field
access / receiver prep already route non-var bases through
`lower_expr_for_borrow` → `emit_struct_extract` and
`prepare_call_arg_for_expr` — exactly how borrowed params behave today.
