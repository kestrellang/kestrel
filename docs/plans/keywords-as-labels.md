# Keywords Usable as Labels

**Status**: Design
**Issue**: [#21](https://github.com/kestrellang/kestrel/issues/21)
**Target**: 0.16

## Summary

Allow all keywords to appear as parameter labels in function, init, subscript, and enum case declarations, and at call sites:

```kestrel
func insert(at index: Int, in list: Array[Int]) -> Array[Int] { ... }

insert(at: 3, in: items)
```

Today these fail because the lexer produces keyword-specific tokens (`Token::For`, `Token::In`, etc.) and the parser only accepts `Token::Identifier` in label position.

## Motivation

Labels are a core part of Kestrel's API design language. Natural English phrasing often requires words that happen to be keywords. The stdlib already has explicit workarounds:

**Result/Optional combinators** — `andValue` and `orValue` exist solely because `and`/`or` are keywords:
```kestrel
// Today (result.ks:246): "Named `andValue` (not `and`) because `and` is a reserved keyword"
func andValue[U](other: Result[U, E]) -> Result[U, E]
func orValue(other: Result[T, E]) -> Result[T, E]

// With this feature:
func and[U](other: Result[U, E]) -> Result[U, E]
func or(other: Result[T, E]) -> Result[T, E]
```

**Iterator adapters** — `matching` is used throughout where `if` or `where` would read better:
```kestrel
// Today:
func filter(matching predicate: (Item) -> Bool) -> FilterIterator[Self]
func take(matching predicate: (Item) -> Bool) -> Optional[Item]
func retain(matching predicate: (K, V) -> Bool)
func removeAll(matching predicate: (K, V) -> Bool)

// With this feature:
func filter(if predicate: (Item) -> Bool) -> FilterIterator[Self]
```

**Logical protocols** (logical.ks:2-3) — explicitly documents the workaround: "Method names use 'logical*' prefix because 'and', 'or', 'not' are keywords."

Note: this feature only covers **labels**, not method names. `logicalAnd`/`logicalOr`/`logicalNot` are method names and would remain unchanged. `andValue`/`orValue` are also method names, so renaming them is a separate decision — but the feature unblocks the option of e.g. `func and[U](other:)` where `and` is both the method name context and would benefit from the label.

## Design Decisions

**All keywords allowed, except `mutating` and `consuming`.** Label position is syntactically unambiguous — always inside a parameter list (after `(`) followed by a bind-name pattern and `:`, or at a call site followed by `:` and an expression. No keyword can start a valid pattern or be confused with its statement-level meaning. `mutating` and `consuming` are excluded because they are already parsed as access modes in parameter position — allowing them as labels would create ambiguity (`func foo(mutating x: Int)` already means `x` is mutating).

**Labels only, not bind-names.** `func foo(for for: Int)` is invalid — the second `for` is a bind-name (local variable), and keywords remain reserved in expression/statement context. Only the label position (the API-facing name) gets the allowance.

**No escaping syntax.** Unlike Swift's backtick approach (`` `in` ``), Kestrel allows keywords directly. Backticks add visual noise and a concept to learn. Since the grammar is unambiguous, no escaping is needed.

**Enum case labels included.** Enum case parameters follow the same label rules:
```kestrel
enum Event {
    case move(to: Point)
    case wait(for: Duration)
}

let e = Event.move(to: origin)
```

## Syntax Grammar

No new grammar productions. The existing `label` position expands from `IDENTIFIER` to `IDENTIFIER | KEYWORD`:

```
parameter      = access_mode? label? pattern ':' type default?
call_argument  = (label ':')? expression
label          = IDENTIFIER | KEYWORD
```

## Pipeline Trace

| Stage | What happens | Changes needed |
|-------|-------------|----------------|
| **Lexer** | Keywords tokenized as `Token::For`, `Token::In`, etc. | None |
| **Parser (params)** | `parameter_parser` accepts label via `select!` on `Token::Identifier` | Widen to accept keyword tokens too |
| **Parser (args)** | `argument_parser` accepts label via `select!` on `Token::Identifier` | Same |
| **Emitter → CST** | Label token emitted into CST | Normalize keyword tokens to `SyntaxKind::Identifier` in label position |
| **AST builder** | `extract_single_param` matches `SyntaxKind::Identifier` before Pattern | None if emitter normalizes; otherwise widen match |
| **AST** | `AstParam.label: Option<String>` | None — already a string |
| **HIR / type-infer / MIR** | Labels compared as strings | None |

### Implementation approach

**Normalize to `SyntaxKind::Identifier` in the emitter.** When the parser matches a keyword token in label position, the emitter writes it as `SyntaxKind::Identifier` in the CST. Every downstream consumer (AST builder, LSP, formatter) sees a uniform `Identifier` token with the keyword text — no shotgun surgery.

## Parser Changes (Detail)

### Keyword-or-identifier combinator

A shared combinator that accepts any keyword or identifier token in label position:

```rust
// In lib/kestrel-parser/src/common/parsers.rs

fn label_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, Span, ParserExtra<'tokens>> + Clone {
    trivia(select! {
        Token::Identifier = e => to_kestrel_span(e.span()),
        // All keywords
        Token::As = e => to_kestrel_span(e.span()),
        Token::And = e => to_kestrel_span(e.span()),
        Token::Break = e => to_kestrel_span(e.span()),
        Token::Case = e => to_kestrel_span(e.span()),
        Token::Continue = e => to_kestrel_span(e.span()),
        Token::Deinit = e => to_kestrel_span(e.span()),
        Token::Else = e => to_kestrel_span(e.span()),
        Token::Enum = e => to_kestrel_span(e.span()),
        Token::Extend = e => to_kestrel_span(e.span()),
        Token::Fileprivate = e => to_kestrel_span(e.span()),
        Token::For = e => to_kestrel_span(e.span()),
        Token::Func = e => to_kestrel_span(e.span()),
        Token::Get = e => to_kestrel_span(e.span()),
        Token::Guard = e => to_kestrel_span(e.span()),
        Token::If = e => to_kestrel_span(e.span()),
        Token::Import = e => to_kestrel_span(e.span()),
        Token::In = e => to_kestrel_span(e.span()),
        Token::Indirect = e => to_kestrel_span(e.span()),
        Token::Init = e => to_kestrel_span(e.span()),
        Token::Internal = e => to_kestrel_span(e.span()),
        Token::Let = e => to_kestrel_span(e.span()),
        Token::Loop = e => to_kestrel_span(e.span()),
        Token::Match = e => to_kestrel_span(e.span()),
        Token::Module = e => to_kestrel_span(e.span()),
        Token::Not = e => to_kestrel_span(e.span()),
        Token::Or = e => to_kestrel_span(e.span()),
        Token::Private = e => to_kestrel_span(e.span()),
        Token::Protocol = e => to_kestrel_span(e.span()),
        Token::Public = e => to_kestrel_span(e.span()),
        Token::Return = e => to_kestrel_span(e.span()),
        Token::Set = e => to_kestrel_span(e.span()),
        Token::Static = e => to_kestrel_span(e.span()),
        Token::Struct = e => to_kestrel_span(e.span()),
        Token::Subscript = e => to_kestrel_span(e.span()),
        Token::Throw = e => to_kestrel_span(e.span()),
        Token::Throws = e => to_kestrel_span(e.span()),
        Token::Try = e => to_kestrel_span(e.span()),
        Token::Type = e => to_kestrel_span(e.span()),
        Token::Var = e => to_kestrel_span(e.span()),
        Token::Where = e => to_kestrel_span(e.span()),
        Token::While = e => to_kestrel_span(e.span()),
    })
}
```

If `Token` has (or can gain) a method like `fn text(&self) -> &str` or a `filter_map` approach, this could be simplified to avoid listing every variant. Worth checking during implementation.

### Files to modify

1. **`lib/kestrel-parser/src/common/parsers.rs`** — `parameter_parser` (line 323): replace `ident` with `label_parser()`
2. **`lib/kestrel-parser/src/expr/postfix.rs`** — `argument_parser` (line 71): same replacement
3. **`lib/kestrel-parser/src/common/emitters.rs`** (or equivalent emitter) — normalize keyword tokens to `SyntaxKind::Identifier` when emitting in label position
4. **`lib/kestrel-ast-builder/src/builders/params.rs`** — likely no change if emitter normalizes

## Diagnostics

No new diagnostics needed. Keywords in label position are valid by construction.

## Testing

### Parser tests

- Function with keyword labels: `func insert(at index: Int, in list: Array[Int])`
- Call with keyword labels: `insert(at: 0, in: items)`
- Subscript with keyword labels: `subscript(for key: String) -> Value`
- Enum case with keyword labels: `case move(to: Point)`
- Representative keyword sample: `for`, `in`, `as`, `if`, `while`, `try`, `return`, `match`, `guard`, `let`, `var`, `get`, `set`, `and`, `or`, `not`
- Mixed keyword and identifier labels: `func foo(in x: Int, name y: String)`

### Integration tests (`.ks` testdata)

- **Execution test**: define and call a function with keyword labels, verify correct dispatch
- **Overload resolution**: keyword labels participate in overload identity (e.g., `func foo(in x: Int)` vs `func foo(at x: Int)` are distinct)
- **Enum case construction and matching**: enum cases with keyword labels can be constructed and pattern-matched
- **Error test**: type mismatch at a keyword-labeled argument still reports correctly

## Resolved Questions

1. **`mutating`/`consuming` excluded.** These are already parsed as access modes in parameter position, so they are excluded from the allowed keyword set to avoid ambiguity.

2. **LSP highlighting.** Keyword-labels are colored as labels, not as keywords. This falls out naturally from normalizing to `SyntaxKind::Identifier` in the emitter — the LSP sees an `Identifier` token and highlights it as a label.

3. **Keywords as method names — out of scope, permanently.** This feature covers labels only. `logicalAnd`/`logicalOr`/`logicalNot` and `andValue`/`orValue` are method names and are unaffected.
