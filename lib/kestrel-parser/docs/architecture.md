# kestrel-parser Architecture

Handwritten recursive-descent parser: lexer tokens → events → lossless rowan
CST. Linear time, coded diagnostics, recovery per construct.

## Pipeline Position

```
Source Text → Tokens → Parser → CST (rowan) → AST Build → Name Res → HIR → Type Infer
                        ^^^
                     this crate
```

## Structure

```
tokens ──▶ core::Parser ──▶ internal events ──▶ event::Event ──▶ TreeBuilder ──▶ SyntaxNode
               ▲                (precede links)     (+ coded errors)  (re-inserts trivia)
          grammar/*   one function per construct
```

| File | Responsibility |
|------|---------------|
| `core.rs` | Token source over **non-trivia** tokens (each knows whether a newline preceded it and whether it touches its predecessor), precomputed bracket matching, the marker API (`start` / `complete` / `abandon` / `precede`), bounded speculation (`checkpoint` / `rollback`), and conversion of the buffer into public events (errors sorted by position and deduplicated). |
| `grammar/mod.rs` | Entry points (`source_file`, `expression_only`), `delimited` list helper with per-list recovery. |
| `grammar/items.rs` | Declarations: item list, type bodies (one `Ctx` decides which members a body accepts), parameters, accessors, type aliases. |
| `grammar/attrs.rs` | `@attribute(args)` lists. |
| `grammar/generics.rs` | Type parameter lists, conformance lists, where clauses. |
| `grammar/types.rs` | Type expressions. |
| `grammar/patterns.rs` | Patterns, and the irrefutable parameter-pattern subset. |
| `grammar/exprs.rs` | Expressions, postfix chains, trailing closures, control flow. |
| `grammar/blocks.rs` | Code blocks and statements; the statement-vs-value decision. |
| `event.rs` | `Event`, `EventSink`, `TreeBuilder` (inserts trivia from the source between emitted tokens). |
| `syntax_error.rs` | `E8xx` codes and message construction. Every parse error has a code. |
| `parser.rs` | `ParseResult`, `ParseError`, `Parser::parse`. |

Each grammar file starts with the grammar it implements, in EBNF.

## Key Design Decisions

**No backtracking, no re-parsing.** Choices use bounded lookahead: a fixed
number of tokens, or one bracket group via the precomputed matching table
(closure headers `{ (…) in`, function types `(…) ->`, qualified associated
type targets `P[…].Item`). The single speculative parse is a type-argument
list after an expression path segment (`foo[Int]` vs. an unrelated `[…]`); it
is bounded by its brackets and never contains an expression. A block parses
each expression once and decides afterwards — from its kind and the next
token — whether it is a statement, a statement-like expression, or the
block's value (this was audit H7: the combinator parser re-parsed tail
expressions at every closure level, exponential in nesting).

**Every source token is in the tree, in order, with its own kind.**
Separators and brackets are real tokens of the list they belong to;
trailing closures follow the call's `)` inside its `ArgumentList`
(audit H1). Tokens the grammar cannot place are wrapped in an `Error`
node. Invariant (tested over the whole stdlib): zero parse errors ⇒ zero
`Error` elements and an exact round trip.

**Missing children are absent.** A missing `;`, `)`, `}` or member name is
reported and simply not in the tree; there are no synthesized tokens.

**Line breaks matter in three places**, all via `Parser::nl_before`: a call's
`(` and a trailing closure's `{` (or `label: {`) must be on the operand's
line. Everything else is newline-insensitive, as before.

**Condition mode.** `if`/`while`/`for`/`match` heads, match guards and
`if let` values use the expression grammar with a restriction (no trailing
closures, no assignment, no block-like primaries, a single prefix operator)
instead of a second grammar. Bracketed sub-expressions lift it.

**Operators are parsed by precedence climbing** (`exprs::binary_bp`), so
`ExprBinary` nodes already have the language's precedence and
associativity — later stages lower them as written. Binding powers, loosest
first: `or` 10, `??` 15 (right-associative), `and` 20, comparisons 30
(non-chaining), `..=`/`..<` 40, `+ - | ^` 50, `* / % &` 60, `<< >>` 70.
Each `ExprBinary` spans exactly its operands.

**Interpolated strings are structured.** The lexer splits
`"a\(x)b"` into `StringStart StringFragment InterpStart <tokens> InterpEnd
… StringEnd` (a plain string stays one `String` token); the parser builds
`ExprInterpolatedString > Interpolation > Expression (FormatSpec)?` and
parses each hole as an ordinary expression in place. A broken hole is an
`Error` node with a diagnostic prefixed "invalid expression in string
interpolation"; nothing downstream re-lexes string text.

**The tree conforms to `kestrel.ungram`.** Every error-free parse matches
the grammar in `lib/kestrel-syntax-tree` (`tests/conformance.rs` checks the
whole corpus; debug builds assert it on every parse), so the generated typed
views read it correctly. Grouping parens in a type are a `TyParen` node; an
expression body `func f() = e` is `FunctionBody('=' Expression)`.

## Error Recovery

| Construct | Recovers to |
|-----------|-------------|
| Top-level items | next item start (keyword, modifier, `@`) |
| Type-body members | next member start or `}` |
| Block statements | next statement keyword, expression start, or past `;` |
| Delimited lists | next `,` or the closer (bracket groups skipped whole) |
| Match arms | next `,` or `}` |

A broken construct keeps its node with the parts that parsed; errors are
anchored on the offending token, or — for a missing `;`/`)`/`}` — on the
token before the gap.

## Dependencies

| Crate | Usage |
|-------|-------|
| `stacker` | Stack growth for deeply nested source |
| `kestrel-lexer` | `Token` input |
| `kestrel-syntax-tree` | `SyntaxKind`, `SyntaxNode`, `GreenNodeBuilder` |
| `kestrel-span` | `Span` |
