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
| Literals | `Integer`, `Float`, `String`, `Char`, `RawString`, `Boolean`, `Null` | `String` = a cooked string with no `\(…)` hole |
| Interpolated strings | `StringStart`, `StringFragment`, `InterpStart`, `InterpEnd`, `FormatSpec`, `StringEnd` | Emitted by the modal driver |
| Keywords | `func`, `struct`, `enum`, `let`, `var`, `if`, `while`, `match`, ... | ~40 keywords |
| Operators | `+`, `-`, `==`, `->`, `=>`, `??`, `..=`, `..<`, `<<=`, ... | Longest-match ordering |
| Punctuation | `(`, `)`, `{`, `}`, `[`, `]`, `;`, `,`, `.`, `:` | |
| Special | `Underscore`, `Identifier` | `_` has higher priority than `Identifier` |

## Lexing Strategy

Built on the **logos** procedural macro framework for code, plus a small
modal driver (`modal.rs`) for cooked strings. Simple tokens use regex
patterns; complex tokens use custom callbacks:

| Piece | Handles |
|-------|---------|
| `modal::lex_modal` | Cooked strings: a mode stack of *code* / *string* / *hole*. String mode scans literal text up to `\(` or the closer; hole mode lexes tokens with logos, counting `()[]{}` to find the closing `)`; a `:` at depth 0 starts the format spec. Nested strings push another frame. |
| `parse_pound_string` | `#"…"#` raw strings with variable pound depth (one `RawString` token, no interpolation) |
| `parse_block_comment` | `/* ... */` with nesting |
| `is_valid_identifier` | Unicode identifiers (XID_Start + XID_Continue) |

## Key Design Decisions

**Interpolated strings are lexed modally.** `"a \(x:08x) b"` comes out as
`StringStart StringFragment InterpStart Identifier Colon FormatSpec InterpEnd
StringFragment StringEnd`: hole contents are ordinary tokens, so the parser
parses them in place (no re-lexing anywhere downstream). A string with no
hole is collapsed back into a single `String` token, so literal patterns and
attribute arguments keep one token. In string mode `\` always consumes the
next character (`\"` does not close, `\\(` is not a hole).

**Unterminated strings.** A single-line string that never closes is cut at
its first line break (a *closed* string may span lines), so the rest of the
file still lexes; the token has no closer and is diagnosed downstream (E707
for a plain string, the parser for an interpolated one).

**Trivia preserved.** Unlike many lexers that discard whitespace, this one emits trivia tokens so the rowan-based CST can reconstruct exact source positions.

**Unicode identifiers.** Identifiers follow Unicode XID rules, not ASCII-only.

## Module Map

| File | Responsibility |
|------|---------------|
| `lib.rs` | `Token` enum, `lex()` function, logos callbacks |
| `modal.rs` | Modal lexing of cooked strings and interpolation holes |

## Dependencies

| Crate | Usage |
|-------|-------|
| `logos` | Procedural macro lexer framework |
| `unicode-xid` | Unicode identifier validation |
| `kestrel-span` | `Span`, `Spanned<T>` for source locations |
