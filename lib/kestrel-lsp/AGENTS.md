# kestrel-lsp — Agent Guide

## Position lookups go through the source map — never span matching

Every "what is at this position?" question (hover, definition, references,
rename, highlight, completion context) resolves through `BodySourceMap`
(`kestrel-hir-lower/src/source_map.rs`) for body contents and through the
`CstNode` pointers on entities for declarations. Do not match HIR `Span`s
against the cursor and do not search source text backwards.

**Why:** HIR spans are not unique — desugared nodes share their source's span,
and a local's `span` covers its whole `let` statement. Span matching made
desugared nodes steal hover and made rename edit the wrong range (audit F2: it
replaced whole statements and inserted at file offset 0). The source map is
written by the same walk that lowers the body, so it cannot disagree with HIR.
When a lookup finds no entry (a synthesized local), refuse the operation.
