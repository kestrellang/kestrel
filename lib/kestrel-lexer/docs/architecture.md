# kestrel-lexer Architecture

Lexical analysis for the Kestrel compiler. Converts source text into a stream of typed tokens with precise source spans, as the first phase of compilation.

## Pipeline Position

```
Source Text → Lexer → Tokens → Parser → CST → AST Build → Name Res → HIR → Type Infer
               ^^^
            this crate
```

The lexer produces `Spanned<Token>` values. Trivia tokens (whitespace, comments) are emitted rather than skipped — the parser uses them for CST source position tracking.

## Core Types

| Type | Description |
|------|-------------|
| `Token` | Enum with ~70 variants covering all Kestrel lexemes |
| `lex(source, file_id)` | Main entry point — returns an iterator of `Result<SpannedToken, Spanned<()>>` |

## Token Categories

| Category | Examples | Notes |
|----------|----------|-------|
| Trivia | `Whitespace`, `Newline`, `LineComment`, `BlockComment` | Preserved for CST positions |
| Literals | `Integer`, `Float`, `String`, `Char`, `RawString`, `Boolean`, `Null` | |
| Keywords | `func`, `struct`, `enum`, `let`, `var`, `if`, `while`, `match`, ... | ~40 keywords |
| Operators | `+`, `-`, `==`, `->`, `=>`, `??`, `..=`, `..<`, `<<=`, ... | Longest-match ordering |
| Punctuation | `(`, `)`, `{`, `}`, `[`, `]`, `;`, `,`, `.`, `:` | |
| Special | `Underscore`, `Identifier` | `_` has higher priority than `Identifier` |

## Lexing Strategy

Built on the **logos** procedural macro framework. Simple tokens use regex patterns; complex tokens use custom callbacks:

| Callback | Handles | Complexity |
|----------|---------|-----------|
| `parse_string` | String literals with `\(...)` interpolation | Nested strings inside interpolations |
| `scan_interpolation` | Expression scanning inside `\(...)` | Tracks paren/bracket/brace depth |
| `parse_raw_string` | `"""..."""` with variable quote depth | Counts opening/closing quotes |
| `parse_block_comment` | `/* ... */` with nesting | Tracks comment depth |
| `is_valid_identifier` | Unicode identifiers | XID_Start + XID_Continue rules |

## Key Design Decisions

**Interpolated strings as single tokens.** The lexer emits the entire interpolated string `"\(expr)"` as one `String` token. The parser splits it later. This keeps the lexer stateless.

**Trivia preserved.** Unlike many lexers that discard whitespace, this one emits trivia tokens so the rowan-based CST can reconstruct exact source positions.

**Unicode identifiers.** Identifiers follow Unicode XID rules, not ASCII-only.

## Module Map

| File | Responsibility |
|------|---------------|
| `lib.rs` | `Token` enum, `lex()` function, all custom parsing callbacks |

## Dependencies

| Crate | Usage |
|-------|-------|
| `logos` | Procedural macro lexer framework |
| `unicode-xid` | Unicode identifier validation |
| `kestrel-span` | `Span`, `Spanned<T>` for source locations |
