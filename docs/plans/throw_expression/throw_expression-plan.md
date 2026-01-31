# Throw Expression Implementation Plan

## Test Strategy

- Test categories to write:
  - Basic throw expressions
  - Throw with different error types
  - Throw in generic functions
  - Throw in nested functions/closures
  - Error cases (missing expression, incompatible types)
  - Edge cases (dead code detection, Never type propagation)

- Key behaviors to verify:
  - throw expr desugars to return R.fromResidual(expr)
  - Type is Never (diverging expression)
  - Works with try operator patterns

- Error cases to test:
  - Missing expression after throw
  - Throw outside function
  - Return type doesn't implement FromResidual

## Implementation Phases

### Phase 0: Tests (First!)

Files: lib/kestrel-test-suite/tests/expressions/throw.rs

- [ ] Basic throw expression compiles
- [ ] Throw with custom error type
- [ ] Throw in function returning Result type
- [ ] Throw in generic function with FromResidual bound
- [ ] Error: missing expression after throw
- [ ] Error: throw outside function (at module level)
- [ ] Never type propagation with throw

### Phase 1: Lexer (new token)

Files: lib/kestrel-lexer/src/lib.rs

- [ ] Add `Throw` keyword token

### Phase 2: Syntax Tree

Files: lib/kestrel-syntax-tree/src/lib.rs

- [ ] Add `ExprThrow` to SyntaxKind enum
- [ ] Update kind_from_raw() if needed

### Phase 3: Parser

Files: lib/kestrel-parser/src/expr/mod.rs

- [ ] Add `Throw` variant to ExprVariant
- [ ] Parse `throw <expression>` syntax
- [ ] Add throw to expression parsing (likely in expression_ending_with_brace_or_block or similar)

### Phase 4: Semantic Symbol (no new symbol needed)

No new symbol required - throw is an expression, not a declaration.

### Phase 5: Builder (BUILD)

Files: lib/kestrel-semantic-tree/src/expr.rs

- [ ] Add `Throw` variant to ExprKind enum
- [ ] Add `throw_expr(value, span)` helper method to Expression

### Phase 6: Binder (BIND)

Files: lib/kestrel-semantic-tree-binder/src/body_resolver/expressions.rs

- [ ] Add `resolve_throw_expression()` function
- [ ] Desugar throw to: return R.fromResidual(expr)
- [ ] Register in expression resolution match statement

### Phase 7: Type Inference

Files: lib/kestrel-semantic-type-inference/src/constraint_generator.rs

- [ ] Handle ExprKind::Throw (type is Never, no constraints needed)

### Phase 8: Lowering

Files: lib/kestrel-execution-graph-lowering/src/expr.rs

- [ ] No changes needed - throw desugars to return in binder phase

### Phase 9: Validation

Files: lib/kestrel-semantic-analyzers/src/analyzers/

- [ ] Update exhaustiveness checker to handle throw (type is Never)
- [ ] Update dead code detection if needed

## Verification

- [ ] All tests pass: `cargo test`
- [ ] Linting clean: `cargo clippy`
- [ ] Formatted: `cargo fmt`

## Reference Patterns

Follow these existing implementations:
- Return expression: lib/kestrel-semantic-tree-binder/src/body_resolver/expressions.rs:1567-1589
- Try operator (desugaring pattern): lib/kestrel-semantic-tree-binder/src/body_resolver/expressions.rs:1591-1748
