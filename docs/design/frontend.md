# Front end: source text to declarations and bodies

How Kestrel gets from a `.ks` file to the declaration entities that name
resolution and type checking work on, and to the HIR of each body. This
describes the front end after the `frontend-rewrite` branch; each section
names the crate that owns the stage.

```
source ──lex──▶ tokens ──parse──▶ CST (rowan) ──build──▶ ECS declarations
        kestrel-lexer    kestrel-parser   kestrel-syntax-tree   kestrel-ast-builder
                                          (kinds, grammar,      (bodies stay CST:
                                           typed views)          Valued(SyntaxNodePtr))
                                               │
                                               └──lower (per body, on demand)──▶ HIR + BodySourceMap
                                                  kestrel-hir-lower (LowerBodyWithSourceMap)
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
`ast_type::lower_type(&ast::Ty)`. Bodies are not lowered here at all: a
declaration with a body (function or initializer, accessor, subscript,
field initializer, parameter default) records only
`Valued(SyntaxNodePtr)`, the one "has a body" component.

**No syntax in components.** A declaration keeps `CstNode(SyntaxNodePtr)`,
its body `Valued(SyntaxNodePtr)`; conformance and where-clause entries keep
pointers too. The file entity owns the tree as `FileSyntax(GreenNode)` (the
immutable, `Send` green tree). `kestrel_ast_builder::syntax::{cst_node,
valued_node}` resolve a pointer through the `World` or a `QueryContext`.

## Bodies (`kestrel-hir-lower`)

`LowerBodyWithSourceMap { entity, root }` lowers one body straight from the
CST to `HirBody`, on demand, and returns it with a `BodySourceMap`;
`LowerBody` is the projection most consumers use. There is no intermediate
body AST (`AstBody` and the ast-builder's `lower.rs` are gone; `kestrel-ast`
keeps only type syntax, operator enums and escape decoding).

- **One walk.** The lowerer reads the generated typed views where they help
  and raw `SyntaxNode` children elsewhere (`syntax.rs` holds the shared CST
  helpers: block/tail split, call arguments, path segments, conditions,
  token → operator mappings). Operators lower as the parser grouped them.
- **Missing syntax is explicit.** Absent children become `ExprSrc::Error` /
  `PatSrc::Error`, and an absent name `HirName::Missing` — never `""`. The
  parser has already reported the gap.
- **HIR is unchanged in shape.** It keeps embedded `Span`s for diagnostics;
  `Local::span` is still the whole declaring statement, because type
  inference anchors and deduplicates "could not infer type" on it.
- **Source map.** `BodySourceMap` links every lowered expression, pattern and
  statement to the node it came from (and back), every user-spelled local to
  its binding node and identifier range, and every path segment that names an
  expression of its own (a local, a member `Field`) to that expression by its
  identifier range. Synthesized ids (`self`, implicit `it`, desugaring
  temporaries) have no entry, so position-based tools fail closed on them.
  The LSP's cursor lookups (hover, go-to-definition, references, highlight,
  rename, code actions) all go through it — none match on spans.
- **Implicit `it`.** A closure without a parameter header gets an `it`
  parameter when its body refers to the *name* `it` — a value path whose
  first segment is `it` — not when the token merely appears (`{ p.it }` has
  no parameter). `it` belongs to the innermost closure written without a
  parameter list; closures with explicit parameters are transparent to the
  search. E142 / E143 are anchored at the first such reference.

## Name resolution (`kestrel-name-res`)

- **Lang items** resolve through the `@builtin(.X)` index only; a declaration
  that merely shares a builtin's name is never taken for it. A second
  annotation of the same builtin is E400.
- **Type scopes** are member scopes: in a struct/enum/protocol body or any of
  its extensions, the lexical names are the type's non-instance members from
  every part (nested types, aliases, enum cases, statics) plus that part's
  own type parameters. Instance members are reached only through `self.`.
- **Selective imports** bind only declarations visible from the importing
  file, like wildcard imports.

## Diagnostics

Every front-end diagnostic has a code: E800–E809 from the parser, the HIR
lowering codes (E010–E011, E122–E141, E213, E317–E321, E708–E710, plus E438 /
E476 shared with the analyzers), and the analyzers' own. See
`docs/error-codes.md`.

## Not yet built

The target design's remaining pieces:

- **Item tree + ID map.** Entities are still created straight from the CST by
  the builders, and components still carry spans (`DeclSpan`, spans inside
  `AstType`). The target is a position-independent item tree per file with
  IDs from (container, kind, name, disambiguator) and an ID map
  (item → `AstPtr`) as the only place spans live.
- **Spans out of HIR.** HIR still embeds `Span`s, and diagnostics in type
  inference, the analyzers and MIR lowering read them. The source map makes
  it possible to derive them instead; that migration has not started.
- **Incremental bodies.** `LowerBodyWithSourceMap` is keyed by entity, and
  entities are not yet stable across edits (no item tree), so a body is
  re-lowered whenever its declarations are rebuilt.
- **Stable identity.** Entities are respawned with fresh IDs when a file is
  rebuilt, which blocks real incremental reuse. (Invalidation itself is now
  durable: memos compare against `EntityRecord::last_changed`, not the
  per-revision change set — audit F20.) (Thread safety is done:
  `ParseResult` holds a `GreenNode`, kestrel-hecs requires `Send + Sync`, and
  `World: Send` is asserted — audit F42, `e7179d31`.)

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
