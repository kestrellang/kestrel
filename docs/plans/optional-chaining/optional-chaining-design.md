# Optional Chaining (`a?.b`) Design

> **Status: design, not started.** Milestone 0.19 ("Expression sugar").
> Base commit `def25459` (`future/0.18`). File and line anchors refer to that
> tree. Companion designs: [placeholder arguments](../placeholder-arguments/placeholder-arguments-design.md),
> [pipe operator](../pipe-operator/pipe-operator-design.md).

## Overview

Optional chaining reaches through an optional-like value without unwrapping it
by hand:

```kestrel
let theme = user?.profile?.settings.theme ?? Theme.default
let n = cache?.lookup(key)?.count         // Int64?
inventory?.items.push(item)               // mutates in place; type ()?
player?.score += 10
let r = callback?.(event)                 // call an optional function
```

`a?.rest` evaluates `a` once and asks it for its payload. If there's none,
**the rest of the postfix chain is skipped** and the whole expression is
`.None`. Otherwise the chain continues on the payload, and its result is
wrapped in `Optional`. If the result is already an `Optional`, it's
**flattened** instead: `a?.b` with `b: Int64?` is `Int64?`, not `Int64??`.

Like every other Kestrel operator, `?.` is **backed by a protocol**,
`OptionalChain`. `Optional` conforms to it in the stdlib. Other types can
conform too: a future `@weak` reference (0.22), a lazily loaded handle, or a
user's own `Maybe`-like type.

The chain's result is **always `Optional`**, whatever the base type is. That's
the same rule Swift uses. A "same container, new payload" result, where
`res?.x` stays a `Result`, would need higher-kinded types. Users who want that
use `map`.

### Non-goals

- **Container-preserving chains** (see above).
- **Optional pattern sugar** (`case let x?`).

## Syntax

```
postfix_op ::= … | '?.' member | '?.' '(' args ')'
```

`?.` is two tokens, `Question` then `Dot`, **joined** with no whitespace
between them (`p.joined_at`, `kestrel-parser/src/core.rs:214`). There's no new
lexer token. `?` is unused in expression position today: it only appears in
types (`Int?`) and in `func f() ?`. Two joined tokens avoid any lexer
interaction with `as?` and `??`.

| Written | Means |
|---|---|
| `a?.b` | payload's `b`, wrapped or flattened |
| `a?.b.c()` | whole tail runs on the payload, or is skipped |
| `a?.b?.c` | the second bind runs only if the first succeeded |
| `a?.items(0)` | subscript (call syntax) on the payload's `items` |
| `f?.(x)` | call the payload: an optional function, or an optional value with a subscript |
| `a?.b = v` | assign through the chain; type `()?` |
| `a?.n += 1` | compound-assign through the chain; type `()?` |
| `a?.push(x)` | `mutating` method on the payload, in place |
| `(a?.b).c` | parentheses end the chain: `.c` applies to the `Optional` |
| `a?.b ?? d` | `(a?.b) ?? d`: `??` is a binary operator, the chain is postfix |
| `a ? .b` | not a chain (`?` must be joined): parse error |

### Where a chain ends

The chain is the **maximal postfix chain** containing the `?.`: member
accesses, tuple indexes, calls, trailing closures, `!`, `..`, and line-continued
`.member`s. It ends at any binary operator, `as?`/`is`, a closing delimiter,
or the end of the expression. For assignment, the chain is the left side,
and the right side is evaluated **only if** the chain reaches its target
(Swift's rule): `a?.b = expensive()` doesn't call `expensive()` when `a` is
`.None`.

## The protocol

```kestrel
// lang/std/core/optional_chain.ks
/// Backs the `?.` operator. A conforming type can hand out its payload
/// three ways; the compiler picks one per chain from what the chain does.
@builtin(.OptionalChainProtocol)
public protocol OptionalChain {
    /// The type the chain continues on.
    type Wrapped

    /// Read-only chains: `a?.count`, `a?.describe()`.
    @builtin(.OptionalChainBorrowMethod)
    func chainBorrow() -> Optional[&Wrapped]

    /// Chains that mutate the payload: `a?.push(x)`, `a?.n = 1`.
    @builtin(.OptionalChainMutateMethod)
    mutating func chainMutate() -> Optional[&mutating Wrapped]

    /// Chains on a temporary: `load()?.consume()`.
    @builtin(.OptionalChainTakeMethod)
    consuming func chainTake() -> Optional[Wrapped]
}

extend Optional[T]: OptionalChain {
    type Wrapped = T
    func chainBorrow() -> Optional[&T] { self.asRef() }
    mutating func chainMutate() -> Optional[&mutating T] { … }   // match self { .Some(&mutating v) => .Some(v), .None => .None }
    consuming func chainTake() -> Optional[T] { self }
}
```

These are three requirements, not one generic one, because Kestrel's access
modes are part of the method signature. The returns use `Optional[&T]` /
`Optional[&mutating T]`, which already exist in the stdlib
(`Optional.asRef`, `lang/std/result/optional.ks:126`, and
`PointerIterator.next() -> Optional[&mutating T]`, `lang/std/memory/pointer.ks:615`).

`Result[T, E]` **also conforms**, chaining on the `Ok` value
(`Wrapped = T`). Since the chain's result is always `Optional`, an `.Err`
becomes `.None` and **the error is dropped**: `parse(s)?.count` is `Int64?`.
Users who need the error write `r.map { … }` or `match`.

```kestrel
extend Result[T, E]: OptionalChain {
    type Wrapped = T
    func chainBorrow() -> Optional[&T] { self.okRef() }       // result.ks:135
    mutating func chainMutate() -> Optional[&mutating T] { … } // match self { .Ok(&mutating v) => .Some(v), .Err(_) => .None }
    consuming func chainTake() -> Optional[T] { self.ok() }   // result.ks:368
}
```

## Semantics

### 1. Desugaring

The parser marks the chain (§Pipeline). HIR lowering turns each `?.` bind into
a nested match on a protocol call. For binds `b₁ … bₙ` in evaluation order,
with `bᵢ` on base `sᵢ`:

```
Sugar(OptionalChain,
  Match(source: OptionalChain, scrutinee: ProtocolCall(s₁, OptionalChain, <mode₁>), arms: [
    .Some($o₁) => Match(OptionalChain, ProtocolCall(s₂[$o₁], OptionalChain, <mode₂>), [
        .Some($o₂) => … tail[$oₙ] …,
        .None => .None ]),
    .None => .None ]))
```

- `$oᵢ` is an ordinary owned binding of the method's result: a `&Wrapped`, a
  `&mutating Wrapped`, or a `Wrapped`. No `&` binder patterns are needed,
  because the protocol method already produced the reference.
- `sᵢ₊₁` is lowered with `Local($oᵢ)` in place of the bound sub-expression,
  so it's only evaluated inside the `.Some` arm.
- `<modeᵢ>` is the method. HIR lowering emits `chainBorrow` as a
  provisional name, and inference settles it (§2).

Lowering is bottom-up, like the CST. Lowering `ExprOptionalChain` pushes a
frame on a lowering-context stack. Lowering each `ExprOptionalBind(inner)`
lowers `inner`, defines a fresh local `$o`, records `(inner, $o)` in the frame
and returns `Local($o)`. When the chain's tail is lowered, the frame is popped
and the matches are built from the last bind outwards.

**Assignment.** `a?.b = v` lowers the left side as a chain whose innermost
arm body is `Assign { target: $oₙ.b, value: v }`. `v` is lowered inside that
arm, so it only runs on success. Compound assignment rewrites the same way
(`SugarKind::CompoundAssign` inside the arm). The place check for assignment
targets accepts an `ExprOptionalChain` whose tail is a place.

### 2. Choosing the mode (the hard part)

Which method a bind calls depends on what the chain does with the payload,
and that's only known **after member resolution**: `a?.push(x)` mutates only
because `push` resolves to a `mutating` method.

| The chain after bind `bᵢ` … | and the base `sᵢ` is | Mode |
|---|---|---|
| Only reads (fields, borrowing methods, subscript `get`) | anything | `chainBorrow` |
| Calls a `mutating` member, is an assignment target, or passes a `&mutating` / `mutating` argument rooted at `$oᵢ` | a `var` place | `chainMutate` |
| Same | a `let` place | `chainMutate`, then the existing immutability error (E200 family) at the base |
| Calls a `consuming` member on `$oᵢ`, or moves it | an rvalue (call result, literal) | `chainTake` |
| Same | a place | E158 "can't consume through `?.` on a place"; suggest `.take()?.` |

Inference records the decision per bind in `TypedBody` as
`chain_modes: HashMap<HirExprId, ChainMode>`, keyed by the scrutinee
`ProtocolCall`. The decision is made when the member constraints in the arm
resolve. That's the same point where a method call's receiver access mode is
known today. The binding's type follows the mode (`&W` / `&mutating W` /
`W`). Since mutability of a reference is analyze's job, not unify's, the
provisional `&W` typing doesn't block resolution.

`ChainMode` is the **single source of truth**. MIR lowering calls the method
it names, and `access_mode` checks exclusivity and mutability against it.

Nested binds decide independently. In `a?.b?.push(x)`, the inner bind mutates,
so the outer bind must also mutate: the mutation propagates **outward**. An
outer bind takes the strongest mode of anything rooted at its `$o`, including
inner binds.

### 3. Typing and flattening

The match's result is checked against `Optional[α]` (fresh `α`). If there's an
outer expectation (`let x: Int64? = a?.b`), it's unified with `Optional[α]`
first. In `generate.rs`'s `HirExpr::Match` arm (`generate.rs:741`) that's the
existing **check mode** path. Each arm body goes through `check_leaf`, which
emits `Coerce(body → Optional[α])`. `Coerce` already tries `unify` first and
falls back to `FromValue` promotion:

| Tail type | `Coerce(tail → Optional[α])` | Result |
|---|---|---|
| `U` (not Optional) | unify fails → promote via `FromValue` | `Optional[U]` |
| `Optional[U]` | unify succeeds, `α = U` | `Optional[U]` (flattened) |
| Generic `T` (rigid) | unify fails → promote | `Optional[T]`, even after mono when `T = Optional[X]` (the generic body decided) |
| `()` (mutation, assignment) | promote | `Optional[()]` |
| `Never` | `Never` arm doesn't pin | `Optional[α]`, `α` from context or defaulted |

**One required change.** `check_leaf` defers the coerce only for leaves whose
own constraint produces their type (`mark_synth_pending`, `generate.rs:149`).
That covers `Call`, `MethodCall` and `ProtocolCall`. A chain tail that's a
**field access** (`a?.name`) is a `Field`, so its coerce would unify the
still-unresolved field type with `Optional[α]` before member resolution.
`name: String` then collides with `Optional`. Fix: `mark_synth_pending` also
covers `Field` and `TupleIndex`. That's a general bidi fix:
`let x: Int64? = s.count` has the same latent order dependence.

### 4. Values out of a borrowed payload

In `chainBorrow` / `chainMutate` mode the tail reads through a reference. The
arm value then **decays** like any match-arm value
(`docs/language/references.md`, "Decay"):

- `Copyable` → copied.
- `Cloneable` → cloned (`user?.name` with `name: String`).
- Neither → E503 "move out of a borrow". The user matches by hand or uses
  `chainTake` on an owned value.

### 5. Statement position

`log?.write(line);` has type `Optional[()]` and is discarded. There's no
unused-value analyzer on the base commit, so nothing warns. A future one
should exempt `Optional[()]`.

### 6. Interactions

- **`??`:** `a?.b ?? d` works as expected.
- **`try`:** `try a?.b()` where `b` throws gives `try` an
  `Optional[Result[…]]`, and `try` unwraps the Optional. That's legal but
  surprising, so it's documented. There's no lint in 0.19.
- **`!`:** `a?.b!` force-unwraps `b` inside the chain. `(a?.b)!` unwraps the
  result.
- **Placeholders:** `xs.map(_?.name)` is a placeholder closure whose body is
  a chain (see the placeholder design).
- **Pipe:** `a?.b |> f` pipes the chain's `Optional`, since `|>` ends the chain.
- **Closures:** a chain inside a closure follows the closure's kind rules. A
  mutating chain on a captured `var` makes it a `mutating`-kind capture.
- **Exclusivity:** `a?.push(a.count)` has a `&mutating` bind on `a` live while
  `a.count` reads `a`. That's the existing access-mode analyzer's error, as for
  `a.push(a.count)` today.

## Current state

On `def25459`:

- `Question` (`?`) is a token used only in types and after `func f()`.
- `Optional[&T]` / `Optional[&mutating T]` returns exist, as do arm-value
  decay and E503.
- `FromValue` promotion through `Coerce` exists, and check-mode match arms use
  `check_leaf`.
- Operators already desugar to builtin protocols resolved through
  `resolve_builtin` (`kestrel-hir/src/builtin.rs`).

## Pipeline impact

| Stage | Change |
|-------|--------|
| **Lexer** | None (two existing tokens, joined) |
| **Parser / CST** | `ExprOptionalBind` node: wraps the operand and holds `?`. `ExprOptionalChain` node: wraps the whole chain. In `tight_ops` (`exprs.rs:354`): on `Question` joined to a following `Dot`, close the chain, wrap it in `ExprOptionalBind`, set a "saw bind" flag, then continue with `member()` or, if `(` follows the `.`, `call()`. In `postfix()` (`exprs.rs:330`): include the line-continued `.member` loop. When closing a chain whose flag is set, wrap it in `ExprOptionalChain`. Same in the `cond` path (`if let x = a?.b`). The assignment statement parser accepts an `ExprOptionalChain` LHS. Add both kinds to `kinds.txt`; regenerate |
| **Stdlib** | `OptionalChain` protocol plus `Optional` and `Result` conformances (above). Builtins `OptionalChainProtocol`, `OptionalChainBorrowMethod`, `OptionalChainMutateMethod`, `OptionalChainTakeMethod` in `kestrel-hir/src/builtin.rs` |
| **HIR** | `MatchSource::OptionalChain`, `SugarKind::OptionalChain`. Chain frame stack on the lowering context. Scrutinees are `ProtocolCall(base, OptionalChain, "chainBorrow")`. Assignment and compound assignment lowered into the innermost arm (§1). No new `HirExpr` variant, so the walker sweep is zero |
| **Type infer** | `Match` arm: when `source == OptionalChain`, check arms against `Optional[α]` (§3). `ChainMode` decision per bind, recorded in `TypedBody.chain_modes` (§2), including outward propagation. `mark_synth_pending` covers `Field` / `TupleIndex`. Missing-conformance error E157 via the `Sugar` cascade-suppression hook (the primary `Conforms` on `OptionalChain`), like `for` / `try` |
| **Analyzers** | `access_mode` reads `chain_modes` for exclusivity and `let`-base mutability. A consuming chain on a place is E158 |
| **MIR** | Lower the scrutinee `ProtocolCall` with the method named by `chain_modes`, not the provisional name. The rest is ordinary match lowering. MIR snapshot tests pin each mode |
| **LSP** | Completion after `?.` lists `Wrapped`'s members. Hover on `?.` shows the chosen mode |
| **Docs** | New `docs/language/optional-chaining.md`, the operator table in `kestrel-hir/docs/desugaring.md`, `docs/language/syntax.md` |

## Error cases

Codes are proposals. Confirm they're still free when implementing (the next
free E1xx on the base commit was E156; the pipe design no longer needs one).

| Code | Where | Message |
|---|---|---|
| E157 | type infer | `` `?.` requires a type that conforms to `OptionalChain`, found `T` `` + note "use `.` for a non-optional value" |
| E158 | analyzer | `` can't consume through `?.` on a place `` + note "use `a.take()?.…` to move the value out" |
| E200 family | existing | mutating through `?.` on a `let` base |
| E503 | existing | tail value is neither Copyable nor Cloneable (§4) |

## Edge cases

- **Nested optionals:** `a: Int64??`. `a?.description` binds `Int64?` and calls
  `description` on the inner `Optional`. Needs a test.
- **Generic tail:** `func f[T](o: Box[T]?) -> T? { o?.value }` promotes (rigid
  `T`). Needs a test with `T = Int64?`.
- **Generic base:** `func g[O: OptionalChain](o: O) -> O.Wrapped? …` works
  through the protocol, which is the payoff of making it a protocol.
- **Implicit `it`:** `{ it?.name }` works.
- **`?.` on a type path** (`Type?.x`): parse error, as today.
- **Short-circuit side effects:** `a?.f(sideEffect())` doesn't run
  `sideEffect()` when `a` is `.None`. Needs a test.

## Test plan

Tests first, under `lib/kestrel-test-suite/testdata/expressions/optional_chaining/`:

- Execution, read: field, method, subscript `a?.items(0)`, `f?.(x)`,
  multi-level, flattening, promotion, `??` fallback, `.None` skipping tail
  side effects, `if let` with a chain, inside a closure.
- Execution, mutate: `a?.push(x)`, `a?.n = 1`, `a?.n += 1`, `a?.b?.push(x)`
  (outward propagation), the RHS not evaluated on `.None`.
- Execution, take: `load()?.consume()`.
- Execution, `Result`: `ok?.x` is `.Some`, `err?.x` is `.None`, and the tail
  doesn't run on `.Err`; mutation through `r?.n += 1`.
- Execution, protocol: a user type conforming to `OptionalChain`; a generic
  `O: OptionalChain`.
- Diagnostics: E157 on a non-conforming type, E158, mutating through a `let`
  base, E503 on a non-Cloneable tail, exclusivity `a?.push(a.count)`.
- MIR snapshot: each mode calls its method, and a borrowed place is never
  copied.
- Inference: `let x: Int64? = s.count` (the `mark_synth_pending` fix).

## Effort

About 2 weeks:

| Work | Estimate |
|---|---|
| Parser (chain marking, `?.(`, assignment LHS) | 1 day |
| Stdlib protocol, builtins, `Optional` conformance | 0.5 day |
| HIR lowering (frame stack, nested matches, assignment) | 2 days |
| Inference (check-mode rule, `mark_synth_pending`, `ChainMode` + propagation) | 3–4 days |
| Analyzer and MIR (mode-driven method choice, E157/E158) | 1–2 days |
| Tests, LSP, docs | 2 days |

`ChainMode` is the risk. If the decision point turns out not to be clean in
the solver, the fallback is a syntactic pre-pass. Any `=`, compound-assign or
`&mutating` argument in the tail forces `chainMutate`, and otherwise mutating
*methods* are found by a post-inference re-check that reports "re-run with
mutate". Only the first half is cheap, so it's worth prototyping
`ChainMode` first.

## Alternatives considered

- **Optional-only, `&` binder patterns, no protocol.** Simpler (no stdlib
  addition), but it goes against "operators are protocols" and rules out weak
  references and user types.
- **Reuse `Tryable`.** `tryExtract()` is `consuming`, so it would move a place
  base, and its residual model is about early return, not a per-expression
  result.
- **New `HirExpr::OptionalChain` node.** Clearer HIR, but every `HirExpr`
  walker needs an arm. `Match` + `MatchSource` carries the same information.
- **A new `?.` token.** Equivalent, but it interacts with `as?` and with `T?`
  followed by `.`.

## Implementation notes to confirm in the prototype

These have a chosen answer. A prototype must confirm it holds.


- **Q3. The binder's type before the mode is known.** §2 types `$o` as
  `&W` provisionally and may switch the mode to `chainMutate` later. If
  `unify` distinguishes `&W` from `&mutating W`, the switch invalidates
  types already solved. **Recommendation:** type the binder as `&W` for
  the whole solve and treat mutability purely as `ChainMode` (enforced by
  `access_mode`, lowered by MIR). This matches how `Ref` mutability is
  "analyze's job, not typing's" today. A prototype must confirm that member
  resolution of a `mutating` method on a `&W` receiver isn't rejected during
  solving.
- **Q4. A tail type that's still unknown when the synth gate drops.** §3
  defers the coerce until the tail resolves. If the gate drops first (the
  stall-breaker), `Coerce` unifies and *assumes flattening*.
  **Recommendation:** for `OptionalChain` arms, never unify on stall. Keep
  deferring, and report "can't infer the type of this `?.` chain" if it never
  resolves. A wrong flatten would be a silent miscompile.
- **Q5. `Optional[&T]` bases.** `chainBorrow` on `Optional[&T]` returns
  `Optional[& &T]`. **Recommendation:** rely on the existing ref-of-ref peel
  and add a test. If the peel doesn't apply to protocol returns, specialize
  `Optional[&T]`'s conformance to return `Optional[&T]`.
- **Q6. Smaller syntax points (no decision needed).** `a?.0` (tuple index)
  works through `member()`, so it's added to the tests.
  `x as? T?.y` stays a parse error, as today; write `(x as? T)?.y`.

## Decisions

- **`Result` conforms to `OptionalChain`** (2026-10-25). `r?.x` chains on the
  `Ok` value and gives `Optional`, dropping the error.
- **No lint for `try a?.b()` in 0.19** (2026-10-25). The interaction in §6 is
  documented only.
