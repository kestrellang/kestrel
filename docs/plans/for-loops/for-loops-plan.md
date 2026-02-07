# For Loops Implementation Plan

## Test Strategy

- Basic for loop iteration
- Pattern destructuring (tuples, structs)
- Mutable bindings (`for var x in ...`)
- Labeled for loops with break/continue
- Nested for loops
- Empty iterator behavior
- Type inference through iteration
- Error cases: non-iterable types, refutable patterns

## Implementation Phases

### Phase 0: Tests (First!)

Files: `lib/kestrel-test-suite/tests/for_loops.rs`

- [ ] Basic for loop over range
- [ ] For loop with tuple destructuring
- [ ] For loop with mutable binding
- [ ] Labeled for loop with break
- [ ] Labeled for loop with continue
- [ ] Nested for loops
- [ ] For loop over iterator directly
- [ ] Empty iterator (body never executes)
- [ ] Error: non-iterable type
- [ ] Error: refutable pattern

### Phase 1: Builtins

Files: `lib/kestrel-semantic-tree/src/builtins.rs`

- [ ] Add `IteratorProtocol` to `LanguageFeature`
- [ ] Add `IteratorNextMethod` to `LanguageFeature`
- [ ] Add `IterableProtocol` to `LanguageFeature`
- [ ] Add `IterableIterMethod` to `LanguageFeature`
- [ ] Add `OptionalEnum` to `LanguageFeature`
- [ ] Add `OptionalSomeCase` to `LanguageFeature`
- [ ] Add `OptionalNoneCase` to `LanguageFeature`
- [ ] Add `BuiltinDefinition` for each
- [ ] Add `BuiltinKind::EnumCase` if not exists
- [ ] Update `BuiltinRegistry` for enum case registration/lookup

### Phase 2: Standard Library

Files: `lang/std/iter/iterator.ks`, `lang/std/result/optional.ks`

- [ ] Add `@builtin(.IteratorProtocol)` to `Iterator`
- [ ] Add `@builtin(.IteratorNextMethod)` to `next()`
- [ ] Add `@builtin(.IterableProtocol)` to `Iterable`
- [ ] Add `@builtin(.IterableIterMethod)` to `iter()`
- [ ] Add `extend Iterator: Iterable` conformance
- [ ] Add `@builtin(.OptionalEnum)` to `Optional`
- [ ] Add `@builtin(.OptionalSomeCase)` to `Some`
- [ ] Add `@builtin(.OptionalNoneCase)` to `None`

### Phase 3: Lexer

Files: `lib/kestrel-lexer/src/lib.rs`

- [ ] Verify `Token::For` exists (add if missing)
- [ ] Verify `Token::In` exists (already confirmed)

### Phase 4: Syntax Tree

Files: `lib/kestrel-syntax-tree/src/lib.rs`

- [ ] Add `ExprFor` to `SyntaxKind`
- [ ] Add `ForPattern` to `SyntaxKind` (wrapper for the pattern)
- [ ] Add `ForIterable` to `SyntaxKind` (wrapper for the expression)
- [ ] Update `kind_from_raw()` for new kinds

### Phase 5: Parser

Files: `lib/kestrel-parser/src/expr/mod.rs`

- [ ] Add `For` variant to `ExprVariant` enum
- [ ] Create `for_expression()` parser function
- [ ] Parse optional label
- [ ] Parse `for` keyword
- [ ] Parse pattern
- [ ] Parse `in` keyword
- [ ] Parse iterable expression
- [ ] Parse block body
- [ ] Emit `ExprFor` node with children
- [ ] Integrate into expression parser (no `;` required)

### Phase 6: Binder - Desugaring

Files: `lib/kestrel-semantic-tree-binder/src/body_resolver/expressions.rs`

- [ ] Add `resolve_for_expression()` function
- [ ] Look up `Iterable.iter()` method via builtin registry
- [ ] Look up `Iterator.next()` method via builtin registry
- [ ] Look up `Optional.Some` case via builtin registry
- [ ] Create synthetic `var iter = expr.iter()` statement
- [ ] Create `.Some(pattern)` enum pattern
- [ ] Create `iter.next()` method call expression
- [ ] Create `while let` condition with the pattern and call
- [ ] Resolve body statements in new scope
- [ ] Construct final `WhileLet` expression with label
- [ ] Handle span preservation for error messages
- [ ] Register in expression resolver dispatch

## Verification

- [ ] All tests pass: `cargo test`
- [ ] Linting clean: `cargo clippy`
- [ ] Formatted: `cargo fmt`

## File Summary

| Phase | Files |
|-------|-------|
| Tests | `lib/kestrel-test-suite/tests/for_loops.rs` |
| Builtins | `lib/kestrel-semantic-tree/src/builtins.rs` |
| Std | `lang/std/iter/iterator.ks`, `lang/std/result/optional.ks` |
| Lexer | `lib/kestrel-lexer/src/lib.rs` |
| Syntax | `lib/kestrel-syntax-tree/src/lib.rs` |
| Parser | `lib/kestrel-parser/src/expr/mod.rs` |
| Binder | `lib/kestrel-semantic-tree-binder/src/body_resolver/expressions.rs` |
