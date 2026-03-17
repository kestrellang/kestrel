# kestrel-span Architecture

Source location tracking for the Kestrel compiler. Every token, AST node, HIR expression, and type inference constraint carries a `Span` so that errors can point back to the original source code.

## Pipeline Position

```
Source Text → Lex → Parse → AST Build → Name Res → HIR Lower → Type Infer → Codegen
   ^^^         ^^^    ^^^      ^^^          ^^^        ^^^          ^^^
   spans originate here and propagate through every phase
```

This is a foundational crate — all other lib2 crates depend on it.

## Core Types

| Type | Description |
|------|-------------|
| `Span` | Source location: `file_id` (which file), `start`/`end` (byte offsets) |
| `Spanned<T>` | Generic wrapper pairing any value with its `Span`. `Eq`/`Hash` compare by value only |
| `Name` | Type alias for `Spanned<String>` — an identifier with its source location |
| `SourceFile` | Type alias for `codespan-reporting::SimpleFile` — used for error rendering |

## `Span`

| Field | Type | Description |
|-------|------|-------------|
| `file_id` | `usize` | Opaque identifier for the source file (entity index in the ECS world) |
| `start` | `usize` | Byte offset (inclusive) |
| `end` | `usize` | Byte offset (exclusive) |

Key methods:

- `new(file_id, range)` — create from file ID and byte range
- `synthetic(file_id)` — zero-length span for compiler-generated code (implicit imports, desugared nodes)
- `is_synthetic()` — true when `start == end == 0`
- `range()` — return as `std::ops::Range<usize>`

## `Spanned<T>`

Pairs a value with its source location. Comparison and hashing use only the value — the span is metadata. This means two `Spanned<String>` values with the same string but different locations are considered equal.

## Module Map

| File | Responsibility |
|------|---------------|
| `lib.rs` | `Span`, `Spanned<T>`, `Name`, `SourceFile` — all types in one file |

## Dependencies

| Crate | Usage |
|-------|-------|
| `codespan-reporting` | `SimpleFile` for error rendering integration |
