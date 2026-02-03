# Expression-Bodied Functions Implementation Plan

## Test Strategy

- Basic expression-bodied functions (top-level, methods, static methods)
- Generics and where clauses with expression bodies
- Protocol default implementations with expression bodies
- Error cases (missing return type, extern with body)
- Multi-line expressions
- All parameter features (default values, access modes, patterns)

## Implementation Phases

### Phase 0: Tests (First!)
Files: `lib/kestrel-test-suite/tests/declarations/expression_bodied_functions.rs`
- [ ] Basic expression-bodied function
- [ ] Expression-bodied function with parameters
- [ ] Expression-bodied method (instance)
- [ ] Expression-bodied static method
- [ ] Expression-bodied function with generics
- [ ] Expression-bodied function with where clause
- [ ] Protocol default implementation with expression body
- [ ] Multi-line expression body
- [ ] Expression body with closure return
- [ ] Error: expression body on extern function

### Phase 1: Parser Data Structures
Files: `lib/kestrel-parser/src/common/data.rs`
- [ ] Create `FunctionBodyData` enum with `Block(CodeBlockData)` and `Expression(ExprVariant)` variants
- [ ] Update `FunctionDeclarationData.body` from `Option<CodeBlockData>` to `Option<FunctionBodyData>`
- [ ] Update `InitializerDeclarationData.body` similarly (initializers can also use expression bodies)

### Phase 2: Parser
Files: `lib/kestrel-parser/src/common/parsers.rs`
- [ ] Update `function_body_parser()` to parse `= expression` OR `{ code_block }` OR nothing
- [ ] Return `Option<FunctionBodyData>` instead of `Option<CodeBlockData>`

### Phase 3: Emitter
Files: `lib/kestrel-parser/src/common/emitters.rs`
- [ ] Update `emit_function_body()` to handle both `FunctionBodyData::Block` and `FunctionBodyData::Expression`
- [ ] For expression bodies: emit `FunctionBody` node containing `Equals` token and `Expression` node

### Phase 4: Syntax Tree (if needed)
Files: `lib/kestrel-syntax-tree/src/lib.rs`
- [ ] Verify `SyntaxKind::Equals` exists (it does - used for variable init)
- [ ] No new syntax kinds needed - `FunctionBody` already supports containing `Expression`

### Phase 5: Semantic (Already Done!)
The semantic layer is already prepared:
- `resolve_function_body()` in `context.rs:214-237` handles `Expression` children
- `FunctionBinder` validation at line 301 anticipates expression bodies
- No changes needed

## Files to Modify

| File | Change |
|------|--------|
| `lib/kestrel-parser/src/common/data.rs` | Add `FunctionBodyData` enum, update `FunctionDeclarationData` |
| `lib/kestrel-parser/src/common/parsers.rs` | Update `function_body_parser()` |
| `lib/kestrel-parser/src/common/emitters.rs` | Update `emit_function_body()` |
| `lib/kestrel-test-suite/tests/declarations/expression_bodied_functions.rs` | New test file |
| `lib/kestrel-test-suite/tests/declarations/mod.rs` | Add `mod expression_bodied_functions;` |

## Verification
- [ ] All tests pass: `cargo test`
- [ ] Linting clean: `cargo clippy`
- [ ] Formatted: `cargo fmt`

## Implementation Notes

1. The `=` token (`Token::Equals`) is already defined in the lexer
2. The semantic layer (`resolve_function_body` in `body_resolver/context.rs`) already handles expression bodies - it looks for `SyntaxKind::Expression` children in `FunctionBody`
3. The binder validation for extern functions already checks for expression bodies (line 301 in `function.rs`)
4. Expression parsing already exists via `expression_parser()`

## Risk Assessment

**Low risk** - The semantic infrastructure is already in place. Changes are isolated to:
1. Parser data structures (adding an enum variant)
2. Parser logic (trying `= expr` before `{ block }`)
3. Emitter logic (emitting the right syntax nodes)

No changes to symbol creation, binding, or code generation are expected.
