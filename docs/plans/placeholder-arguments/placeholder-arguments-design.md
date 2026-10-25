# Placeholder Arguments (`f(_, 1)`) Design

> **Status: design, not started.** Milestone 0.19 ("Expression sugar").
> Base commit `def25459` (`future/0.18`). File and line anchors refer to that
> tree. Companion designs: [pipe operator](../pipe-operator/pipe-operator-design.md),
> [optional chaining](../optional-chaining/optional-chaining-design.md).

## Overview

A placeholder argument turns a call into a function value with the
placeholder positions left open:

```kestrel
let addOne = add(_, 1)              // { (a) in add(a, 1) }
let parseHex = Int64.parse(_, radix: 16)
xs.map(clamp(_, min: 0, max: 255))
```

Each `_` in a call's argument list becomes one parameter of a synthesized
closure, in left-to-right order. Everything else in the call stays exactly as
written. Nothing after HIR lowering knows that `_` was written. The feature
is pure sugar for a closure literal, and inference, captures, closure kinds and
MIR treat it as one.

Placeholders are also the hole that the [pipe operator](../pipe-operator/pipe-operator-design.md)
fills: `x |> f(a, _)` substitutes `x` directly into the slot and builds no
closure.

### Non-goals

- **`_.member` / `_.method()`** (Scala's `users.map(_.name)`). Closures already
  have an implicit `it` parameter (`users.map { it.name }`, see
  `lower_closure` in `kestrel-hir-lower/src/expr.rs:1607`), which covers this.
- **Operator sections** (`_ + 1`, `(_ < limit)`). `{ it + 1 }` covers them.
- **Placeholders for the callee or receiver** (`_(x)`, `_.f()`).
- **Placeholders in type position.** `_` in types already means "infer"; that's
  unchanged.

## Syntax

```
argument ::= (label ':')? (expr | '_')
```

A bare `_` is accepted **only as the whole value of a call argument**. That
includes plain calls, method calls, static calls, initializer calls and
subscript calls (subscripts use call syntax, so `table(_)` is
`{ (i) in table(i) }`).

| Written | Means |
|---|---|
| `f(_)` | `{ ($p0) in f($p0) }` |
| `f(_, 1)` | `{ ($p0) in f($p0, 1) }` |
| `f(_, _)` | `{ ($p0, $p1) in f($p0, $p1) }` |
| `f(x: _, y: 2)` | `{ ($p0) in f(x: $p0, y: 2) }` |
| `obj.m(_)` | `{ ($p0) in obj.m($p0) }` |
| `Point(x: _, y: _)` | `{ ($p0, $p1) in Point(x: $p0, y: $p1) }` |
| `f(g(_))` | `f({ ($p0) in g($p0) })` (binds to the nearest call) |
| `f(_ + 1)` | error E811 (`_` is not the whole argument) |
| `let y = _` | error E811 |

Trailing closures are unaffected. `f(_) { body }` is
`{ ($p0) in f($p0) { body } }`.

The closure's type is unlabeled, `(A, B) -> R`, like every Kestrel function
type. Labels stay on the inner call.

## Semantics

### 1. Desugaring

`f(a₁, …, aₙ)` where the arguments at positions `i₁ < … < iₖ` are `_`
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

The `$p` names are synthetic and can't collide with user names (the `$`
prefix is the existing convention: `$try_value`, `$cast`, `$iter`).

### 2. Evaluation of the other arguments

The non-placeholder arguments are **inside the closure body**, so they are
evaluated **on every call**, not when the closure is created:

```kestrel
let f = log(_, timestamp())   // timestamp() runs each time f is called
```

This is the same rule as Scala. It falls out of the desugaring with no extra
machinery. The alternative (evaluate once, when the closure is created) needs
temporaries that outlive the expression, and normal closures capture **views
of the frame** (`docs/design/closures.md`). That's awkward for temporaries,
and `let` + `it` already expresses it:

```kestrel
let ts = timestamp()
let f = { log(it, ts) }
```

*Open question Q1 asks to confirm this.*

### 3. Types and closure kind

The synthesized closure goes through `gen_closure`
(`kestrel-type-infer/src/generate.rs:915`) like any closure literal:

- It is inserted into `closure_literal_exprs`, so the parameter-convention
  inference (borrow, `MutBorrow` upgrade) applies to it.
- With an expected function type (`xs.map(add(_, 1))`), parameter types and
  the closure kind come from the expectation. `escaping` / `consuming`
  expectations make it an owning closure, and capture rules apply as usual.
- With no expectation (`let g = add(_, 1)`), parameter types come from the
  call. The closure is a normal (frame-bound) closure.
- If the callee is an overload set and there's no expectation
  (`let g = print(_)`), inference may not be able to pick an overload. That
  reports the existing "could not infer type" error, anchored on the `_`.

### 4. Captures

The non-placeholder arguments and the callee are free variables of the
closure body. `ClosureCaptures` (`kestrel-type-infer/src/captures.rs`)
captures them by place, exactly as for a hand-written closure. Nothing new.

### 5. Interaction with pipe

When a call containing `_` is the right side of `|>`, the pipe lowering
substitutes the left side into the slot and **does not** build a closure. The
shared helper is described under [Pipeline impact](#pipeline-impact).

## Current state

On `def25459`:

- `Underscore` is already a token (`kestrel-lexer/src/lib.rs:170`).
- `_` is not in `at_expr_start` (`kestrel-parser/src/grammar/exprs.rs:44`), so
  `f(_)` currently fails with E803 "expected expression".
- Closures with unannotated params and the implicit `it` parameter are fully
  supported, as are all four closure kinds.

## Pipeline impact

| Stage | Change |
|-------|--------|
| **Lexer** | None |
| **Parser / CST** | `argument()` (`exprs.rs:439`): after an optional label, if at `Underscore` *and* the next token is `,` or `)`, emit an `ExprPlaceholder` node wrapping the token. Otherwise fall through to `expr()`. `_` stays out of `at_expr_start`, so every other position keeps failing. Add `ExprPlaceholder` to `kinds.txt` and regenerate. `argument_list_body`'s "expected expression" check must accept `Underscore` |
| **Decls (ECS)** | None |
| **HIR** | `SugarKind::Placeholder`. In `lower_call` (`kestrel-hir-lower/src/expr.rs:731`), before lowering args: if any `ArgSyntax` is a placeholder, push a scope, define `$p0…`, lower the call with `HirExpr::Local($pᵢ)` in those slots (going through the existing `lower_member_call` / `lower_path_call` / direct paths unchanged), pop the scope, and wrap the result as `Closure` + `Sugar`. Factor this as `lower_call_with_holes(node, fill: impl FnMut(i) -> HirExprId)` so the pipe operator can reuse it with a fill that returns the piped value. An `ExprPlaceholder` reached any other way (it can't, by grammar, but be defensive) lowers to `HirExpr::Error` with E811 |
| **Type infer** | None beyond what closures already do. Anchor "could not infer" for a placeholder closure on the first `_` span |
| **Captures / analyzers** | None. `Sugar` is transparent to every consumer |
| **MIR / codegen** | None |
| **LSP** | Hover on `_` shows the parameter type; signature help works unchanged since the inner call is a normal call |
| **Docs** | `docs/language/closures.md` (new "Placeholder arguments" section), `docs/language/functions.md` cross-link, `kestrel-hir/docs/desugaring.md` |

## Error cases

Codes are proposals. Confirm they're still free in
`lib/kestrel-analyze/AGENTS.md` when implementing, since other work
allocates concurrently.

| Code | Where | Message |
|---|---|---|
| E811 | parser | `` `_` placeholder must be a whole call argument `` (for `_` in any other expression position, including `f(_ + 1)`). Note: "use `{ it + 1 }` for an expression" |
| — | type infer | Existing "could not infer type" when the placeholder closure has no expectation and an overloaded callee |

## Edge cases

- **Nested calls.** `f(g(_))` binds to `g`, giving `f({ (p) in g(p) })`. This
  is almost never what's wanted when `f` expects a value, and the error then
  is an ordinary type mismatch. Consider a note on that mismatch: "`_` binds to
  the innermost call".
- **Placeholder inside a closure.** `{ f(_) }` is a closure returning a
  closure. That's legal.
- **`mutating` parameters.** `swap(_, &mutating b)` → the closure body
  contains `&mutating b`, which makes it a `mutating`-kind capture. The usual
  closure-kind rules apply (E624/E625).
- **Labels that are keywords** (`f(in: _)`): `at_label` already accepts keyword
  labels.
- **Default parameters.** `f(_)` where `f` has trailing defaulted params:
  defaults are filled inside the body as usual.
- **Generic callee.** `identity(_)` with no expectation leaves `T` unresolved:
  that's the existing "could not infer" error.

## Test plan

Write tests first under `lib/kestrel-test-suite/testdata/expressions/placeholders/`:

- Execution: single, multiple, labeled, method, static, initializer and
  subscript placeholders. Passing to `map`/`filter`. A per-call side-effect
  counter that proves the other arguments run once per call.
- Execution: captures of a non-Copyable local through a placeholder closure
  (frame view). `escaping` expectation (owned capture).
- Diagnostics: E811 for `let x = _`, `f(_ + 1)`, `return _`, and `_` as a
  callee. Overloaded callee with no context. `f(g(_))` mismatch.
- MIR snapshot: one placeholder call lowers to the same MIR as the
  hand-written closure.

## Effort

About 2–4 days: parser (half a day), HIR helper and sugar kind (1 day), tests
and diagnostics (1–2 days), docs.

## Open questions

- **Q1.** Confirm the other arguments run on every call (§2).
- **Q2.** Should `f(g(_))` be an error ("ambiguous placeholder") instead of
  binding to `g`? This design says no: the rule is simple and Scala users
  know it.
