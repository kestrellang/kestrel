# kestrel-codegen-cranelift-3

MIR-3 → Cranelift IR → native code. This file documents known rules and
invariants. It is not exhaustive — when you discover a new rule, add it here.

## Debugging

- `kestrel dump mir <file> --std lang/std -f Foo.bar` dumps one function's MIR.
  Use `-f` to filter by substring match on the function name.
- `KESTREL_VERBOSE_CODEGEN=1` prints the name + error for every function that
  fails to compile (skipped with a trap stub).
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

## Switch on enum values

Always `emit_discriminant(enum_val)` → I32 discriminant, then
`emit_switch(disc, ...)`. Do NOT pass the raw enum value to `emit_switch`.
The switch codegen compares against variant indices using the discriminant
width from the mono enum; passing the full scalar enum value instead of an
extracted discriminant causes silent misrouting (the fallback width is I64,
which mismatches I8 discriminant scalars). Pattern matching in
`pattern.rs:~195` is the canonical example.
