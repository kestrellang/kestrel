# Character Literals Implementation Plan

## Test Strategy

### Test Categories
1. **Basic literals** - ASCII characters, digits, symbols
2. **Escape sequences** - `\n`, `\t`, `\'`, `\\`, `\0`, `\xNN`, `\u{NNNN}`
3. **Unicode characters** - Multi-byte UTF-8, emoji, international characters
4. **Error cases** - Empty, multi-char, invalid escapes, out of range, grapheme clusters
5. **Type inference** - Default to `lang.i32`, protocol-based inference

### Key Behaviors to Verify
- Character literals parse and compile
- Escape sequences produce correct values
- Invalid literals produce appropriate errors
- Type inference works with `ExpressibleByCharLiteral`

## Implementation Phases

### Phase 0: Tests (First!)
Files: `lib/kestrel-test-suite/tests/char_literals.rs`

**Basic literals:**
- [ ] ASCII letters: `'a'`, `'Z'`
- [ ] Digits: `'0'`, `'9'`
- [ ] Symbols: `'!'`, `'@'`, `' '` (space)

**Escape sequences:**
- [ ] Basic escapes: `'\n'`, `'\t'`, `'\r'`, `'\0'`
- [ ] Quote escapes: `'\''`, `'\"'`, `'\\'`
- [ ] Hex escapes: `'\x00'`, `'\x41'` (A), `'\x7F'`
- [ ] Unicode escapes: `'\u{0}'`, `'\u{41}'`, `'\u{1F600}'`, `'\u{10FFFF}'`

**Unicode characters (single code points):**
- [ ] Greek: `'Ω'` (U+03A9, single code point)
- [ ] CJK: `'日'` (U+65E5, single code point)
- [ ] Emoji: `'🦅'` (U+1F985, single code point)
- [ ] Precomposed: `'é'` (U+00E9, single code point - NOT decomposed)

**Error: multiple characters:**
- [ ] Two ASCII chars: `'ab'` → "character literal may only contain one codepoint"
- [ ] Three ASCII chars: `'abc'` → error
- [ ] Multiple escapes: `'\n\t'` → error

**Error: grapheme clusters (multiple code points that look like one character):**
- [ ] Decomposed é: `'e\u{0301}'` (e + combining acute) → error
- [ ] Family emoji: `'👨‍👩‍👧'` (multiple code points with ZWJ) → error
- [ ] Flag emoji: `'🇺🇸'` (two regional indicators) → error

**Error: invalid escapes:**
- [ ] Invalid escape char: `'\q'` → "invalid escape sequence"
- [ ] Incomplete hex: `'\x4'` → "incomplete escape sequence"
- [ ] Hex out of range: `'\xFF'` → "ascii escape out of range"
- [ ] Unicode out of range: `'\u{FFFFFF}'` → "unicode escape out of range"
- [ ] Surrogate code point: `'\u{D800}'` → "surrogate code point"
- [ ] Missing unicode braces: `'\u0041'` → error

**Error: empty literal:**
- [ ] Empty: `''` → "empty character literal"

**Type inference:**
- [ ] Default inference to `lang.i32`
- [ ] Char literal in return position with `lang.i32` return type

### Phase 1: Lexer
Files: `lib/kestrel-lexer/src/lib.rs`

- [ ] Add `Char` token variant to `Token` enum
- [ ] Add regex pattern for character literals: `'([^'\\]|\\.)*'`
  - Matches single quotes with content (escape sequences or regular chars)
  - Similar pattern to string literals but with single quotes

### Phase 2: Syntax Tree
Files: `lib/kestrel-syntax-tree/src/lib.rs`

- [ ] Add `Char` to `SyntaxKind` enum (token kind, near `String`)
- [ ] Add `ExprChar` to `SyntaxKind` enum (expression kind, near `ExprString`)
- [ ] Update `kind_from_raw()` match for both new variants

### Phase 3: Parser
Files: `lib/kestrel-parser/src/expr/mod.rs`

- [ ] Add `Char(Span)` variant to `ExprVariant` enum
- [ ] Add parser case for `Token::Char` in expression parsing
- [ ] Emit `SyntaxKind::ExprChar` containing the `Char` token

### Phase 4: Semantic Tree
Files: `lib/kestrel-semantic-tree/src/expr.rs`, `lib/kestrel-semantic-tree/src/builtins.rs`

- [ ] Add `Char(u32)` variant to `LiteralValue` enum
- [ ] Add `ExpressibleByCharLiteral` to `LanguageFeature` enum in builtins.rs
- [ ] Update `from_name()` in builtins.rs to parse "ExpressibleByCharLiteral"
- [ ] Add `Expression::char_infer(value: u32, span: Span)` helper method
- [ ] Update `debug_compact()` to handle `LiteralValue::Char`

### Phase 5: Body Resolver (BIND)
Files: `lib/kestrel-semantic-tree-binder/src/body_resolver/expressions.rs`

- [ ] Add `SyntaxKind::ExprChar` match arm in `resolve_expression()`
- [ ] Create `extract_char_value()` function:
  - Strip surrounding single quotes
  - Call `unescape_char()` to process content
  - Validate single code point
  - Return `u32` value
- [ ] Create `unescape_char()` function (reuse logic from `unescape_string()`)
- [ ] Add error diagnostics:
  - `EmptyCharacterLiteralError`
  - `MultipleCodepointsInCharLiteralError`
  - (Escape errors reuse existing: `InvalidEscapeSequenceError`, etc.)

### Phase 6: Type Inference
Files: `lib/kestrel-semantic-type-inference/src/constraint_generator.rs`

- [ ] Add `LiteralValue::Char` case in literal constraint generation
- [ ] Generate `ExpressibleByCharLiteral` protocol constraint
- [ ] Set default type to `lang.i32`

### Phase 7: Codegen (if needed)
Files: `lib/kestrel-compiler/src/` (likely minimal changes)

- [ ] Verify `LiteralValue::Char` is handled in codegen
- [ ] Should compile to i32 constant (similar to integer literals)

## Verification

After each phase:
```bash
cargo test
```

Final verification:
```bash
cargo test
cargo clippy
cargo fmt --check
```

## File Change Summary

| File | Changes |
|------|---------|
| `lib/kestrel-lexer/src/lib.rs` | Add `Char` token |
| `lib/kestrel-syntax-tree/src/lib.rs` | Add `Char`, `ExprChar` to SyntaxKind |
| `lib/kestrel-parser/src/expr/mod.rs` | Add `Char` variant and parser |
| `lib/kestrel-semantic-tree/src/expr.rs` | Add `LiteralValue::Char`, helper |
| `lib/kestrel-semantic-tree/src/builtins.rs` | Add `ExpressibleByCharLiteral` |
| `lib/kestrel-semantic-tree-binder/src/body_resolver/expressions.rs` | Add char resolution |
| `lib/kestrel-semantic-type-inference/src/constraint_generator.rs` | Add char constraint |
| `lib/kestrel-test-suite/tests/char_literals.rs` | New test file |
| `lib/kestrel-test-suite/tests/mod.rs` | Register test module |

## Dependencies

- Phase 1 (Lexer) has no dependencies
- Phase 2 (Syntax) depends on Phase 1
- Phase 3 (Parser) depends on Phase 2
- Phase 4 (Semantic) has no dependencies (can parallel with 1-3)
- Phase 5 (Body Resolver) depends on Phases 2, 3, 4
- Phase 6 (Type Inference) depends on Phase 4
- Phase 7 (Codegen) depends on Phase 4
