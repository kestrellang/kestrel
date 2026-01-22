# Common Implementation Issues

This document captures common pitfalls encountered when implementing new language features.

## Statement-Like Expressions

Some expressions (like `if`, `while`, `loop`, `for`, `match`) don't require a semicolon when used as statements in a block. To implement this correctly:

1. Add your expression variant to `is_statement_like_expr()` in `kestrel-parser/src/block/mod.rs`
2. The block parser will then allow these expressions to appear without a trailing semicolon

If you forget this, users will get confusing parse errors when they write code like:
```
for i in range {
    sum = sum + i
}
count  // "expected semicolon" error on the for loop
```

## Semantic Errors Belong in Analyzers

Semantic validation (type checking, pattern validation, etc.) should be implemented as analyzer passes in `kestrel-semantic-analyzers`, not inline in the body resolver (`kestrel-semantic-tree-binder`).

- **Body resolver**: Transforms syntax trees into semantic trees, resolves names and types
- **Analyzers**: Validate semantic correctness, report errors

This separation keeps the body resolver focused on tree construction and makes validation logic easier to test and maintain independently.

## Expressions Followed by Braces Need Special Parsing

If your expression can be followed by `{` (like `for pattern in iterable { ... }`), the sub-expression before the brace must use a trailing-closure-less parser variant. Otherwise, the parser will consume the `{` as a trailing closure argument.

For example, in `for i in range { body }`:
- The `range` expression must be parsed with `condition_binary` (no trailing closures)
- If you use the full `expr_parser`, it will try to parse `{ body }` as a trailing closure on `range`

See `for_expr` in `kestrel-parser/src/expr/mod.rs` for an example.
