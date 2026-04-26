# Kestrel LSP — Hover behavior

A quick reference for what `textDocument/hover` does in different cursor
positions. Implementation lives in `src/handlers/hover.rs`.

The handler tries an **entity hover** first (signature + doc comments for a
declaration), and falls back to an **expression-type hover** (the inferred
type of the expression at the cursor) when there's no declaration to point
at.

---

## Cursor positions

| Cursor on | Hover box | Highlight |
|---|---|---|
| **Function reference** — `foo` in `foo(args)` | ` ```kestrel`<br>`func foo(x: Int, y: Int) -> Int`<br>` ``` `<br>+ leading `///` doc comments | The `foo` identifier |
| **Method/field access** — `bar` in `obj.bar` or `obj.bar()` | Method/field signature + docs | The whole `obj.bar` expression (the smallest `HirExpr` containing the cursor) |
| **Type used as constructor** — `Foo` in `Foo(name: "x")` | Struct/enum/protocol signature trimmed to header + docs | The `Foo` identifier |
| **Implicit-member case** — `.Case` in `match x { .Case => … }` | Enum case signature + docs | The expression span |
| **Declaration name itself** — `foo` in `func foo() { … }` | Same signature + docs as references | Just the identifier portion of the `DeclSpan` (via `get_name_span`) |
| **Local variable** — `x` in `let x = 1; x + 1` | ` ```kestrel`<br>`let x: Int`<br>` ``` `<br>+ `[Go to type definition](file://…#L42)` link | The `x` identifier |
| **Mutable local** — `x` in `var x = 1; x = 2` | Same shape but `var x: Int` | The `x` identifier |
| **Literal** — `42`, `"hello"`, `true` | ` ```kestrel`<br>`Int`<br>` ``` ` | The literal span |
| **Other sub-expression** — operators, calls treated as values | Inferred type as a code block | The expression span |
| **Whitespace / trivia / comments** | (nothing) | n/a |
| **Type position** — `Foo` in `func bar(x: Foo)` or `let y: Foo` | (nothing — deferred; HIR doesn't carry type-position spans as expressions yet) | n/a |

---

## Signature rendering

Signatures are sliced from the source text of the declaration, from the
start of `DeclSpan` to the start of the first body block (`FunctionBody`,
`StructBody`, `EnumBody`, `ProtocolBody`, `ExtensionBody`,
`SubscriptBody`, `PropertyAccessors`, or `CodeBlock`). When there's no
body block (stored fields, type aliases) we use the full `DeclSpan`.
Trailing `;` / whitespace is trimmed.

What that gets you:

- `func foo(x: Int, y: Int) -> Int` (without the body braces)
- `public struct Command: Cloneable` (just the header)
- `init(name: String)` (initializer signature; counts as a "type"-like
  entity for hover purposes)
- `var paddleSize: Int { 4 }` for stored / computed fields — the body of
  computed fields is trimmed too

---

## Doc-comment extraction

The parser splices trivia (whitespace, newlines, comments) into the tree
right before the next significant token, so doc comments end up in
different places depending on the declaration's preamble:

- `public struct Foo` — leading trivia lands **inside** the `Visibility`
  node, before the `public` token.
- `@attr struct Foo` — leading trivia lands **inside** the `AttributeList`.
- `struct Foo` (no preamble) — trivia is a sibling token of the empty
  `Visibility` node, just before the `struct` keyword.

To handle all three cases uniformly the collector walks
`descendants_with_tokens` (an in-order flat token stream) and gathers
every doc comment it sees until it hits the first non-preamble token.
Preamble tokens (`public`, `private`, `internal`, `fileprivate`, `@`) are
transparent; whitespace and newlines are transparent; everything else
ends the walk.

Recognized doc-comment styles:

- **Line doc** — `///` (rejected for `////` section dividers) or `//!`
- **Block doc** — `/** … */` (rejected for `/*** … */`)

Stripping rules:

- `///` / `//!` prefix removed, plus one optional trailing space
- `/** … */` markers removed; per-line leading `*` (with optional space)
  also removed

---

## Highlight ranges

The LSP `Hover.range` controls which text the editor outlines while the
hover popup is open. We pick the narrowest range that makes sense for
each cursor case:

- **Entity hover** — when the cursor sits inside an HIR expression that
  resolves to an entity, we highlight the `HirExpr`'s own span (the
  identifier or call, not the whole declaration). When the cursor is on
  a declaration's own identifier (no enclosing expression), we use the
  identifier span via `get_name_span`. Last resort: zero-width range at
  the cursor.
- **Expression-type fallback** — we use the smallest `HirExpr` at the
  cursor (`semantic::hir_expr_span`).

The `DeclSpan` of the entity is **not** used for the highlight, so you
won't see the whole `func main() { … }` light up when hovering a single
identifier inside its body.

---

## What's NOT yet covered

- **Type-position hover** — `Foo` in `func bar(x: Foo)` or `let y: Foo`.
  These are stored as `HirTy` on the binding, not as a `HirExpr`, so
  there's no expression for `hir_expr_at` to find. Will need a separate
  CST-driven lookup.
- **Overload-set disambiguation** — when the cursor lands on an
  `OverloadSet` (multiple functions sharing a name), we render only the
  first candidate. A real impl would either show all of them or use the
  argument types to pick the chosen one.
- **Method-chain receivers** — `f().g().h` works today via inference's
  `resolutions` map, but only because each `.x` is its own
  `MethodCall`/`Field` HirExpr. If the parser drops part of the chain
  during recovery, hover degrades.
- **Stdlib `lang` builtins** — types like `lang.i64` have no source file,
  so the type-link in the local-variable hover comes back `None` for
  them.

---

## Tests

`src/handlers/hover.rs::tests` covers:

- Doc-line / doc-block predicates and stripping (`is_doc_line`,
  `strip_doc_line`, `is_doc_block`, `strip_doc_block`)
- Function signature + doc rendering (call-site and decl-site cursors)
- Struct signature trimming (body removed)
- Doc collection through `Visibility` (the `public func greet` case)
- Local-variable suppression of entity hover (so `var/let name: Type`
  fires instead)
- File-URI link generation for named types
