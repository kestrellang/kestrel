# kestrel-parser

Recursive descent from tokens to a lossless rowan CST. Read
`docs/architecture.md` first; it states the design rules this file enforces.

- **No backtracking, no re-parsing.** Decide with bounded lookahead (a fixed
  number of tokens, or one bracket group through the precomputed matching
  table). The only speculative parse is a type-argument list after an
  expression path segment. A tail expression re-parsed per closure level was
  exponential (audit H7).
- **Every source token is in the tree, in order.** Tokens the grammar can't
  place go in an `Error` node. Zero parse errors must mean zero `Error` nodes
  and an exact round trip (tested over the std).
- **Missing children are absent.** Report a missing `;`, `)` or name and leave
  it out. Never synthesize a token.
- **The tree conforms to `kestrel.ungram`** in kestrel-syntax-tree. A shape
  change edits the grammar first, then the parser, then the readers the
  regenerated views break (`tests/conformance.rs` checks the corpus).
- **Every parse error has an `E8xx` code**, assigned only in
  `syntax_error.rs` and listed in `docs/error-codes.md`. One code per kind of
  mistake, so tests can pin it with `// ERROR(E8xx)`.
- **Line breaks matter only through `Parser::nl_before`**: a call's `(` and a
  trailing closure's `{` must be on the operand's line. Don't add newline
  sensitivity anywhere else without a design note.
- **Condition heads use the expression grammar in condition mode**, not a
  second grammar.
- **Each grammar file opens with the EBNF it implements.** Update it with the
  code.
- New syntax also needs a contextual-keyword check or a token (see
  kestrel-lexer's AGENTS.md) and the `feature` skill's checklist.

## Adding a keyword

A reserved keyword needs every one of these, or it half-works:

1. `Token` variant (kestrel-lexer) and a `kinds.txt` line, appended
   (kestrel-syntax-tree; see its AGENTS.md for the regeneration bootstrap).
2. `is_label_keyword` (`grammar/mod.rs`), so it can still be an argument or
   parameter label like every other keyword.
3. `kind_spelling` (`syntax_error.rs`), or messages print `'token'`.
4. `at_stmt_boundary` (`grammar/blocks.rs`) if it starts a statement, so
   block recovery stops in front of it.
5. Its dispatch:
   - a **statement** (`let`, `guard`, `deinit x;`) is a `block_items` arm in
     `grammar/blocks.rs`;
   - a **statement-like expression** (`if`, `loop`, `match`: no `;` needed) is
     `at_stmt_like` / `stmt_like` in `grammar/exprs.rs` plus `is_stmt_like` in
     `blocks.rs`. These lists are for expressions only.
6. The grammar file's EBNF header and `kestrel.ungram`.
