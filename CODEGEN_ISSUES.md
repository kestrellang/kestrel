# Codegen Issues

Errors encountered when running:
```
cargo run -- build lang/std2/**/*.ks lang/io/**/*.ks examples/pong.ks
```

## Fixed Issues

### 1. Missing self_type in codegen substitution (FIXED)
- **File:** `lib/kestrel-codegen-cranelift/src/context.rs:224`
- **Fix:** Added `subst.set_self_type(inst.self_type)` after building substitution

### 2. Silent failure with unwrap_or (FIXED)
- **Files:** Multiple files in codegen
- **Fix:** Changed `apply_ty_readonly().unwrap_or()` to `.expect()` for better error visibility

### 3. Wrong self_type propagation in direct calls (FIXED)
- **Files:** `collect.rs` and `rvalue.rs`
- **Fix:** Only pass self_type if the callee actually uses Self in its signature

### 4. Integer abs comparison with wrong type (FIXED)
- **File:** `lib/kestrel-execution-graph-lowering/src/expr.rs:1941`
- **Fix:** Added `make_int_zero_for_mir_ty()` to create zero with correct bit width

## Remaining Issues

### 1. Function Not Found: Optional.unwrap with unresolved type param `I1TE`
```
error: function not found: std.result.Optional.unwrap (lookup: _K3std6result8Optional6unwrapP1R3std6result8OptionalI1TEI3std6memory10RawPointerE)
```
The type parameter `T` in `Optional[T]` is not being substituted. Needs investigation.

### 2. Float abs Verifier Error - Wrong comparison operand type
```
error: failed to define function '_K3std3num7Float643absP1R3std3num7Float64': Verifier errors:
- inst8 (v7 = fcmp.f64 lt v6, v1  ; v1 = 0): arg 1 (v1) has type i64, expected f64
```
The float comparison is using an i64 zero instead of f64const 0.0. The issue is in how the MIR Places are being resolved during codegen - the wrong value is being used for the float literal.

### 3. String function Verifier Errors
```
error: failed to define function '_K3std4text6String9trimStartP1R3std4text6String': Verifier errors:
error: failed to define function '_K3std4text6String7trimEndP1R3std4text6String': Verifier errors:
```
Similar type mismatch issues in string functions.

### 4. error immediate
```
error: code generation failed: unsupported: error immediate
```
An `ImmediateKind::Error` value is reaching codegen, indicating a lowering failure earlier in the pipeline.

### 5. Witness Not Found
```
error: monomorphization error: no witness found: protocol Id(197) for type Id(587)
```
A protocol witness lookup is failing for some type.

## Analysis

The main remaining issues seem to be:
1. Value tracking during codegen - wrong values being used for comparisons
2. Type substitution still failing in some cases (Optional.unwrap)
3. Error values propagating through the pipeline
