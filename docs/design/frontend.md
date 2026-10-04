# Front end: source text to declarations

How Kestrel gets from a `.ks` file to the declaration entities that name
resolution and type checking work on. This describes the front end after the
`frontend-rewrite` branch; each section names the crate that owns the stage.

```
source ──lex──▶ tokens ──parse──▶ CST (rowan) ──build──▶ ECS declarations
        kestrel-lexer    kestrel-parser   kestrel-syntax-tree   kestrel-ast-builder
                                          (kinds, grammar,      (+ AstBody for bodies,
                                           typed views)          lowered to HIR later)
```

## Principles

1. **The grammar is a file.** `lib/kestrel-syntax-tree/kestrel.ungram`
   describes every node's children. The typed views and the validator are
   generated from it; the parser is tested to conform to it. A tree-shape
   change starts there.
2. **The CST is lossless and honest.** Every source byte is in the tree, in
   order, with its own kind. Error-free source produces no `Error` elements.
   Missing children are absent, never synthesized.
3. **Each fact is decided once.** Precedence is decided by the parser, not
   regrouped later; interpolation holes are parsed by the parser, not
   re-lexed from string text; a token's spelling lives in `kinds.txt`.
4. **Linear time.** No backtracking: bounded lookahead plus a precomputed
   bracket-matching table. Nested trailing closures parse in linear time.

## Lexing (`kestrel-lexer`)

`lex()` is a modal scanner over logos tokens. Ordinary code is one mode;
inside an interpolated string the lexer emits
`StringStart (StringFragment | InterpStart <code tokens> (Colon FormatSpec)? InterpEnd)* StringEnd`,
tracking bracket depth so `"\(f(a: [1: 2]))"` closes the hole at the right
`)`. A `:` at depth 0 of a hole starts its format spec. A string without
holes stays one `String` token. An unterminated single-line string ends at
the first newline.

## Parsing (`kestrel-parser`)

A handwritten recursive-descent parser in the rust-analyzer style: a token
source over non-trivia tokens (each records whether a newline precedes it and
whether it is joined to the next), markers (`start`/`complete`/`precede`/
`abandon`) that produce events, and a tree builder that re-inserts trivia
from the source. See `lib/kestrel-parser/docs/architecture.md` for the
statement-vs-value rules, condition mode, and recovery points.

Syntax errors carry codes E800–E809 (`docs/error-codes.md`). Errors are
anchored on the offending token, or for a missing closer on the token before
the gap, and sorted by position.

Expressions use precedence climbing with one table
(`exprs::binary_binding_power`): `or` < `??` (right-assoc) < `and` <
comparisons (non-chaining) < ranges < additive/bitwise-or < multiplicative/
bitwise-and < shifts.

## The CST (`kestrel-syntax-tree`)

- `kinds.txt` — every `SyntaxKind`, append-only (rowan stores raw `u16`s).
- `kestrel.ungram` — one rule per node kind.
- `src/ast/` — generated typed views: a struct per node with an accessor per
  named child, an enum per union (`Item`, `Expr`, `Ty`, `Pat`, …), plus
  `ext.rs` with the `Has*` traits shared by declarations. `AstPtr<N>` is a
  `Send`, hashable kind+range handle for keeping a node across stages.
- `src/validate/` — `validate(&root)` lists the nodes whose children do not
  match their rule. `kestrel-parser/tests/conformance.rs` runs it over the
  whole corpus; debug builds assert it on every error-free parse.

`UPDATE_GENERATED=1 cargo test -p kestrel-syntax-tree --test sourcegen`
regenerates the three generated files; without the variable the test fails
when they are stale.

## Declarations (`kestrel-ast-builder`)

`build_declarations` walks `ast::SourceFile::items()` with an explicit stack
and dispatches on the `ast::Item` variant. Builders read their declaration
through its view; shared helpers take the `Has*` traits. Types lower with
`ast_type::lower_type(&ast::Ty)`. Function, getter and initializer bodies are
lowered to `AstBody` by `lower.rs` (still an untyped CST walk) and from there
to HIR by `kestrel-hir-lower`, which lowers operators as written.

## Verification

`scripts/frontend-oracle.py` is the differential oracle used for every step
of the rewrite. `collect` runs a `kestrel` binary over the corpus
(`lib/kestrel-test-suite/testdata`, `lang`, `examples`) and stores, per file,
the CST dump, the diagnostics, and (for execution tests) build+run results;
`compare` diffs two collections. CST comparison normalizes the intended shape
changes (trivia, the baseline's lost/duplicated tokens, interpolation,
operator grouping, trailing-closure order, `TyParen`, single expression-body
wrapper); diagnostic comparison is order-insensitive, simulates each
diagnostics test's verdict, and re-runs differing files to separate the
compiler's pre-existing nondeterminism from real differences.
