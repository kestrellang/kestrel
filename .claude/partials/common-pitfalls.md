# Common Pitfalls

## Lexer/Parser

1. **Forgetting to update `kind_from_raw()`** - Adding `SyntaxKind` but not the match statement in `kind_from_raw()` causes runtime errors

2. **Wrong event order** - Not matching `start_node()`/`finish_node()` pairs causes malformed trees

3. **Missing Name wrapper** - Emitting `Identifier` directly instead of wrapping in `Name` node breaks uniform extraction

4. **Missing Visibility node** - Not emitting `Visibility` node when no modifier present (must always emit, even if empty)

5. **Not adding to declaration_item** - Creating parser but forgetting to add to `.or()` chain in `declaration_item_parser_internal()` - feature won't parse!

## Semantic

6. **Not registering builder** - Creating builder but not adding to `lowerer.rs` `builder_for()` match

7. **Not registering binder** - Creating binder but not adding to `DeclarationBinderRegistry::new()`

8. **Wrong symbol kind** - Using wrong `KestrelSymbolKind` variant

9. **Missing parent-child link** - Creating symbol but not adding to parent via `add_child()`

10. **Storing data in structs** - Storing strings instead of wrapping `SyntaxNode` (breaks lossless property)

## Validation

11. **Wrong visibility token** - Using `SyntaxKind::Visibility` for the token instead of `SyntaxKind::Public`, etc.

12. **Creating wrapper when not needed** - Adding unnecessary wrappers for single-use syntax

## Testing

13. **Test not running** - Forgetting to add `mod {feature};` to the test module's `mod.rs`

14. **Wrong error substring** - Using error substring that doesn't match actual error message

## Debugging Tips

- If symbols aren't appearing: check builder registration, check `declaration_item` chain
- If types aren't resolving: check binder registration, check `BodyResolver` match arms
- If tests fail silently: run with `-- --nocapture` to see output

For debugging workflow: `docs/contributing/workflows.md#debugging-semantic-resolution-issues`
