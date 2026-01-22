# Character Literals Design

## Overview

Add character literal syntax to Kestrel using single quotes (`'a'`). Character literals represent Unicode scalar values (code points) and integrate with the `ExpressibleByCharLiteral` protocol system for flexible type inference.

## Motivation

Currently, working with characters in Kestrel requires verbose constructs like `CodePoint(UInt32(intLiteral: 65))` for the letter 'A'. Character literals provide:
- Readable character constants: `'A'` instead of magic numbers
- Direct support for escape sequences: `'\n'`, `'\t'`, `'\u{1F600}'`
- Type flexibility through the literal protocol system

## Syntax

```kestrel
// Basic ASCII characters
let a = 'a'
let space = ' '
let zero = '0'

// Escape sequences
let newline = '\n'
let tab = '\t'
let quote = '\''
let backslash = '\\'

// Unicode characters
let omega = 'Ω'
let emoji = '🦅'
let unicode = '\u{1F600}'

// Hex escape (ASCII range)
let bell = '\x07'
```

## Semantic Behavior

### Type Inference

Character literals use inference types with the `ExpressibleByCharLiteral` protocol:

```kestrel
// Infers to lang.i32 (default)
let c = 'a'

// Infers to Char if context requires it (after stdlib rename)
let ch: Char = 'a'

// Infers to UInt32 if that type implements ExpressibleByCharLiteral
let u: UInt32 = 'a'
```

### Internal Representation

- Character literals are stored as `u32` values internally (Unicode scalar values)
- Valid range: 0x0000 to 0x10FFFF, excluding surrogate pairs (0xD800-0xDFFF)
- Default type when no context: `lang.i32`

### Escape Sequences

Supported escape sequences (consistent with string literals):

| Escape | Meaning | Value |
|--------|---------|-------|
| `\'` | Single quote | 0x27 |
| `\"` | Double quote | 0x22 |
| `\\` | Backslash | 0x5C |
| `\n` | Newline | 0x0A |
| `\r` | Carriage return | 0x0D |
| `\t` | Tab | 0x09 |
| `\0` | Null | 0x00 |
| `\xNN` | Hex byte (ASCII, 00-7F) | NN |
| `\u{N...}` | Unicode scalar (1-6 hex digits) | N... |

## Error Cases

| Condition | Error Message |
|-----------|---------------|
| Empty literal `''` | "empty character literal" |
| Multiple characters `'ab'` | "character literal may only contain one codepoint" |
| Invalid escape `'\q'` | "invalid escape sequence '\\q'" |
| Incomplete escape `'\x4'` | "incomplete escape sequence" |
| Hex out of range `'\xFF'` | "ascii escape out of range, must be 0x00-0x7F" |
| Unicode out of range `'\u{FFFFFF}'` | "unicode escape out of range" |
| Surrogate code point `'\u{D800}'` | "unicode escape is a surrogate code point" |
| Unterminated `'a` | Lexer error: unterminated character literal |

## Edge Cases

1. **Single quote in character**: Use escape `'\''`
2. **Backslash**: Use escape `'\\'`
3. **Multi-byte UTF-8**: Allowed - `'Ω'`, `'日'`, `'🦅'` are all single code points
4. **Grapheme clusters**: Error - `'é'` with combining accent (2 code points) produces "character literal may only contain one codepoint"
5. **Newline in literal**: NOT allowed (use `'\n'` escape)
6. **Maximum value**: `'\u{10FFFF}'` is the highest valid Unicode scalar

## Type Model (Future stdlib alignment)

The stdlib will be updated (separate task) to rename:
- `CodePoint` → `Char` (single Unicode scalar value)
- `Char` → `Grapheme` (extended grapheme cluster)

This gives the intuitive model:
- `'a'` → `Char` via `ExpressibleByCharLiteral`
- `"é"` → `Grapheme` via `ExpressibleByStringLiteral`

## Open Questions (Resolved)

1. **Q: Unicode vs byte?**
   A: Unicode scalar values (full Unicode support)

2. **Q: Default type?**
   A: `lang.i32` to hold any Unicode scalar value

3. **Q: Literal protocol?**
   A: Yes, add `ExpressibleByCharLiteral` for type flexibility

4. **Q: Where to validate?**
   A: Lexer tokenizes permissively (like strings), semantic layer validates content

5. **Q: Multi-codepoint graphemes?**
   A: Error - character literals are single code points only. Use string literals for grapheme clusters.

6. **Q: Naming model?**
   A: `Char` = code point, `Grapheme` = cluster (stdlib rename is separate task)
