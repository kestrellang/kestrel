# kestrel-codegen-cranelift

MIR → Cranelift IR → native code. This file documents known rules and
invariants. It is not exhaustive — when you discover a new rule, add it here.

## Debugging

- `kestrel dump mir <file> --std lang/std -f Foo.bar` dumps one function's MIR.
  Use `-f` to filter by substring match on the function name.
- A function that fails codegen/verification **aborts the whole build**:
  `define_all_functions` collects every failure, then returns
  `CodegenError::CompilationFailed`, so no object is emitted and no binary is
  linked (exit non-zero). The error lists every failing function by name — there
  is no warn-and-continue / trap-stub-and-ship path anymore (it produced clean
  builds that SIGILL'd at runtime; #151/#149). Trap stubs are still emitted
  in-loop but never serialized (`finish()` is skipped on the error path).
- There is no per-function CLIF dump yet. To inspect Cranelift IR, add a
  temporary `println!("{}", builder.func.display())` in `func.rs`.

## @guaranteed representation (Option B)

- `@guaranteed` values are always **pointers** in Cranelift, even for Scalar
  types. An `@guaranteed Int64` is `ptr_ty` pointing to the i64 on the stack.
- `@owned` Scalar values are the scalar itself (e.g. `i64`).
- `resolve_scalar(builder, id)` bridges the gap: for `@guaranteed` Scalar
  types it **loads** from the pointer; for `@owned` it returns the value as-is.

## resolve_scalar vs get_value

- `get_value` returns the raw Cranelift value — a pointer for @guaranteed, a
  scalar for @owned. No transformation.
- `resolve_scalar` loads through @guaranteed indirection for Scalar repr types.
  Use it when you need the **value**, not the address.
- **Exception — `Op::PtrTo`**: needs the address, not the value. The Op1
  handler checks for PtrTo and uses `get_value` on @guaranteed args instead of
  `resolve_scalar`, since the @guaranteed address IS the pointer we want.

## Type layout: `tc.repr` is the single authority

- `TypeCache::repr` / `classify` (`ty.rs`) is the **sole** place that decides a
  type's Cranelift representation. `build_signature`, `compile_struct`,
  `compile_struct_extract`, `compile_enum`, `resolve_scalar` — all derive their
  cl types from it. Never re-derive a layout decision locally; if you need a
  type's repr, call `repr`.
- **Single-field newtypes delegate to their field.** A one-field struct's value
  *is* its field's value (`compile_struct`'s single-field path returns the field
  directly; `compile_struct_extract` returns the base unchanged). So
  `classify_named` must report the field's own repr, NOT an integer collapsed by
  byte size. Collapsing 8→I64 / 4→I32 silently mis-typed float newtypes
  (`Float64`/`Float32`): the auto clone-shim's signature said `-> i64` while its
  body returned an `f64`, failing Cranelift verification → trap stub. The fix
  recurses `self.repr(field_ty)` and uses that scalar; integer-by-size is only a
  fallback for pure-discriminant enums and structs over a non-scalar field.
- Invariant pinned by `debug_assert_eq!(base_cl, field_cl)` in
  `compile_struct_extract`: a single-field newtype's repr must equal its field's
  repr. If it fires, some layout decision has diverged from `classify_named`
  again — fix the divergence, don't add a coercion (bitcast/load) to bridge it.

## Switch on enum values

Always `emit_discriminant(enum_val)` → I32 discriminant, then
`emit_switch(disc, ...)`. Do NOT pass the raw enum value to `emit_switch`.
The switch codegen compares against variant indices using the discriminant
width from the mono enum; passing the full scalar enum value instead of an
extracted discriminant causes silent misrouting (the fallback width is I64,
which mismatches I8 discriminant scalars). Pattern matching in
`pattern.rs:~195` is the canonical example.

## Two ref-value representations (stage 2b/extend-&T contract)

A `&T` is a pointer scalar with TWO codegen representations, keyed by the
MIR value's TYPE:

- **Ref-TYPED value** (`MirTy::Ref`): @owned → the value IS the ref scalar
  (pointer to pointee); @guaranteed → the value is the address OF the ref
  slot (one extra level). Generic bodies at `T = &U` and `extend &T` method
  params produce these.
- **Pointee-typed VIEW** (the expression seam's peeled form — `let r = &x`
  bindings, receiver Views): @guaranteed value whose codegen value IS the
  ref scalar.

The two translation seams: the fused `BeginBorrow` peel (ref-typed operand,
pointee-typed result → @guaranteed loads the stored address, @owned passes
through) and the call-arg carve (`param_is_ref` × `arg_is_ref_typed`: a
ref-typed @guaranteed arg is ALREADY the address ByRef wants — pass as-is;
a pointee-typed view must SPILL to materialize the ref slot). A missed
translation does not crash — it reads pointer bits as the pointee (values
compare as addresses). Both backends must mirror any change here.
