# Dictionary Literals Implementation Plan

## Test Strategy

### Test Categories
1. **Basic functionality**: Empty and non-empty dictionary literals
2. **Type inference**: Context-based inference, default type fallback
3. **Protocol conformance**: Custom types with `ExpressibleByDictionaryLiteral`
4. **Error cases**: Type mismatches, missing context, syntax errors
5. **Edge cases**: Trailing commas, nested literals, computed keys

### Key Behaviors to Verify
- `[:]` creates empty dictionary when type context exists
- `[k: v]` infers Key and Value types from elements
- All keys unify to same type, all values unify to same type
- Custom types conforming to protocol work with literals
- Default type used when no context available

## Implementation Phases

### Phase 0: Tests (First!)
**Files**: `lib/kestrel-test-suite/tests/dictionary_literals.rs`

- [ ] Basic empty dictionary with type annotation
- [ ] Basic non-empty dictionary
- [ ] Type inference from assignment context
- [ ] Type inference from function parameter
- [ ] Default type when no context
- [ ] Nested dictionary literals
- [ ] Trailing comma allowed
- [ ] Computed key expressions
- [ ] Error: empty dictionary without type context
- [ ] Error: key type mismatch
- [ ] Error: value type mismatch
- [ ] Error: type doesn't conform to protocol

### Phase 1: Lexer
**Files**: `lib/kestrel-lexer/src/lib.rs`

No changes needed - all required tokens exist:
- [x] `LBracket` `[`
- [x] `RBracket` `]`
- [x] `Colon` `:`
- [x] `Comma` `,`

### Phase 2: Syntax Tree
**Files**: `lib/kestrel-syntax-tree/src/lib.rs`

- [ ] Add `ExprDictionary` to `SyntaxKind` enum
- [ ] Add `DictionaryEntry` to `SyntaxKind` enum (wrapper for key-value pair)
- [ ] Update `kind_from_raw()` match statement

### Phase 3: Parser
**Files**: `lib/kestrel-parser/src/expr/mod.rs`

- [ ] Add `Dictionary` variant to `ExprVariant` enum
- [ ] Modify array parser to use look-ahead for disambiguation
- [ ] Create `emit_dictionary_expr()` function
- [ ] Update `emit_expr_variant()` to handle `ExprVariant::Dictionary`

Parser logic:
```
[           → start
  ]         → empty array
  :         →
    ]       → empty dictionary
  expr      →
    :       → dictionary (parse value, then pairs)
    , or ]  → array (continue with elements)
```

### Phase 4: Semantic Tree
**Files**: `lib/kestrel-semantic-tree/src/expr.rs`

- [ ] Add `Dictionary` variant to `ExprKind` enum
  ```rust
  /// Dictionary literal: `["key": value, ...]`
  Dictionary(Vec<(Expression, Expression)>),
  ```
- [ ] Add `Expression::dictionary()` constructor method

### Phase 5: Builtins
**Files**: `lib/kestrel-semantic-tree/src/builtins.rs`

- [ ] Add `_ExpressibleByDictionaryLiteral` to `LanguageFeature` enum
- [ ] Add `DefaultDictionaryLiteralType` to `LanguageFeature` enum
- [ ] Update `from_str()` for new variants
- [ ] Update `as_str()` for new variants
- [ ] Add `BuiltinDefinition` entries for new builtins

### Phase 6: Body Resolver
**Files**: `lib/kestrel-semantic-tree-binder/src/body_resolver/expressions.rs`

- [ ] Add match arm for `SyntaxKind::ExprDictionary`
- [ ] Create `resolve_dictionary_expression()` function
- [ ] Extract key-value pairs from `DictionaryEntry` nodes
- [ ] Resolve each key and value expression
- [ ] Return `Expression::dictionary(pairs, ty, span)`

### Phase 7: Type Inference
**Files**: `lib/kestrel-semantic-type-inference/src/constraint_generator.rs`

- [ ] Add match arm for `ExprKind::Dictionary`
- [ ] Add conformance constraint to `_ExpressibleByDictionaryLiteral`
- [ ] Create infer types for Key and Value
- [ ] Add normalizes constraints for Key and Value associated types
- [ ] Equate all keys to Key type, all values to Value type

```rust
ExprKind::Dictionary(pairs) => {
    // Conform to _ExpressibleByDictionaryLiteral
    if let Some(protocol_id) = ctx
        .oracle()
        .builtin_protocol(LanguageFeature::_ExpressibleByDictionaryLiteral)
    {
        let protocol_ref = ProtocolRef::new(protocol_id, expr.span.clone());
        ctx.conforms(expr.ty.id(), protocol_ref);
    }

    let key_ty = Ty::infer(expr.span.clone());
    let value_ty = Ty::infer(expr.span.clone());
    ctx.register_type(&key_ty);
    ctx.register_type(&value_ty);

    ctx.normalizes(expr.ty.id(), "Key".to_string(), key_ty.id(), expr.span.clone());
    ctx.normalizes(expr.ty.id(), "Value".to_string(), value_ty.id(), expr.span.clone());

    for (key, value) in pairs {
        generate_expression_constraints(ctx, key);
        generate_expression_constraints(ctx, value);
        ctx.equate(key.ty.id(), key_ty.id(), key.span.clone());
        ctx.equate(value.ty.id(), value_ty.id(), value.span.clone());
    }
}
```

### Phase 8: Code Generation
**Files**: `lib/kestrel-execution-graph-lowering/src/expr.rs`

- [ ] Add match arm for `ExprKind::Dictionary`
- [ ] Lower key-value pairs to tuple array
- [ ] Generate stack allocation for tuple buffer
- [ ] Generate call to `_ExpressibleByDictionaryLiteral.init`

### Phase 9: Standard Library
**Files**: `lang/std/core/literals.ks`

- [ ] Uncomment and update `ExpressibleByDictionaryLiteral` protocol
- [ ] Add `_ExpressibleByDictionaryLiteral` protocol
- [ ] Add `DefaultDictionaryLiteralType` type alias
- [ ] Add default implementation bridging the two protocols

**Files**: `lang/std/collections/dictionary.ks`

- [ ] Add `ExpressibleByDictionaryLiteral` conformance to Dictionary
- [ ] Add `_ExpressibleByDictionaryLiteral` conformance to Dictionary
- [ ] Implement `init(dictionaryLiteral:)` method

## Verification

After each phase:
```bash
cargo test -p kestrel-test-suite -- dictionary
```

Final verification:
```bash
cargo fmt
cargo clippy
cargo test
```

## File Change Summary

| File | Changes |
|------|---------|
| `lib/kestrel-syntax-tree/src/lib.rs` | Add `ExprDictionary`, `DictionaryEntry` |
| `lib/kestrel-parser/src/expr/mod.rs` | Parser + emitter for dictionary literals |
| `lib/kestrel-semantic-tree/src/expr.rs` | Add `ExprKind::Dictionary` |
| `lib/kestrel-semantic-tree/src/builtins.rs` | Add `_ExpressibleByDictionaryLiteral`, `DefaultDictionaryLiteralType` |
| `lib/kestrel-semantic-tree-binder/src/body_resolver/expressions.rs` | Resolve dictionary expressions |
| `lib/kestrel-semantic-type-inference/src/constraint_generator.rs` | Type inference for dictionaries |
| `lib/kestrel-execution-graph-lowering/src/expr.rs` | Code generation |
| `lang/std/core/literals.ks` | Dictionary literal protocols |
| `lang/std/collections/dictionary.ks` | Dictionary conformance |
| `lib/kestrel-test-suite/tests/dictionary_literals.rs` | Tests |
