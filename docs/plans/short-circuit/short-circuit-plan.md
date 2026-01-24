# Short-Circuit Evaluation Implementation Plan

## Test Strategy

- Verify short-circuit behavior (RHS not evaluated when not needed)
- Test chained operators (`a and b and c`)
- Test mixed operators (`a or b and c`)
- Test side effects in RHS are skipped appropriately
- Test error cases (type mismatches)

## Implementation Phases

### Phase 0: Tests

**Files:** `lib/kestrel-test-suite/tests/expressions/short_circuit.rs`

- [ ] Basic `and` short-circuit: `false and sideEffect()` doesn't call RHS
- [ ] Basic `or` short-circuit: `true or sideEffect()` doesn't call RHS
- [ ] `and` evaluates RHS when LHS is true
- [ ] `or` evaluates RHS when LHS is false
- [ ] Chained `and`: `false and x and y` skips both
- [ ] Chained `or`: `true or x or y` skips both
- [ ] Mixed operators with precedence
- [ ] Nested expressions: `(a and b) or (c and d)`

### Phase 1: Update Protocol Definitions

**Files:** `lang/std/core/logical.ks`

- [ ] Change `And` protocol: `func logicalAnd(other: Rhs)` → `func logicalAnd(other: () -> Rhs)`
- [ ] Change `Or` protocol: `func logicalOr(other: Rhs)` → `func logicalOr(other: () -> Rhs)`

### Phase 2: Update Bool Implementation

**Files:** `lang/std/core/bool.ks`

- [ ] Update `logicalAnd` to take closure, call conditionally
- [ ] Update `logicalOr` to take closure, call conditionally

### Phase 3: Update Operator Desugaring

**Files:** `lib/kestrel-semantic-tree-binder/src/body_resolver/operators.rs`

- [ ] Detect `BinaryOp::And` and `BinaryOp::Or`
- [ ] Wrap RHS in a closure expression before creating method call
- [ ] Ensure closure has correct type `() -> T`

### Phase 4: Update Primitive Method Registry (if needed)

**Files:** `lib/kestrel-semantic-tree/src/expr.rs`

- [ ] Check if `PrimitiveMethod::BoolAnd` / `BoolOr` need signature updates
- [ ] Update if they're used for type checking

### Phase 5: Fix Any Broken Tests

- [ ] Run full test suite
- [ ] Update any tests that directly call `.logicalAnd(value)` to use closure syntax

## Verification

- [ ] All tests pass: `cargo test`
- [ ] Linting clean: `cargo clippy`
- [ ] Formatted: `cargo fmt`
