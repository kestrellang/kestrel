# kestrel-ast

Type syntax (and its arena), operator enums, escape decoding, type
pretty-printing — and anything that must be shared by crates on both sides of
the AST-builder / HIR split, because this is the deepest crate
`kestrel-ast-builder` and `kestrel-hir` can both reach. Function bodies have no
AST: `kestrel-hir-lower` lowers them from the CST.

## `escape.rs` is THE escape table

"What does `\X` mean?" is answered in exactly one place:
`escape::decode_escape`. Callers differ only in what they do with the outcome —
span arithmetic, error recovery, whether errors become data or diagnostics.
Never re-implement the table, not even "just the simple escapes".

It had three implementations, and they disagreed in ways that miscompiled (F26):

- the string decoder (`kestrel-hir-lower::literal`) — the full table;
- the char decoder (`kestrel-hir-lower::pat`) — read `\u{…}` hex with an
  unbounded loop, no close-brace requirement and no digit limit, so
  `'\u{00000041}'` compiled to `'A'` while `"\u{00000041}"` was rejected;
- `unescape_char_simple` (in the since-deleted `kestrel-ast-builder::lower`), used for the literal
  segments of any string containing `\(` — **no `\x` arm, no `\u` arm, no error
  path**, so `"\u{41} \(x)"` silently produced the text `u{41} `.

`EscapeErrorKind` and `UnicodeEscapeErrorReason` live here for the same reason;
`kestrel_hir::body` re-exports them so existing paths keep resolving. There is
still one definition.

### Adding an escape

One arm in `decode_escape`, one case in its `decode_all` test helper. Errors are
data (`EscapeErrorKind`) — this crate never renders a diagnostic. The
`StringEscapeAnalyzer` owns E700-E703, and it reads escape errors off **both**
`HirLiteral::String` and `HirLiteral::Char` through one accessor, so a new
literal kind that starts carrying errors only has to be added there once.

## Adding a variant to a shared enum

`Decoded.raw` must be the exact source text of the sequence, byte for byte:
callers derive spans from `raw.len()` and push it back into the decoded output
on error. `raw_round_trips_the_source_text` pins this.
