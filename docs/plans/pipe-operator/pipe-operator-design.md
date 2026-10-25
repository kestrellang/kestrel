# Pipe Operator (`|>`) Design

> **Status: design, not started.** Milestone 0.19 ("Expression sugar").
> Base commit `def25459` (`future/0.18`). File and line anchors refer to that
> tree. Depends on [placeholder arguments](../placeholder-arguments/placeholder-arguments-design.md)
> for the `_` slot.

## Overview

`x |> f` passes `x` to `f`. It lets a chain of free-function calls read left
to right instead of inside out:

```kestrel
// Today
let report = render(summarize(parse(read(path))))

// With pipe
let report = path |> read |> parse |> summarize |> render

// With a slot
let n = text |> Int64.parse(_, radix: 16) |> clamp(_, min: 0, max: 255)
```

The operator is a purely syntactic rewrite into a call that already exists.
Nothing after HIR lowering knows `|>` was written.

The right side is **any expression that evaluates to something callable with
the left value**. The one special case is a call with a `_` slot, which takes
the value in that slot directly:

```kestrel
x |> f                 // f(x)
x |> makeAdder(1)      // makeAdder(1)(x): call the result
x |> f(_, 1)           // f(x, 1): fill the slot, no closure
```

### Non-goals

- **A `Pipeable` protocol / user overloading.** Every other binary operator
  desugars to a protocol call (`kestrel-hir/docs/desugaring.md`). Pipe is
  function application, not an operation on the left value, so it doesn't.
- **Method pipes** (`x |> .trim()`). That's `x.trim()`.
- **Function composition** (`f >> g`). Not in 0.19.

## Syntax

```
expr ::= expr '|>' expr        // lowest binary precedence, left-associative
```

| power | operators |
|------:|-----------|
| **5** | **`\|>`** (new, left-assoc) |
| 10 | `or` |
| 15 | `??` (right-assoc) |
| 20 | `and` |
| … | unchanged |

Lowest precedence means the whole left side is piped:

```kestrel
a + b |> f            // f(a + b)
x ?? d |> f           // f(x ?? d)
x |> f |> g           // g(f(x))
x |> f ?? d           // x |> (f ?? d), not (x |> f) ?? d
```

The last line is a precedence trap. Write `(x |> f) ?? d`. A note on the
resulting type error should suggest the parentheses.

Statements end with `;`, and the binary-operator loop (`binary_bp`) doesn't
check for newlines, so a line can start with `|>`:

```kestrel
let r = xs
    |> transform
    |> render;
```

## Semantics

### 1. Desugaring

`lhs |> rhs` lowers according to the shape of `rhs`:

| `rhs` is | Lowers to |
|---|---|
| A call with a `_` anywhere in its **top-level** arguments: `f(a, _)`, `f(_.name)`, `f(_ + 1)` | every such `_` is `lhs`: `f(a, lhs)`, `f(lhs.name)`, `f(lhs + 1)`. No closure |
| Same, with `_` used more than once: `f(_, _.id)` | `{ let $pipe = lhs; f($pipe, $pipe.id) }` for an rvalue `lhs`; a place `lhs` is substituted at each use (borrowed) |
| Anything else: a path, member path, a call with no `_` (`makeAdder(1)`), closure literal, parenthesized expression, `OverloadSet` | `Call { callee: rhs, args: [lhs] }`: `rhs` is evaluated, then called with `lhs` |

The result is wrapped in `HirExpr::Sugar { kind: SugarKind::Pipe }`, so
diagnostics anchor on the `|>` token and inner errors don't cascade.

The call-with-slot case reuses the placeholder helper
(`lower_call_with_holes`, see the placeholder design) with a fill that returns
the lowered `lhs` instead of a fresh closure parameter. The fill applies to
holes and to `_` inside top-level arguments alike. That gives every call form
(method, static, initializer, subscript) for free. **Nested** argument lists
keep the ordinary placeholder meaning: in `x |> f(g(_.y))`, `_.y` is a closure
passed to `g`.

### 2. Evaluation order and ownership

The lowered `lhs` is substituted **directly** into the argument slot, with no
`let` temporary. So:

- **Borrows and access modes behave as if written by hand.**
  `buf |> process(_)` borrows `buf` if `process` borrows. A temporary would
  move a non-Copyable `buf` and break that.
- **Evaluation follows argument order.** In `x() |> f(a(), _)`, `a()` runs
  **before** `x()`, because `f(a(), x())` evaluates left to right.

The second point is surprising when reading left to right. The alternative
binds `lhs` to a temporary first when it isn't a place. That keeps reading
order and only changes ownership for rvalues, which are owned anyway:

| `lhs` | Substitute directly | Temporary first |
|---|---|---|
| Place (`buf`, `self.items`) | ✓ borrow preserved | ✗ moves the place |
| Rvalue (`load()`) | order: after earlier args | order: first ✓; ownership: same (already owned) |

**Decided:** substitute places directly, and bind rvalues to a `$pipe`
temporary in a `Block` before the call. Both rules are local to HIR lowering
(a place is `Local` / `Field` / `TupleIndex` chains of those, the same check
as `is_place_syntax` in `kestrel-hir-lower/src/desugar.rs:26`). An rvalue used
more than once through `$pipe` must be Copyable or Cloneable; otherwise the
existing use-after-move error fires on the second use.

### 3. Types

There's no new inference. `Call` / `MethodCall` constraints do everything.
Calling a closure literal directly (`x |> { it * 2 }`) works through the same
`Call` path as `{ it * 2 }(x)`.

A call with no `_` is called **as a function value**. So `x |> f(a)` type-checks
only if `f(a)` returns something callable with `x`'s type. That's the curried
case (`x |> makeAdder(1)`). When it fails because `f(a)` isn't a function, or
doesn't take `x`'s type, the `Pipe` sugar turns the error into one with a
fix-it: "`f(a)` returns `R`, which can't be called with `X`; did you mean
`f(_, a)`?". Kestrel's labeled, method-first APIs make guessing "the first
argument" unreliable, which is why `|>` never inserts `x` implicitly.

## Current state

On `def25459`:

- `|` is `Pipe` (bitwise or, power 50). `|>` lexes as `Pipe` `Greater`,
  which is never valid, so the new token can't break existing code.
- Closures are written `{ (x) in … }`, never `|x| …`, so there's no
  closure-syntax conflict.

## Pipeline impact

| Stage | Change |
|-------|--------|
| **Lexer** | `token PipeGreater '\|>'` in `kestrel-syntax-tree/kinds.txt`; regenerate. Logos longest-match keeps `\|`, `\|=`, `\|>` distinct |
| **Parser / CST** | `binary_binding_power` (`exprs.rs:140`): `K::PipeGreater => 5`. Update the doc table above it. The node is an ordinary `ExprBinary` |
| **AST ops** | `BinaryOp::Pipe` in `kestrel-ast/src/ops.rs` with spelling `\|>`. It has **no** row in `lookup_binary_op` / `lookup_short_circuit_op` |
| **HIR** | `SugarKind::Pipe`. In the binary-op lowering path, intercept `BinaryOp::Pipe` **before** the operands are lowered generically, because the right side's CST shape decides the lowering (§1). Lower `lhs` once; then either `lower_call_with_holes(rhs, fill = lhs)`, or lower `rhs` as an expression and build `Call` |
| **Type infer / analyzers / MIR / codegen** | None |
| **LSP** | Signature help inside `f(…, _)` on the right of `\|>` shows the slot's parameter. Semantic tokens: operator |
| **Docs** | Operator table in `docs/language/syntax.md`, `kestrel-hir/docs/desugaring.md` (a "Not protocol-backed" note), `docs/language/functions.md` |

## Error cases

Codes are proposals. Confirm they're still free when implementing.

| Code | Where | Message |
|---|---|---|
| — | type infer | Existing "not callable" / argument mismatch, anchored by the `Pipe` sugar on the `\|>`, with the "did you mean `f(_, a)`?" note when `rhs` is a call with no `_` (§3) |

## Edge cases

- **Line starts with `|>`.** Works today's way for every binary operator
  (`binary_bp` ignores newlines). A parser test pins it so a future
  newline-sensitive change doesn't break the style.
- **Trailing closure on the right.** `xs |> forEach(_) { print(it) }`: the
  trailing closure joins the call's argument list as usual and `_` is still the
  only hole.
- **`try`.** `try` takes a postfix operand (`operand()` in `exprs.rs`), so
  `try x |> f` parses as `(try x) |> f`. To try the result, write
  `try (x |> f)`. `x |> try f(_)` is an error, because `try f(_)` isn't a call.
- **Generic callee.** `x |> identity` with `identity[T]`: inferred from `x`
  like any call.
- **Overloads by label.** `x |> f` only works when `f`'s first parameter is
  unlabeled (Kestrel requires labels at call sites). With a labeled first
  parameter, use `x |> f(label: _)`. The type error should suggest it.

## Test plan

Tests first, under `lib/kestrel-test-suite/testdata/expressions/pipe/`:

- Execution: bare function, chain of three, slot first / middle / last,
  labeled slot, method / static / initializer / subscript targets, closure
  literal target, precedence against `+`, `??`, `or`.
- Execution: a non-Copyable place piped into a borrowing function is still
  usable afterwards. An rvalue left side runs before the other arguments
  (`x() |> f(a(), _)` calls `x()` first).
- Execution: `x |> f(_.name)`, `x |> f(_ + 1)`, `x |> f(_, _.id)` (one
  evaluation of an rvalue `x`), and `x |> f(g(_.y))` (nested stays a closure).
- Execution: `x |> makeAdder(1)` (a call with no `_` is called with `x`).
- Diagnostics: the "did you mean `f(_, a)`?" note on `x |> f(a)`,
  labeled-first-param suggestion, non-callable rhs, `x |> f ?? d` precedence
  note.
- Parser: a line starting with `|>` continues the expression.

## Effort

About 1–2 days once placeholder arguments are in: token and precedence (an
hour), HIR lowering (half a day), tests and diagnostics (1 day).

## Decisions

- **Rvalue left sides are evaluated first** (2026-10-25). Places are
  substituted directly, and rvalues go through a `$pipe` temporary (§2).
- **Every `_` in the right-hand call's top-level arguments is the piped
  value** (2026-10-25). `x |> f(_.name)` is `f(x.name)`, and `x |> f(_, _.id)`
  is `f(x, x.id)`. Nested argument lists keep the placeholder meaning. This
  removed the "more than one `_`" error.
- **Trailing closures on the right keep the general rule** (per the design's
  recommendation, not separately reviewed). `xs |> map { it * 2 }` is
  `map({ … })(xs)`. A note suggests `xs.map { … }` when `map` is a method on the
  piped type.
