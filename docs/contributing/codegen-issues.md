# Kestrel Codegen Issues

This document tracks codegen issues discovered through systematic testing (50 test runs of the standard library build).

## Source of Non-Determinism ✅ FIXED

**Location**: `lib/kestrel-codegen-cranelift/src/monomorphize/instantiation.rs:89-99`

Previously, `MonomorphizationSet` used `HashSet` which has non-deterministic iteration order.

**Fix Applied**: Replaced `HashSet` with `IndexSet` (from `indexmap` crate):

```rust
pub struct MonomorphizationSet {
    pub functions: IndexSet<FunctionInstantiation>,  // Deterministic iteration order
    pub structs: IndexSet<StructInstantiation>,      // Deterministic iteration order
    pub enums: IndexSet<EnumInstantiation>,          // Deterministic iteration order
}
```

Now builds produce consistent, reproducible errors across runs.

---

## Current Issues (as of 2025-01-18)

### Issue #1: Error Immediate (22% of failures)

- **Error**: `unsupported: error immediate`
- **Location**: `lib/kestrel-codegen-cranelift/src/rvalue.rs:756` and `rvalue.rs:1462`
- **Root Cause**: `ExprKind::Error` expressions reach codegen as `ImmediateKind::Error` poison values. At `expr.rs:1368-1370`:
  ```rust
  ExprKind::Error => {
      // Error expression - return error value (error already reported)
      Value::Immediate(Immediate::error())
  }
  ```
  The comment says "error already reported" but if the earlier error wasn't properly flagged (e.g., was a warning), the poison value reaches codegen.
- **Occurrences**: 11/50
- **Status**: Needs investigation into why `ExprKind::Error` nodes exist without corresponding error diagnostics

### Issue #2: Cranelift Frontend Panic (20% of failures)

- **Error**: `panicked at cranelift-frontend/src/frontend.rs:519`
- **Root Cause**: Variables referenced before being defined in Cranelift's SSA builder. This occurs when:
  1. A function body has an `Immediate::error()` value in a critical path
  2. Type mismatches cause bad codegen
  3. Control flow is not properly established due to earlier errors
- **Occurrences**: 10/50
- **Status**: Secondary issue - fixing Issues #1 and #3 should reduce these panics

### Issue #3: Witness Call Requires Self Type ✅ FIXED

- **Error**: `unsupported: witness call requires Self type for: std.core.Comparable`
- **Location**: `lib/kestrel-codegen-cranelift/src/monomorphize/collect.rs:80-109`
- **Root Cause**: Protocol extension methods (like `Comparable.lessThan`) have `Self` in their parameter types. During seeding, these functions were added to the result set with `self_type: None`, then skipped during processing. But codegen still tried to compile them with the missing self_type.
- **Fix Applied**: Modified `seed_non_generic_functions()` to filter out functions that need Self type. These functions are now only added to the result set when called through witness resolution, which provides the concrete self_type.

```rust
// In seed_non_generic_functions:
let needs_self_type = func_def.params.iter().any(|&param_id| {
    let param = &self.mir.params[param_id];
    self.type_needs_self(self.mir.ty(param.ty))
}) || self.type_needs_self(self.mir.ty(func_def.ret));

if needs_self_type {
    continue;  // Skip - will be processed when called with concrete types
}
```

### Issue #4: Verifier Type Mismatches ✅ MOSTLY FIXED

- **Error**: `Verifier errors: arg 1 (vX) has type i64, expected i32`
- **Location**: `lib/kestrel-execution-graph-lowering/src/expr.rs`
- **Root Cause**: Numeric literals in intrinsic calls were being inferred as wrapper struct types (like `Int64`, `Float64`) instead of primitive types (like `lang.i32`, `lang.f32`).
- **Occurrences**: Originally 20/50 (40%), now fixed for most cases
- **Status**: Fixed for integer and float literals

**Fixes Applied**:
1. Added `lower_expression_with_expected_type()` helper that checks if the expected type is a primitive int/float and the expression is a literal.
2. For `TyKind::Int(bits)` - use `make_int_immediate()` with the expected bits.
3. For `TyKind::Float(bits)` - use `make_float_immediate()` with the expected bits (fixes `Float32.abs`, `Float64.abs`).
4. Updated intrinsic handlers (`IntBinary`, `IntBinarySigned`, `IntBinaryUnsigned`, `FloatBinary`) to use this helper.

### Issue #9: Return Type Mismatch in Complex Control Flow ✅ FIXED

- **Error**: `Verifier errors: return ... has type i8, must match function signature of i64`
- **Location**: Cranelift SSA construction
- **Function**: `std.text.decodeUtf8$at` (was the only failing function)
- **Root Cause**: When an if-expression with type `Never` (all branches diverge) is lowered, a result temp local was created but never assigned in the branches that return early. Cranelift's SSA builder then incorrectly aliased the undefined variable to a boolean condition value.

**Fix Applied**: In `lower_if()` (expr.rs:3606-3833), skip creating `if_result` and `join_block` when the if-expression has type `Never`:

```rust
// Check if all branches diverge (result type is Never)
let all_branches_diverge = matches!(expr.ty.kind(), TyKind::Never);

// Only create result local and join block if branches converge
let (result_place, join_block) = if all_branches_diverge {
    (None, None)
} else {
    // ... create if_result and join_block as normal
    (Some(result_place), Some(join_block))
};
```

All uses of `result_place` and `join_block` are now guarded with `if let Some(...)`. When all branches diverge:
- No result local is created (avoiding the SSA aliasing bug)
- No join block is created (since control never reaches it)
- Return `Value::Immediate(Immediate::unit())` as placeholder

---

## Previously Fixed Issues

### Issue #5: Unsupported Place Kind for Type Lookup ✅ FIXED

- **Error**: `unsupported: unsupported place kind for type lookup`
- **File**: `lib/kestrel-codegen-cranelift/src/terminator.rs:198-263`
- **Root Cause**: `get_place_type()` missing handler for `PlaceKind::Index`
- **Fix**: Added `PlaceKind::Index` handler at `terminator.rs:259-344`

### Issue #6: Unresolved Type Argument for Direct Call ✅ FIXED

- **Error**: `unresolved type argument for direct call ... : Error`
- **File**: `lib/kestrel-execution-graph-lowering/src/expr.rs:2285-2293`
- **Root Cause**: `TyKind::Error` types in type arguments passed to codegen
- **Fix**: Added early detection to skip emitting calls with error type arguments

### Issue #7: Function Not Found ✅ FIXED

- **Error**: `function not found: std.result.Optional.unwrap`
- **File**: `lib/kestrel-codegen-cranelift/src/monomorphize/collect.rs:174-186`
- **Root Cause**: `StatementKind::Call` not inferring Self type from call arguments
- **Fix**: Added self-type inference logic matching `Rvalue::Call` pattern

### Issue #8: Diagnostic Severity ✅ FIXED

- **Error**: Various `Unsupported*` errors not stopping compilation
- **File**: `lib/kestrel-execution-graph-lowering/src/error.rs:143-190`
- **Root Cause**: `LoweringError` variants using `Diagnostic::warning()` instead of `Diagnostic::error()`
- **Fix**: Changed all `Unsupported*` diagnostics to use `Diagnostic::error()`
