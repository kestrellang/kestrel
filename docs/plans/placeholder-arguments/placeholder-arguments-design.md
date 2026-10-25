# Placeholder Arguments (`f(_, 1)`, `map(_.name)`) Design

> **Status: design, not started.** Milestone 0.19 ("Expression sugar").
> Base commit `def25459` (`future/0.18`). File and line anchors refer to that
> tree. Companion designs: [pipe operator](../pipe-operator/pipe-operator-design.md),
> [optional chaining](../optional-chaining/optional-chaining-design.md).

## Overview

A placeholder `_` in a call's arguments builds a function value. It has two
forms, told apart by what follows the `_`:

```kestrel
// 1. Hole: a bare `_` argument leaves that slot of the call open.
let addOne = add(_, 1)                    // { (a) in add(a, 1) }
let parseHex = Int64.parse(_, radix: 16)

// 2. Member placeholder: `_` heading a chain makes that ARGUMENT a closure.
let names = users.map(_.name)             // users.map({ (u) in u.name })
let adults = users.filter(_.age >= 18)    // users.filter({ (u) in u.age >= 18 })
let byAge = users.sorted(by: _.age)
let cities = users.map(_.address?.city)
```

| Form | Spelled | Closure covers | Params |
|---|---|---|---|
| **Hole** | `_` as the whole argument | the **call** | one per hole, left to right |
| **Member placeholder** | `_` followed by `.`, `?.`, `(`, `!` inside an argument | that **argument** | exactly one |

Both are pure sugar for a closure literal. Nothing after HIR lowering knows `_`
was written: inference, captures, closure kinds and MIR see a closure.

Holes are also the slot that the [pipe operator](../pipe-operator/pipe-operator-design.md)
fills: `x |> f(a, _)` substitutes `x` directly and builds no closure.

### Non-goals

- **Bare operator sections** (`_ + 1`, `_ < limit` with no member access).
  `{ it + 1 }` covers them, and `_ + 1` reads too much like a hole.
- **Several member placeholders in one argument** (Scala's `_.a + _.b` taking
  two parameters). That's an error, and you write a closure instead.
- **`_` outside call arguments** (`let f = _.name`). There's no expected type
  to give the parameter a type, and `{ it.name }` works.
- **Placeholders in type position.** `_` in types already means "infer". That's
  unchanged.

## Syntax

```
argument     ::= (label ':')? (hole | expr)
hole         ::= '_'                         // followed by ',' or ')'
primary      ::= … | placeholder_head        // only inside an argument (see below)
placeholder_head ::= '_'                     // must be followed by '.', '?.', '(' or '!'
```

| Written | Means |
|---|---|
| `f(_)` | `{ ($p0) in f($p0) }` |
| `f(_, 1)` | `{ ($p0) in f($p0, 1) }` |
| `f(_, _)` | `{ ($p0, $p1) in f($p0, $p1) }` |
| `f(x: _, y: 2)` | `{ ($p0) in f(x: $p0, y: 2) }` |
| `Point(x: _, y: _)` | `{ ($p0, $p1) in Point(x: $p0, y: $p1) }` |
| `xs.map(_.name)` | `xs.map({ ($p0) in $p0.name })` |
| `xs.filter(_.age >= 18)` | `xs.filter({ ($p0) in $p0.age >= 18 })` (the whole argument) |
| `xs.map(_.name.count)` | `xs.map({ ($p0) in $p0.name.count })` |
| `xs.map(_?.name)` | `xs.map({ ($p0) in $p0?.name })` |
| `fs.map(_(x))` | `fs.map({ ($p0) in $p0(x) })` (call each function) |
| `f(g(_))` | `f({ ($p0) in g($p0) })`: a hole binds to the nearest call |
| `f(g(_.x))` | `f(g({ ($p0) in $p0.x }))`: a member placeholder binds to the nearest argument |
| `xs.map(_.a + _.b)` | error E812 (two member placeholders in one argument) |
| `f(_ + 1)` | error E811 (`_` is neither a whole argument nor a chain head) |
| `let y = _` | error E811 |

**What "inside an argument" means.** The parser counts open argument lists.
Closure bodies, blocks and parenthesized sub-expressions *inside* the argument
don't reset the count, but **closure literals do**. `f({ _.x })` is an error,
because the closure already has `it`.

Trailing closures are unaffected. `f(_) { body }` is
`{ ($p0) in f($p0) { body } }`.

The closure's type is unlabeled (`(A, B) -> R`), like every Kestrel function
type. Labels stay on the inner call.

## Semantics

### 1. Desugaring holes

`f(a₁, …, aₙ)` where the arguments at positions `i₁ < … < iₖ` are holes
lowers to:

```
HirExpr::Sugar {
    kind: SugarKind::Placeholder,
    inner: HirExpr::Closure {
        params: [$p0 … $p(k-1)],          // fresh locals, no annotation
        body: { f(a₁, …, $p0 at i₁, …, $p(k-1) at iₖ, …, aₙ) },
    },
}
```

### 2. Desugaring member placeholders

An argument whose expression contains exactly one `ExprPlaceholderHead` (not
counting any nested inside an inner argument list) lowers to:

```
HirCallArg {
    label,
    value: Sugar(Placeholder, Closure { params: [$p0], body: { <arg with _ ↦ $p0> } }),
}
```

A call can mix the forms. In `zip(_, with: _.id)`, the second argument
becomes a closure first, then the hole makes the whole call a closure:
`{ ($p0) in zip($p0, with: { ($p1) in $p1.id }) }`.

The `$p` names are synthetic and can't collide with user names (the `$`
prefix is the existing convention: `$try_value`, `$cast`, `$iter`).

### 3. Evaluation of the other arguments

Everything that isn't the placeholder is **inside the closure body**, so it's
evaluated **on every call**, not when the closure is created:

```kestrel
let f = log(_, timestamp())        // timestamp() runs each time f is called
xs.filter(_.score > threshold())   // threshold() runs once per element
```

This is the same rule as Scala, and it falls out of the desugaring. To
evaluate once, bind it first: `let t = threshold(); xs.filter(_.score > t)`.

### 4. Types and closure kind

The synthesized closure goes through `gen_closure`
(`kestrel-type-infer/src/generate.rs:915`) like any closure literal:

- It's inserted into `closure_literal_exprs`, so the parameter-convention
  inference (borrow, `MutBorrow` upgrade) applies.
- With an expected function type (`xs.map(_.name)`, `xs.map(add(_, 1))`),
  parameter types and the closure kind come from the expectation. This is
  what makes member placeholders work, since `_.name` needs to know `_`'s type.
  It's the same mechanism as `xs.map { it.name }`.
- With no expectation, hole parameters get their types from the call
  (`let g = add(_, 1)`). A member placeholder with no expectation is the
  existing "could not infer type" error, as for `let f = { it.name }`
  (`testdata/expressions/closures/cannot_infer_it_type_without_context.ks`).
- An overloaded callee with no expectation (`let g = print(_)`) may also be
  "could not infer type", anchored on the `_`.

### 5. Captures

Free variables of the closure body are captured by place through
`ClosureCaptures` (`kestrel-type-infer/src/captures.rs`), exactly as for a
hand-written closure.

### 6. Interaction with pipe

When a call with **holes** is the right side of `|>`, the pipe lowering
substitutes the left side into the (single) hole and builds no closure. Member
placeholders inside that call's other arguments still become closures:
`users |> sortBy(_, key: _.age)` → `sortBy(users, key: { (p) in p.age })`.

## Current state

On `def25459`:

- `Underscore` is already a token (`kestrel-lexer/src/lib.rs:170`).
- `_` isn't in `at_expr_start` (`kestrel-parser/src/grammar/exprs.rs:44`),
  so `f(_)` currently fails with E803 "expected expression".
- Closures with unannotated params, the implicit `it` parameter and all four
  closure kinds are fully supported.

## Pipeline impact

| Stage | Change |
|-------|--------|
| **Lexer** | None |
| **Parser / CST** | Two nodes: `ExprPlaceholder` (a hole) and `ExprPlaceholderHead` (a chain head). A parser field `arg_depth: u32`: `argument_list_body` increments it and `closure()` saves and zeroes it. In `argument()` (`exprs.rs:439`), after an optional label: at `Underscore` followed by `,` or `)`, emit `ExprPlaceholder`. In `primary()` (`exprs.rs:511`): at `Underscore` with `arg_depth > 0` and the next token **joined** and one of `.`, `?`, `(`, `!`, emit `ExprPlaceholderHead` and return it as the chain start, so the normal postfix loop continues. Otherwise `_` at expression start reports E811 instead of the generic E803. `argument_list_body`'s "expected expression" check accepts `Underscore`. Add both kinds to `kinds.txt`; regenerate |
| **Decls (ECS)** | None |
| **HIR** | `SugarKind::Placeholder`. (a) In `lower_call_args` (`kestrel-hir-lower/src/expr.rs:1400`): for each argument, count `ExprPlaceholderHead` nodes in its subtree, stopping at nested `ArgumentList`s and `ExprClosure`s. If there's one, push a scope, define `$p0`, lower the argument with the head mapped to `Local($p0)`, pop the scope, and wrap it as `Closure` + `Sugar`. If there are two or more, report E812. (b) In `lower_call` (`expr.rs:731`): if any argument is an `ExprPlaceholder`, define `$p0…`, lower the call with `Local($pᵢ)` in those slots (going through the existing `lower_member_call` / `lower_path_call` / direct paths unchanged), and wrap it. Factor (b) as `lower_call_with_holes(node, fill: impl FnMut(i) -> HirExprId)` so the pipe operator reuses it with a fill that returns the piped value |
| **Type infer** | None beyond closures. Anchor "could not infer" for a placeholder closure on the `_` span |
| **Captures / analyzers / MIR / codegen** | None. `Sugar` is transparent |
| **LSP** | Hover on `_` shows the parameter type. Completion after `_.` lists members of the expected parameter type (the same path as `it.`) |
| **Docs** | `docs/language/closures.md` (new "Placeholders" section), cross-link from `functions.md`, `kestrel-hir/docs/desugaring.md` |

## Error cases

Codes are proposals. Confirm they're still free when implementing (parser codes
are assigned only in `kestrel-parser/src/syntax_error.rs`; E810 was the last).

| Code | Where | Message |
|---|---|---|
| E811 | parser | `` `_` here isn't a placeholder `` + note "`_` is a placeholder only as a whole call argument (`f(_, 1)`) or at the start of a member chain inside an argument (`map(_.name)`); use `{ it … }` otherwise" |
| E812 | HIR lower | `` an argument can contain only one `_.` placeholder `` + note "write a closure: `{ (a) in a.x + a.y }`" |
| — | type infer | Existing "could not infer type" when there's no expectation |

## Edge cases

- **Nested calls.** In `f(g(_))` the hole binds to `g`. In `f(g(_.x))` the
  member placeholder binds to `g`'s argument. Both follow "nearest". When `f`
  expected a value, the mismatch error gets a note: "`_` binds to the
  innermost call".
- **Implicit `it` and `_` together.** `xs.map({ it + _.x })` is E811, because
  closures reset `arg_depth`. Inside a placeholder closure, `it` still refers to
  any enclosing closure's `it`.
- **`mutating` parameters.** `xs.forEach(_.reset())` with `reset` `mutating`
  needs a `mutating (T) -> ()` expectation, which the closure-kind rules
  already handle (E624/E625).
- **Keyword labels** (`f(in: _)`): `at_label` already accepts them.
- **Default parameters.** Trailing defaults are filled inside the body.
- **Generic callee.** `identity(_)` with no expectation: "could not infer".
- **`_` as a pattern inside an argument** (`f(match x { _ => 1 })`): patterns
  are parsed by the pattern grammar, never `primary()`, so they're unaffected.

## Test plan

Tests first, under `lib/kestrel-test-suite/testdata/expressions/placeholders/`:

- Execution, holes: single, multiple, labeled, method, static, initializer,
  subscript. Passing to `map`/`filter`. A per-call side-effect counter.
- Execution, member: `map(_.name)`, `filter(_.age >= 18)`, `sorted(by: _.age)`,
  `map(_?.city)`, `map(_(x))`, `map(_.name.count)`, mixed with a hole.
- Execution: capturing a non-Copyable local, `escaping` expectation.
- Diagnostics: E811 (`let x = _`, `f(_ + 1)`, `{ _.x }` in an argument),
  E812, no-context member placeholder, overloaded callee with no context,
  nested-call note.
- MIR snapshot: `xs.map(_.name)` lowers the same as `xs.map { it.name }`.

## Effort

About 3–5 days: parser with `arg_depth` (1 day), HIR hole and member lowering
(1–1.5 days), tests and diagnostics (1–2 days), docs.

## Open questions

- **Q1.** In `f(g(_))`, should a hole inside a nested call be an error
  ("ambiguous placeholder") instead of binding to `g`? This design binds to the
  nearest call, like Scala.
