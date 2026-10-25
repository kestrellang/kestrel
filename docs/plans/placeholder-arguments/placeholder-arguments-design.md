# Placeholder Arguments (`f(_, 1)`, `map(_.name)`, `map(_ * 2)`) Design

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

// 2. Argument placeholder: `_` anywhere inside a larger argument makes that
//    ARGUMENT a closure. Every `_` in it is the same value.
let names = users.map(_.name)             // users.map({ (u) in u.name })
let adults = users.filter(_.age >= 18)    // users.filter({ (u) in u.age >= 18 })
let doubled = xs.map(_ * 2)               // xs.map({ (x) in x * 2 })
let full = users.map(_.first + " " + _.last)
let byAge = users.sorted(by: _.age)
let cities = users.map(_.address?.city)
```

| Form | Spelled | Closure covers | Params |
|---|---|---|---|
| **Hole** | `_` as the whole argument | the **call** | one per hole, left to right |
| **Argument placeholder** | `_` anywhere inside an argument that is more than just `_` | that **argument** | exactly one; every `_` in the argument is that parameter |

Both are pure sugar for a closure literal. Nothing after HIR lowering knows `_`
was written: inference, captures, closure kinds and MIR see a closure.

Holes are also the slot that the [pipe operator](../pipe-operator/pipe-operator-design.md)
fills: `x |> f(a, _)` substitutes `x` directly and builds no closure.

### Non-goals

- **Multi-parameter sections** (Scala's `_ + _` meaning `{ (a, b) in a + b }`).
  Here every `_` in one argument is the same parameter, so `_ + _` is
  `{ (x) in x + x }`. Write `reduce(0, { (a, b) in a + b })` or pass `add`.
- **`_` outside call arguments** (`let f = _.name`). There's no expected type
  to give the parameter a type, and `{ it.name }` works.
- **Placeholders in type position.** `_` in types already means "infer". That's
  unchanged.

## Syntax

```
argument     ::= (label ':')? (hole | expr)
hole         ::= '_'                         // followed by ',' or ')'
primary      ::= … | placeholder             // only inside an argument (see below)
placeholder  ::= '_'                         // any operand position: `_.x`, `_ * 2`, `-_`, `f(_)` inside a larger argument
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
| `f(g(_.x))` | `f(g({ ($p0) in $p0.x }))`: a argument placeholder binds to the nearest argument |
| `xs.map(_ * 2)` | `xs.map({ ($p0) in $p0 * 2 })` |
| `xs.filter(_ > 0 and _ < 10)` | `xs.filter({ ($p0) in $p0 > 0 and $p0 < 10 })` (one parameter) |
| `xs.map(-_)` | `xs.map({ ($p0) in -$p0 })` |
| `xs.map(_.a + _.b)` | `xs.map({ ($p0) in $p0.a + $p0.b })` |
| `xs.map(_ + _)` | `xs.map({ ($p0) in $p0 + $p0 })`, not a two-parameter sum |
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

### 0. Boundaries: what a `_` closes over

The boundary is purely syntactic. Every `(` of an argument list opens a new
scope for `_`, and every closure literal closes all scopes.

| `_` form | Closes over | Example | Means |
|---|---|---|---|
| Hole (`_` is the whole argument) | the **nearest enclosing call**: callee, receiver and every other argument | `obj.m(_, g())` | `{ p in obj.m(p, g()) }` |
| `_` inside a larger argument (`_.x`, `_ * 2`, `-_`, `_?.x`, `_(…)`) | the **whole nearest enclosing argument value** (not its label), operators included | `filter(_.age >= 18)` | `filter({ p in p.age >= 18 })` |
| Several `_` in one argument | still the whole argument, **one** parameter | `map(_.first + " " + _.last)` | `map({ p in p.first + " " + p.last })` |
| Argument placeholder in a nested call | that inner call's argument | `xs.map(f(_.x))` | `xs.map(f({ p in p.x }))` (see Decisions) |
| Hole in a nested call | that inner call | `f(g(_))` | `f({ p in g(p) })` |

What **does not** reset the scope (so the closure covers it): operators,
parentheses, `if` / `match` expressions, string interpolation, and array,
dictionary or tuple literals inside the argument.
`f("\(_.name)")` is `f({ p in "\(p.name)" })`, and `f([_.a, 1])` is
`f({ p in [p.a, 1] })`.

What **does** reset it: a closure literal, including trailing closures.
`f({ _.x })` and `f { _.x }` are E811, and the fix is `it`.

**Operators never create a boundary.** An argument placeholder always covers
the whole argument, so precedence doesn't matter: `f(_ * 2 + 1)` and
`f(1 + _ * 2)` both close over the full argument. That includes the
short-circuit operators (`and`, `or`, `??`). They desugar their right side
into a closure internally, but that's not a closure *literal*, so it doesn't
reset the scope: `f(_.a ?? _.b)` is `f({ p in p.a ?? p.b })`.

Where `_` is never a placeholder: outside any argument list (`let g = _.x`,
statement `_.f()`), and in patterns and types. The first is E811. Patterns and
types keep their meaning.

Inside the top-level arguments of a pipe's right-hand call, `_` is the piped
value instead (pipe design, Q2).

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

### 2. Desugaring argument placeholders

An argument (other than a bare hole) whose expression contains one or more
`ExprPlaceholderUse` nodes, not counting any inside an inner argument list or a
closure literal, lowers to the following, with **every** use mapped to the
same `$p0`:

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
  what makes argument placeholders work, since `_.name` needs to know `_`'s type.
  It's the same mechanism as `xs.map { it.name }`.
- With no expectation, hole parameters get their types from the call
  (`let g = add(_, 1)`). A argument placeholder with no expectation is the
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
| **Parser / CST** | Two nodes: `ExprPlaceholder` (a hole) and `ExprPlaceholderUse` (a `_` inside a larger argument). A parser field `arg_depth: u32`: `argument_list_body` increments it and `closure()` saves and zeroes it. In `argument()` (`exprs.rs:439`), after an optional label: at `Underscore` followed by `,` or `)`, emit `ExprPlaceholder`. In `primary()` (`exprs.rs:511`): at `Underscore` with `arg_depth > 0`, emit `ExprPlaceholderUse` and return it as a chain start, so postfix and binary operators continue normally. With `arg_depth == 0`, report E811 instead of the generic E803. Also add `Underscore` to `at_expr_start` behind the same `arg_depth > 0` check, so `-_` and `not _` parse. `argument_list_body`'s "expected expression" check accepts `Underscore`. Add both kinds to `kinds.txt`; regenerate |
| **Decls (ECS)** | None |
| **HIR** | `SugarKind::Placeholder`. (a) In `lower_call_args` (`kestrel-hir-lower/src/expr.rs:1400`): for each argument, look for `ExprPlaceholderUse` nodes in its subtree, stopping at nested `ArgumentList`s and `ExprClosure`s. If there are any, push a scope, define `$p0`, lower the argument with every use mapped to `Local($p0)`, pop the scope, and wrap it as `Closure` + `Sugar`. (b) In `lower_call` (`expr.rs:731`): if any argument is an `ExprPlaceholder`, define `$p0…`, lower the call with `Local($pᵢ)` in those slots (going through the existing `lower_member_call` / `lower_path_call` / direct paths unchanged), and wrap it. Factor (b) as `lower_call_with_holes(node, fill: impl FnMut(i) -> HirExprId)` so the pipe operator reuses it with a fill that returns the piped value |
| **Type infer** | None beyond closures. Anchor "could not infer" for a placeholder closure on the `_` span |
| **Captures / analyzers / MIR / codegen** | None. `Sugar` is transparent |
| **LSP** | Hover on `_` shows the parameter type. Completion after `_.` lists members of the expected parameter type (the same path as `it.`) |
| **Docs** | `docs/language/closures.md` (new "Placeholders" section), cross-link from `functions.md`, `kestrel-hir/docs/desugaring.md` |

## Error cases

Codes are proposals. Confirm they're still free when implementing (parser codes
are assigned only in `kestrel-parser/src/syntax_error.rs`; E810 was the last).

| Code | Where | Message |
|---|---|---|
| E811 | parser | `` `_` here isn't a placeholder `` + note "`_` is a placeholder only inside a call's arguments (`f(_, 1)`, `map(_.name)`, `map(_ * 2)`); use `{ it … }` otherwise" |
| — | type infer | Existing "could not infer type" when there's no expectation |

## Edge cases

- **Nested calls.** In `f(g(_))` the hole binds to `g`. In `f(g(_.x))` the
  argument placeholder binds to `g`'s argument. Both follow "nearest". When `f`
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
- Execution, argument placeholders: `map(_.name)`, `filter(_.age >= 18)`,
  `sorted(by: _.age)`, `map(_?.city)`, `map(_(x))`, `map(_.name.count)`,
  `map(_ * 2)`, `map(-_)`, `filter(_ > 0 and _ < 10)` (one parameter),
  `map(_.first + " " + _.last)`, `map(_ + _)` (doubles), `f("\(_.name)")`,
  `map(_.a ?? _.b)`, mixed with a hole.
- Execution: capturing a non-Copyable local, `escaping` expectation.
- Diagnostics: E811 (`let x = _`, `{ _.x }` in an argument), no-context
  argument placeholder, overloaded callee with no context,
  nested-call note.
- MIR snapshot: `xs.map(_.name)` lowers the same as `xs.map { it.name }`.

## Effort

About 3–5 days: parser with `arg_depth` (1 day), HIR hole and member lowering
(1–1.5 days), tests and diagnostics (1–2 days), docs.

## Open questions

- **Q1.** In `f(g(_))`, should a hole inside a nested call be an error
  ("ambiguous placeholder") instead of binding to `g`? This design binds to the
  nearest call, like Scala.
- **Q3. Pipe interaction.** See the pipe design, Q2: inside a pipe's right
  side, `_` is the piped value, not a placeholder.

## Decisions

- **Every `_` in one argument is the same parameter** (2026-10-25). `_ + _` is
  `{ x in x + x }`. Multi-parameter sections aren't supported.
- **Argument placeholders bind to the nearest argument** (2026-10-25).
  `xs.map(f(_.x))` is `xs.map(f({ p in p.x }))`. When the inner parameter isn't
  a function type, the type error carries the note "`_` binds to the innermost
  argument; write `{ f(it.x) }`". Rejected: the outermost argument (breaks
  nested placeholders like `groups.map(_.members.filter(_.active))`), and the
  nearest argument that expects a function (needs types during HIR lowering,
  and meaning shifts when signatures change).
- **`_` passes through string interpolation, `if`/`match`, literals and all
  operators** (2026-10-25). Only argument lists and closure literals are
  boundaries.
