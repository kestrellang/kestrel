# Null Coalescing Operator Implementation Plan

## Test Strategy

- Basic unwrapping: `Some(x) ?? default` returns `x`
- None case: `None ?? default` returns `default`
- Short-circuit: RHS not evaluated when LHS is Some
- Chaining: `a ?? b ?? c` works correctly
- Mixed with `or`: precedence is correct
- Type errors: non-optional LHS, type mismatch
- Optional-to-optional: `Optional[T] ?? Optional[T] -> Optional[T]`

## Implementation Phases

### Phase 0: Tests (First!)

Files: `lib/kestrel-test-suite/tests/expressions/null_coalescing.rs`

- [ ] Basic unwrap with Some
- [ ] Basic unwrap with None
- [ ] Short-circuit evaluation test (side effect not triggered)
- [ ] Chaining `a ?? b ?? c`
- [ ] Mixed precedence `x ?? y or z`
- [ ] Optional ?? Optional -> Optional
- [ ] Type error: non-optional on LHS
- [ ] Register module in `lib/kestrel-test-suite/tests/expressions/mod.rs`

### Phase 1: Lexer

**Already complete** - `Token::QuestionQuestion` exists at `lib/kestrel-lexer/src/lib.rs`

### Phase 2: Syntax Tree

**Already complete** - `SyntaxKind::QuestionQuestion` exists at `lib/kestrel-syntax-tree/src/lib.rs`

### Phase 3: Parser

**Already complete** - `QuestionQuestion` is in `is_binary_operator_token()` at `lib/kestrel-semantic-tree-binder/src/body_resolver/operators.rs:227`

### Phase 4: Operator Registry

Files: `lib/kestrel-semantic-tree/src/operators.rs`

- [ ] Add new precedence constant `COALESCING = 15` (between DISJUNCTIVE and CONJUNCTIVE)
- [ ] Change `QuestionQuestion` entry to use `COALESCING` precedence
- [ ] Change `QuestionQuestion` entry to use `Right` associativity

### Phase 5: Builtins

Files: `lib/kestrel-semantic-tree/src/builtins.rs`

- [ ] Add `CoalesceOperatorProtocol` to `LanguageFeature` enum
- [ ] Add `CoalesceOperatorMethod` to `LanguageFeature` enum

### Phase 6: Operator Method Feature

Files: `lib/kestrel-semantic-tree/src/operators.rs`

- [ ] Update `BinaryOp::Coalesce.method_feature()` to return `Some(LanguageFeature::CoalesceOperatorMethod)` instead of `None`

### Phase 7: Body Resolver (Short-Circuit)

Files: `lib/kestrel-semantic-tree-binder/src/body_resolver/operators.rs`

- [ ] Add `BinaryOp::Coalesce` to the short-circuit match at line 347:
  ```rust
  if matches!(op, BinaryOp::And | BinaryOp::Or | BinaryOp::Coalesce) {
  ```

### Phase 8: Stdlib - Coalesce Protocol

Files: `lang/std/core/coalesce.ks` (new file)

- [ ] Create `Coalesce[Default]` protocol with `coalesce(default: () -> Default) -> Output`
- [ ] Add `@builtin(.CoalesceOperatorProtocol)` and `@builtin(.CoalesceOperatorMethod)` annotations

### Phase 9: Stdlib - Optional Extension

Files: `lang/std/result/optional.ks`

- [ ] Add `Coalesce[T]` conformance to `Optional[T]` (unwrap case)
- [ ] Add `Coalesce[Optional[T]]` conformance to `Optional[T]` (optional-to-optional case)
- [ ] Import `Coalesce` protocol

### Phase 10: Stdlib Module Export

Files: `lang/std/core/protocols.ks` or appropriate module file

- [ ] Export `Coalesce` protocol publicly

## Verification

After each phase:
```bash
cargo test
```

Final verification:
- [ ] All tests pass: `cargo test`
- [ ] Linting clean: `cargo clippy`
- [ ] Formatted: `cargo fmt`

## File Summary

| File | Action |
|------|--------|
| `lib/kestrel-test-suite/tests/expressions/null_coalescing.rs` | Create |
| `lib/kestrel-test-suite/tests/expressions/mod.rs` | Add module |
| `lib/kestrel-semantic-tree/src/operators.rs` | Update precedence/associativity |
| `lib/kestrel-semantic-tree/src/builtins.rs` | Add language features |
| `lib/kestrel-semantic-tree-binder/src/body_resolver/operators.rs` | Add to short-circuit |
| `lang/std/core/coalesce.ks` | Create protocol |
| `lang/std/result/optional.ks` | Add conformance |
