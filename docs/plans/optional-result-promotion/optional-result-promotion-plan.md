# Optional and Result Type Promotion Implementation Plan

## Overview

Implement implicit type promotion for Optional and Result types via the `FromValue` protocol, providing symmetry with `FromResidual`. 

**Key Design Decision**: Desugar in the **binder** (like `throw`/`try`), not during lowering. When the target type is explicitly known to be Optional/Result, transform the expression to a method call: `5` → `Optional.from(5)`.

This approach:
- Follows existing patterns (`throw`, `try`, `for` loops all desugar in binder)
- Keeps type inference simple (sees normal method calls, no special promotion logic)
- Type checking validates `FromValue` conformance
- Lowering requires no changes (already handles method calls)

## Architecture

```
Binder → Type Inference → Type Check → Lowering
  ↓         ↓              ↓           ↓
Desugar   Standard       Validate    Normal
when      constraints    FromValue   method
target    for method     conformance call
known     call           lowering
```

**Desugaring Examples:**
```kestrel
// Variable declaration
let x: Int? = 5         → let x: Int? = Optional.from(5)

// Return statement
return 5 (in Int? fn)   → return Optional.from(5)

// Assignment
opt: Int? = 0
opt = 10                → opt = Optional.from(10)

// Function argument
call(opt: Int?, 5)      → call(opt: Int?, Optional.from(5))

// If branch
if cond { 5 } else { x: Int? }  → if cond { Optional.from(5) } else { x }
```

**No Desugaring (type inferred):**
```kestrel
let x = 5               // x is Int - no type annotation, no promotion
let y: Int = 5          // y is Int - target type not Optional/Result
```

## Test Strategy

### Test Categories

1. **Basic promotion tests** - Variable declarations, assignments, returns with explicit types
2. **Context tests** - Function arguments, if branches, match arms with known target types
3. **Generic tests** - Type parameter promotion in generic functions
4. **Edge case tests** - Nested optionals, type inference (no promotion), Never type
5. **Error tests** - Type mismatches, multi-level promotion attempts

### Key Behaviors to Verify

- `let x: Int? = 5` desugars to `Optional.from(5)` during binding
- `return 42` in `Int throws Error` function desugars to `Result.from(42)`
- Single-level promotion only (no `Int?? = 5`)
- **No promotion when target type is inferred** (`let x = 5` stays Int)
- Works in all contexts with explicit Optional/Result types

### Error Cases to Test

- Cannot promote to nested Optional (`let x: Int?? = 5`)
- Type mismatch in promotion context
- Generic type parameter mismatch

## Implementation Phases

### Phase 0: Tests (First!)

**Files**: `lib/kestrel-test-suite/tests/types/type_promotion.rs` (new file)

- [ ] **Basic Optional promotion**: `let x: Int? = 5` compiles
- [ ] **Basic Result promotion**: `let r: Int throws Error = 42` compiles
- [ ] **Return statement promotion**: `fn f() -> Int? { return 5 }` compiles
- [ ] **Assignment promotion**: `var opt: Int? = 0; opt = 10` compiles
- [ ] **Function argument promotion**: `fn call(x: Int?) {}; call(5)` compiles
- [ ] **If branch promotion**: `let x: Int? = if true { 5 } else { nil }` compiles
- [ ] **Generic function promotion**: `fn wrap[T](value: T) -> T? { return value }` compiles
- [ ] **No promotion without annotation**: `let x = 5` (x should be Int, not Int?)
- [ ] **Nested Optional rejection**: `let x: Int?? = 5` should fail

### Phase 1: Standard Library - FromValue Protocol

**Files**:
- `lang/std/core/error.ks` - Add `FromValue` protocol alongside `FromResidual`
- `lang/std/result/optional.ks` - Add `FromValue` conformance to Optional
- `lang/std/result/result.ks` - Add `FromValue` conformance to Result

- [ ] Define internal `FromValue[Output]` protocol:
  ```kestrel
  @builtin(.FromValueProtocol)
  protocol FromValue[Output] {
      @builtin(.FromValueMethod)
      static func from(_ value: Output) -> Self
  }
  ```
- [ ] Add `Optional[T]: FromValue[T]` conformance (returns `.Some(value)`)
- [ ] Add `Result[T, E]: FromValue[T]` conformance (returns `.Ok(value)`)
- [ ] Remove unused `Returnable` protocol from `error.ks`
- [ ] Verify standard library compiles

### Phase 2: Builtins Registry

**Files**: `lib/kestrel-semantic-tree/src/builtins.rs`

- [ ] Add `FromValueProtocol` to `LanguageFeature` enum
- [ ] Add `FromValueMethod` to `LanguageFeature` enum
- [ ] Add string name mappings (`"FromValueProtocol"`, `"FromValueMethod"`)
- [ ] Add `BuiltinDefinition` entries:
  - `FromValueProtocol` as protocol with associated type `Output`
  - `FromValueMethod` as method on `FromValueProtocol`

### Phase 3: Binder - Desugaring Logic

**Files**: `lib/kestrel-semantic-tree-binder/src/body_resolver/` (multiple files)

This is the core phase. Desugar promotion when target type is explicitly Optional/Result.

**3a. Variable Declaration Desugaring**

**File**: `lib/kestrel-semantic-tree-binder/src/body_resolver/statements.rs`

- [ ] In `resolve_variable_declaration()`, check if declared type is Optional/Result
- [ ] If so, wrap initializer expression with `from()` call
- [ ] Create deferred static call: `DeclaredType.from(initializer)`
- [ ] Use `LanguageFeature::FromValueMethod` for method resolution

```rust
// Pseudocode:
if target_type_conforms_to_from_value(declared_ty, value_ty) {
    let wrapped = create_from_value_call(declared_ty, value_expr, ctx);
    // Use wrapped as the binding value
}
```

**3b. Assignment Expression Desugaring**

**File**: `lib/kestrel-semantic-tree-binder/src/body_resolver/expressions.rs`

- [ ] In assignment resolution, check target type
- [ ] If target is Optional/Result and value is inner type, wrap value
- [ ] Similar pattern to variable declaration

**3c. Return Statement Desugaring**

**File**: `lib/kestrel-semantic-tree-binder/src/body_resolver/expressions.rs`

- [ ] In return expression resolution, check function return type
- [ ] If return type is Optional/Result, wrap return value
- [ ] Create `TargetType.from(value)` call

**3d. Function Argument Desugaring**

**File**: `lib/kestrel-semantic-tree-binder/src/body_resolver/calls.rs`

- [ ] When resolving call arguments, check each parameter type
- [ ] If parameter is Optional/Result and argument is inner type, wrap argument
- [ ] Handle labeled and unlabeled arguments

**3e. If Branch Desugaring**

**File**: `lib/kestrel-semantic-tree-binder/src/body_resolver/expressions.rs`

- [ ] After resolving if expression, check if result type is Optional/Result
- [ ] For branches that return inner type, wrap with `from()` call
- [ ] Ensure both branches have same type after desugaring

**Helper Functions to Create:**

Create in a new file or existing utility module:

```rust
/// Check if promotion should occur
fn should_promote(target_ty: &Ty, value_ty: &Ty, ctx: &BodyResolutionContext) -> bool {
    // Check if target conforms to FromValue[value_ty]
}

/// Create the desugared expression: TargetType.from(value)
fn create_from_value_call(
    target_ty: &Ty, 
    value_expr: Expression, 
    ctx: &mut BodyResolutionContext
) -> Expression {
    // Create deferred static call to from() method
    // Similar to how throw creates fromResidual call
}
```

### Phase 4: Type Inference - Constraint Generation

**Files**: `lib/kestrel-semantic-type-inference/src/constraint_generator.rs`

With binder desugaring, type inference sees normal method calls. Verify constraints work:

- [ ] For `TargetType.from(value)` calls:
  - Generate standard call constraints
  - Method returns `TargetType` (Self)
  - Argument type must match `Output` associated type
- [ ] Ensure `FromValue` conformance is checked during method resolution
- [ ] Test that inference resolves correctly for:
  - Concrete types (`Int` → `Optional[Int]`)
  - Generic types (`T` → `Optional[T]` in generic function)

**Note**: Type inference should need minimal or no changes. The deferred static call infrastructure already handles method resolution including protocol conformance.

### Phase 5: Type Checking - Validation

**Files**: `lib/kestrel-semantic-analyzers/src/analyzers/type_check/mod.rs`

- [ ] Ensure type checking validates the desugared code correctly
- [ ] For promoted assignments, verify:
  - Method call resolves correctly
  - Return types match
  - No additional errors introduced by desugaring
- [ ] Add specific error message if `FromValue` conformance fails

**Files**: `lib/kestrel-semantic-analyzers/src/analyzers/type_assignability/mod.rs`

- [ ] May need minor updates to handle the desugared pattern
- [ ] Ensure assignability check works with the `from()` call return type

### Phase 6: Integration & Verification

- [ ] Run type promotion tests: `cargo test type_promotion`
- [ ] Run full test suite: `cargo test`
- [ ] Verify no regressions in existing tests
- [ ] Check linting: `cargo clippy`
- [ ] Check formatting: `cargo fmt`
- [ ] Test with real programs:
  ```kestrel
  fn main() {
      let x: Int? = 5
      let r: Int throws Error = 42
      print(x)
  }
  ```

## Files Modified Summary

| File | Changes |
|------|---------|
| `lang/std/core/error.ks` | Add internal `FromValue` protocol |
| `lang/std/result/optional.ks` | Add `FromValue[T]` conformance to Optional |
| `lang/std/result/result.ks` | Add `FromValue[T]` conformance to Result |
| `lib/kestrel-semantic-tree/src/builtins.rs` | Add `FromValueProtocol`, `FromValueMethod` |
| `lib/kestrel-semantic-tree-binder/src/body_resolver/statements.rs` | Variable declaration desugaring |
| `lib/kestrel-semantic-tree-binder/src/body_resolver/expressions.rs` | Assignment, return, if branch desugaring |
| `lib/kestrel-semantic-tree-binder/src/body_resolver/calls.rs` | Function argument desugaring |
| `lib/kestrel-semantic-analyzers/src/analyzers/type_check/mod.rs` | Validation (minor updates) |
| `lib/kestrel-test-suite/tests/types/type_promotion.rs` | Add tests (new file) |

**Note**: Lowering requires no changes! It already handles method calls.

## Implementation Order

1. **Phase 0**: Write failing tests that define expected behavior
2. **Phase 1**: Add `FromValue` protocol to standard library
3. **Phase 2**: Register builtins
4. **Phase 3**: **Binder desugaring** (the main work)
   - Start with variable declarations (simplest)
   - Then return statements
   - Then assignments
   - Then function arguments
   - Finally if branches
5. **Phase 4**: Verify type inference works with desugared code
6. **Phase 5**: Type checking validation
7. **Phase 6**: Integration testing and fixes

## Common Pitfalls to Avoid

1. **Only desugar when target type is explicit**
   - `let x = 5` → NO desugaring (x inferred as Int)
   - `let x: Int? = 5` → Desugar (explicit Optional type)

2. **Check type expansion first**
   - `Int?` expands to `Optional[Int]` - check conformance on expanded type

3. **Single-level promotion**
   - `Int` → `Optional[Int]` ✓
   - `Int` → `Optional[Optional[Int]]` ✗ (doesn't conform to `FromValue[Int]`)

4. **Context tracking**
   - Need symbol ID for conformance checking in correct scope
   - Use `ctx.model.conforms_to()` with proper context

5. **Deferred call creation**
   - Follow pattern from `throw` expression (lines 1756-1763)
   - Use `Expression::deferred_static_call()` with `LanguageFeature::FromValueMethod`

## Type Inference Interaction Summary

| Aspect | With Binder Desugaring |
|--------|----------------------|
| **What type inference sees** | Normal method call: `Optional.from(5)` |
| **Constraints generated** | Standard call constraints (method exists, arg matches, return is Self) |
| **Conformance checking** | Via method resolution (deferred static call with protocol candidates) |
| **Special logic needed** | None - uses existing infrastructure |
| **Inference for generics** | Works naturally via type parameter substitution |

**Comparison with FromResidual:**

| Feature | FromResidual (throw) | FromValue (promotion) |
|---------|---------------------|----------------------|
| **Trigger** | `throw error` | Explicit Optional/Result target type |
| **Desugar location** | Binder | Binder |
| **Becomes** | `return R.fromResidual(error)` | `TargetType.from(value)` |
| **Type inference** | Sees return with call | Sees assignment/return with call |
| **Lowering** | Normal return lowering | Normal method call lowering |

This symmetry makes the implementation clean and maintainable.
