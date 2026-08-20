# F13 — decisions

A missing `;` after an expression statement in a **function-body block** was
silently accepted. `func f() { side() side(); }` compiled with **zero
diagnostics**; the CST showed `Semicolon@69..69 ""` — a zero-width token, no
`Missing` node, no error.

Two discard points, both now closed:

* `lib/kestrel-parser/src/block/mod.rs` — the `try_map` on `expr_item` collapsed
  the real and the synthesised `;` into one arm (`Some((semi, _synth))`), so the
  synth flag the parser had gone to the trouble of computing was thrown away.
* `lib/kestrel-parser/src/stmt/mod.rs` — `emit_expression_statement` called plain
  `add_token`, while the sibling var-decl emitter `emit_variable_declaration`
  already called `add_token_or_missing`. The two paths disagreed about the same
  zero-width span.

The fix is two edits, ~10 lines including comments. No new diagnostic code: this
rides the existing unnumbered parse-error channel, byte-identical to the
already-working var-decl case.

## 1. No `StmtVariant` change — the synth flag does not need carrying

The audit proposed threading the synth flag onto `StmtVariant::Expression` so
the emitter could see it. **Unnecessary.** `add_token_or_missing` already keys
off `span.start == span.end`, and the var-decl path carries no flag either — it
hands the emitter a possibly-zero-width `Span` and lets the sink decide. Adding
a flag would have introduced a *second* source of truth for "was this token
synthesised?", contradicting the one already in `EventSink`.

The parser-side flag is still consulted, but only where it is actually load-bearing:
the `is_statement_like_expr` guard in §2, which needs to distinguish
"synthesised" from "block end" *before* the emitter runs.

## 2. The crux: the statement-like guard was only reachable at block end

`;` is legitimately absent in exactly two positions:

* (a) the block's **final** expression — its value;
* (b) **statement-like expressions anywhere in the block** — `If | While |
  WhileLet | Loop | For | Match` (`is_statement_like_expr`, `block/mod.rs`).

And (b) is exactly where the old code was wrong. The `try_map` arms were ordered

```rust
Some((semi, _synth)) => …Statement(Expression(expr, semi)),
None if is_statement_like_expr(&expr) => …StatementExpr(expr),
None => Err(…),
```

`maybe_semi` is `None` only when `block_end_lookahead` fires — i.e. **only at
block end**. So the statement-like arm was unreachable mid-block, and a
*mid-block* `if` fell into the synth branch and was mislabelled an expression
statement carrying a fake `;`.

Naïvely swapping `add_token` → `add_token_or_missing` without fixing this would
have emitted **1439 false `expected \`;\`` errors across `lang/` alone** —
every mid-block `if` / `while` / `for` / `match` in the stdlib. The new arm

```rust
Some((_, synth)) if synth && is_statement_like_expr(&expr) => Ok(BlockItem::StatementExpr(expr)),
```

**must precede** the `Some((semi, _synth))` arm, which matches every `Some(..)`.
That ordering is the whole fix; the comment in the source says so.

Measured after the fix with the classifier
(`scratchpad/f13/classify.py`, which walks `kestrel dump cst` for zero-width
`Semicolon` tokens): `lang/` went from 1439 statement-like zero-width sites to
**0**, with 0 non-statement-like sites in either direction.

## 3. The new arm changes CST shape and diagnostics, never AST semantics — verified

Claim: routing a mid-block statement-like expression to `BlockItem::StatementExpr`
instead of `BlockItem::Statement(Expression(expr, synth_semi))` cannot change what
the AST means. Verified against the source, not assumed:

* `emit_code_block` (`block/mod.rs`) emits `StatementExpr` as
  `Statement > ExpressionStatement > expr` — the *identical* node shape
  `emit_expression_statement` produces, minus the zero-text `Semicolon` token.
* `AstLowerer::lower_block` (`kestrel-ast-builder/src/lower.rs`) promotes a
  bare `ExpressionStatement` to a tail expression **only under `is_last`**
  (`i == child_count - 1 || children[i+1..].all(RBrace)`). A mid-block item is
  never `is_last`, so both shapes fall through to the same `lower_stmt` call.

The `is_last` gate also rules out the one shape that could have been dangerous:
a synthesised `;` on the *final* statement. It cannot occur — the synth branch is
reachable only after `block_end_lookahead` (`}` / EOF) has already failed, so
there is always a following item, and `is_last` is therefore always false for a
synthesised statement.

## 4. `block/mod.rs:668` is the only producer of a zero-width `;` that reaches the emitter

The `emit_expression_statement` change is inert everywhere else — checked
exhaustively. Every other producer of `StmtVariant::Expression` requires a
**real** `Semicolon` token and can never hand the emitter a zero-width span:

| site | shape |
| --- | --- |
| `stmt/mod.rs` `expression_statement_parser` | `.then(just(Token::Semicolon))` — mandatory |
| `block/mod.rs` `guard_else_items_parser` | `.or_not()` / `.or(empty().to(None))` → `None`, not a zero-width span |
| `block/mod.rs` `block_items_parser` (inline blocks) | same `.or_not()` shape |
| `block/mod.rs` `code_block_parser` **(this one)** | `.or(empty().map_with(…))` — the only synth |

So the only behavioural delta from the `stmt/mod.rs` edit is F13 itself.

## Out of scope — recorded deliberately, do not "fix" while passing through

### 5a. Nested-block cascade (materially larger, separately risked)

`block_items_parser` and `else_block_items_parser` (`block/mod.rs`) have **no
synth branch at all** — they use a plain `.or_not()` on the `;`. A missing `;`
inside an `if` / `while` / `for` / `match` body, or inside a closure body, does
not go silent; it *hard-fails* with a misdirected cascade. Reproduced post-fix:

```
// { print("a") <newline> print("b"); } inside an if body
error: expected '!', '..', or 13 others, found identifier   → on `print("b")`
error: expected `}`                                          → on the func signature
```

```
// same inside a closure body
error: expected 'module', 'import', or 14 others, found ';'
error: expected `}`                                          → on `let f = { ()`
```

F13's *silence* is specific to the outermost function-body block, which is the
only block with a recovery branch. Giving the inline/else parsers the same
synth-and-recover treatment is a materially larger change with its own blast
radius (every `if` body, every closure body in the stdlib) and was not attempted.

### 5b. `is_statement_like_expr` vs `is_inline_statement_like` disagree — leave them

Two lists, deliberately not unified:

* `block/mod.rs` `is_statement_like_expr`: `If | While | WhileLet | Loop | For | Match`
* `expr/mod.rs` `is_inline_statement_like`: the same six **plus `Return | Throw | Try`**

Pointing the block-level guard at the inline version is a one-line change, and it
is tempting because "two lists that should be one" is a textbook single-source-of-truth
smell. **Do not do it.** It would newly *accept* `func f() { return foo }` — no `;`
— at function-body top level, which today is an error. That is a **grammar change,
not a bug fix**, and needs a maintainer decision, not a refactor. Recorded here so
the next person who notices the smell finds the reason instead of the smell.

### 5c. Maximal-munch absorption is unreachable from this fix

```kestrel
t = a
-b;
```

parses as `t = a - b` and prints `2`, not `5`. No synth span is produced — the
expression parser consumes the newline and the `-` as a binary operator, so there
is no zero-width `;` for `code_block_parser` to observe and **no fix at the
`try_map` can reach it**. Closing it means a newline-sensitive expression
terminator, a much deeper grammar decision.

## Testdata: 7 mechanical corrections, 1 deliberately left alone

A classifier sweep over all 3646 testdata files found exactly **8**
non-statement-like zero-width `;` sites in 7 files. Seven of them, across six
files, are `self.field = …` in an initializer with the `;` genuinely absent:

* `memory_model/deinit/partial_drop_all_fields_initialized.ks` (2 sites)
* `memory_model/deinit/partial_drop_no_fields_initialized.ks`
* `memory_model/deinit/partial_drop_on_init_failure.ks`
* `memory_model/deinit/partial_drop_one_of_two_initialized.ks`
* `validation/initializers/failable_init_partial_return_ok.ks`
* `validation/initializers/failable_init_success_return_requires_init.ks`

These are **invalid Kestrel syntax that the parser used to accept**, so adding
the `;` is correcting the test input, not cajoling a test into passing. Each
file's assertions (`expect-exit`, `// ERROR:` annotations) are untouched.

The **8th**, `memory_model/mutating_closures/mutating_consuming_closure_param.ks`,
is *not* a real missing `;` — the line carries one. Its `Missing` node is a
downstream artifact of an already-failing parse: the test deliberately writes
contradictory `mutating consuming` closure-parameter modifiers and asserts the
diagnostic. Its intent is left untouched.

Note that the sibling *trailing* `self.field = …` lines in those same
initializers (the last item before `}`) are **not** missing a `;` — an assignment
as a block's trailing expression is legal — and were left alone.

## Tests

There was no coverage in either direction. Added, under
`lib/kestrel-test-suite/testdata/statements/missing_semicolon/`:

* **Positive (now rejected)** — 4 `diagnostics` tests asserting `expected \`;\``:
  call-then-call, call-then-print, call-then-let, call-then-return.
* **Negative (must stay accepted)** — 5 `execution` tests, one mid-block case per
  `If | While | For | Match | Loop`, none with a trailing `;`. **These matter more
  than the positive ones**: they guard the ~1439-site `lang/` population, where a
  single false positive breaks the entire stdlib.

Plus two unit tests in `lib/kestrel-parser/src/parser.rs`
(`missing_semicolon_on_expression_statement_is_reported`,
`mid_block_statement_like_expr_needs_no_semicolon`) that assert the CST-level
invariant directly — exactly one `Missing` node and one diagnostic in the first
case, zero of each for all five statement-like forms in the second — following
the convention of the surrounding `missing_close_paren_recovers_with_missing_node`
family.
