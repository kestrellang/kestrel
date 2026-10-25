# Optional Chaining (`a?.b`) Design

> **Status: design, not started.** Milestone 0.19 ("Expression sugar").
> Base commit `def25459` (`future/0.18`). File and line anchors refer to that
> tree. Companion designs: [placeholder arguments](../placeholder-arguments/placeholder-arguments-design.md),
> [pipe operator](../pipe-operator/pipe-operator-design.md).

## Overview

Optional chaining reaches through an `Optional` without unwrapping it by hand:

```kestrel
let theme = user?.profile?.settings.theme ?? Theme.default
let n = cache?.lookup(key)?.count         // Int64?
```

`a?.rest` evaluates `a` once. If it's `.None`, **the rest of the postfix
chain is skipped** and the whole expression is `.None`. Otherwise the chain
continues on the payload, and the result is wrapped in `Optional`, unless it
already is one, in which case it's **flattened**: `a?.b` with `b: Int64?` is
`Int64?`, not `Int64??`.

The feature is sugar over `match` with `&` binder patterns, both of which
already exist. The design adds **no new `HirExpr` variant**. It adds one
`MatchSource` and one inference rule (check mode against `Optional[α]`). That
keeps the walker sweep (captures, analyzers, MIR, LSP) at zero.

### Phasing

| Phase | Milestone | Contents |
|---|---|---|
| **1. Read-only chaining** | 0.19 | `?.` member, method and subscript access; `?(…)` calls; place bases borrowed with `&`; rvalue bases owned; flattening |
| **2. Mutating chaining** | 0.20 (with `mutating get`) | `a?.push(x)`, `a?.field = v`, `a?.n += 1`; `&mutating` binder chosen from the resolved members |

Phase 2 is designed here (§7) so phase 1 doesn't paint it into a corner, but
it isn't part of the 0.19 work. *See open question Q1.*

### Non-goals

- **`Result` chaining.** `?.` is Optional-only. Generalizing via `Tryable`
  doesn't work: `tryExtract()` is `consuming` (it would move a place base),
  and "same container, new payload" needs higher-kinded types. Use
  `r.ok()?.x` to go through Optional.
- **Optional pattern sugar** (`case let x?`). Not in 0.19.

## Syntax

```
postfix_op ::= … | '?' '.' member | '?' '(' args ')'   // '?' joined to the following token
```

`?.` and `?(` are **two tokens**, `Question` then `Dot` / `LParen`, with no
whitespace between them (`p.joined_at`, `kestrel-parser/src/core.rs:214`).
There's no new lexer token: `?` is unused in expression position today. It
only appears in types (`Int?`) and in `func f() ?`. Keeping it two tokens
avoids a lexer interaction with `as?` and `??`.

| Written | Means |
|---|---|
| `a?.b` | `match a { .Some(&$o) => $o.b, .None => .None }` + flatten/wrap |
| `a?.b.c()` | `match a { .Some(&$o) => $o.b.c(), … }` (whole tail skipped) |
| `a?.b?.c` | nested: the second bind runs inside the first `.Some` arm |
| `a?.items(0)` | subscript (call syntax) on the payload |
| `f?(x)` | call an optional function value: `match f { .Some(&$o) => $o(x), … }` |
| `(a?.b).c` | parentheses end the chain: `.c` applies to the `Optional` |
| `a?.b ?? d` | `(a?.b) ?? d`: `??` (power 15) is a binary operator, the chain is postfix |
| `a ? .b` | not a chain (`?` must be joined): parse error |

### Where a chain ends

The chain is the **maximal postfix chain** containing the `?`: member
accesses, tuple indexes, calls, trailing closures, `!`, `..`, and line-continued
`.member`s. It ends at any binary operator, `as?`/`is`, a closing delimiter,
or the end of the expression. This is Swift's rule.

## Semantics

### 1. Desugaring

The parser marks the chain (§Pipeline). HIR lowering turns each `?` bind into
a nested match. For binds `b₁ … bₙ` in evaluation order, with `bᵢ` scrutinizing
`sᵢ`:

```
Sugar(OptionalChain,
  Match(source: OptionalChain, scrutinee: s₁, arms: [
    .Some(<binder> $o₁) => Match(OptionalChain, s₂[$o₁], [
        .Some(<binder> $o₂) => … tail[$oₙ] …,
        .None => .None ]),
    .None => .None ]))
```

`sᵢ₊₁` is lowered with `Local($oᵢ)` in place of the bound sub-expression, so
it's only evaluated inside the `.Some` arm. Lowering is bottom-up, like the
CST: lowering `ExprOptionalChain` pushes a frame on a lowering-context stack;
lowering each `ExprOptionalBind(inner)` lowers `inner`, defines a fresh local
`$o`, records `(inner, $o)` in the frame and returns `Local($o)`; when the
chain's tail is lowered, the frame is popped and the matches are built from the
last bind outwards.

### 2. Binder mode: borrow places, own rvalues

| Base `sᵢ` | Binder | Why |
|---|---|---|
| A place (`Local`, `Field`, `TupleIndex` chains of those, `self`) | `&$o` (borrow, `HirPat::Binding { by_ref: Some(false) }`) | `a?.count` must not move `a` |
| Anything else (call result, literal, an inner `$o` read through a method) | `$o` (owned) | The value is a temporary. Owning it means `makeOpt()?.consume()` works and the payload drops at the arm's end, with no spill |

This is decided **syntactically in HIR lowering** with `is_place_syntax`
(`kestrel-hir-lower/src/desugar.rs:26`), the same classifier compound
assignment uses. Phase 2
adds `&mutating` (§7).

### 3. Typing and flattening

The match's result is checked against `Optional[α]` (fresh `α`). If there's
an outer expectation (`let x: Int64? = a?.b`), it's unified with
`Optional[α]` first. In `generate.rs`'s `HirExpr::Match` arm
(`generate.rs:741`) that's the existing **check mode** path: each arm body
goes through `check_leaf`, which emits `Coerce(body → Optional[α])`.
`Coerce` already tries `unify` first and falls back to `FromValue` promotion:

| Tail type | `Coerce(tail → Optional[α])` | Result |
|---|---|---|
| `U` (not Optional) | unify fails → promote via `FromValue` | `Optional[U]`, wrapped with `.from` |
| `Optional[U]` | unify succeeds, `α = U` | `Optional[U]` (flattened) |
| Generic `T` (rigid) | unify fails → promote | `Optional[T]`, also after mono when `T = Optional[X]` (correct: the generic body decided) |
| `Never` (`a?.fail()`) | `Never` arm doesn't pin | `Optional[α]`, `α` from context or defaulted |

The `.None` arm is an `ImplicitMember` checked against `Optional[α]`.

**One required change.** `check_leaf` only defers the coerce for leaves whose
own constraint produces their type (`mark_synth_pending`, `generate.rs:149`,
which covers `Call`, `MethodCall` and `ProtocolCall`). A chain tail that's a
**field access** (`a?.name`) is a `Field`, so its coerce would unify the
still-unresolved field type with `Optional[α]` before member resolution runs.
Then `name: String` collides with `Optional`. Fix: `mark_synth_pending` also
covers `Field` and `TupleIndex` (their `Member` constraint produces the type).
That's a general bidi fix: `let x: Int64? = s.count` has the same latent
order dependence. Until the gate drops at the stall-breaker, a tail of
unknown type waits.

### 4. Values out of a borrowed payload

With a `&$o` binder, the tail reads through a reference. The arm value then
**decays** like any match-arm value (`docs/language/references.md`, "Decay"):

- `Copyable` → copied.
- `Cloneable` → cloned (for example `user?.name` with `name: String`).
- Neither → E503 "move out of a borrow", as for
  `match a { .Some(&x) => x.handle, … }` today. The user writes
  `a?.handle.asRef()`-style access or matches by hand.

Method calls on `$o` use the method's own receiver mode. Borrowing methods
just work, `consuming` ones are E503 on a place base (fine on an rvalue base),
and `mutating` ones are E159 in phase 1 (§Errors).

### 5. Statement position

`log?.write(line);` has type `Optional[()]` and is discarded. There's no
unused-value analyzer on the base commit, so nothing warns. If one is added
later, it should exempt `Optional[()]`.

### 6. Interactions

- **`??`:** `a?.b ?? d` works as expected, since `??` binds looser than any
  postfix.
- **`try`:** `try a?.b()` where `b` throws gives `try` an
  `Optional[Result[…]]`. Its `Tryable` is Optional's, so `try` unwraps the
  Optional and yields the `Result`. That's legal but surprising. Phase 1
  documents it; a lint can follow.
- **`!`:** `a?.b!` force-unwraps `b` *inside* the chain (`!` is a chain
  postfix). `(a?.b)!` unwraps the chain's result.
- **Placeholders:** `a?.f(_)` builds a closure inside the arm. That's legal.
- **Pipe:** `a?.b |> f` pipes the chain's `Optional`, since `|>` ends the
  chain.
- **Closures:** a chain inside a closure captures `a` by the closure's kind
  rules. The `$o` binders are locals of the closure body.
- **`Optional[&T]` base:** binding `&$o` against a `&T` payload gives
  `& &T`, which peels to `&T` under the existing ref-of-ref rules. Needs a
  test.

### 7. Phase 2: mutating chains (0.20, designed here)

`a?.push(x)` and `a?.field = v` need `&mutating $o`, and that's only known after
member resolution in type inference. Phase 2:

1. HIR lowering emits the binder for a **`var` place** base with a new
   "inferred" binder mode (`by_ref` grows from `Option<bool>` to a three-way
   enum). Everything else stays as in §2.
2. Inference resolves `Inferred` to `&mutating` if any member resolved in the
   arm's tail is `mutating`, or the chain is an assignment target. Otherwise
   it resolves to `&`. This mirrors how a method call's receiver access mode
   is already chosen after resolution.
3. Assignment `a?.b = v` lowers to the match with `$o.b = v` in the `.Some` arm
   (type `Optional[()]`). Compound assignment rewrites the same way. The
   place check accepts an `OptionalChain` match whose tail is a place.
4. Exclusivity (`a?.f(a.count)`) is the existing access-mode analyzer's job.

Phase 1 must not foreclose this. The binder choice lives in one helper, and
E159/E160 are the phase-1 stand-ins for the phase-2 paths.

## Current state

On `def25459`:

- `Question` (`?`) is a token used only in types and after `func f()`.
  Expression grammar never sees it.
- `&` / `&mutating` binder patterns exist (`HirPat::Binding::by_ref`,
  `kestrel-hir/src/body.rs:420`). So do arm-value decay and E503.
- `FromValue` promotion through `Coerce` exists. Check-mode match arms use
  `check_leaf`.
- `Optional`, `.Some`, `.None` resolve as builtins (`Builtin::OptionalEnum`,
  `OptionalSomeCase`, `OptionalNoneCase`).

## Pipeline impact

| Stage | Change |
|-------|--------|
| **Lexer** | None (two existing tokens, joined) |
| **Parser / CST** | `ExprOptionalBind` node: wraps the operand and holds `?`. `ExprOptionalChain` node: wraps the whole postfix chain. In `tight_ops` (`exprs.rs:354`): on `Question` joined to a following `Dot` or `LParen`, close the chain, wrap it in `ExprOptionalBind` (bump `?`), set a "saw bind" flag, and continue (`member` / `call`). In `postfix()` (`exprs.rs:330`): include the line-continued `.member` loop. When closing a chain whose flag is set, wrap the result in `ExprOptionalChain`. The `cond` variant (`cond_primary`) gets the same treatment, so `if let x = a?.b { }` works. Add both kinds to `kinds.txt`; regenerate |
| **Decls (ECS)** | None |
| **HIR** | `MatchSource::OptionalChain` and `SugarKind::OptionalChain`. A chain frame stack on the lowering context. Lowering for both nodes as in §1. The binder choice (§2) uses `is_place_syntax`. `MatchSource::is_desugared()` already returns true, so exhaustiveness analysis skips it |
| **Type infer** | In the `Match` arm: when `source == OptionalChain`, build `Optional[α]` (unify with `expect` if present) and run the check-mode loop against it. Add `Field` / `TupleIndex` to `mark_synth_pending` (§3). A "`?.` on non-Optional" error when the scrutinee resolves to a non-Optional (E158) instead of the generic pattern mismatch |
| **Analyzers** | `access_mode`: a `mutating` member through a `&` binder from an `OptionalChain` match reports E159 (phase 1) instead of the generic mutability error. The assignment-target check reports E160 for an `OptionalChain` target |
| **MIR / codegen** | None expected: a `Match` with `&` / owned `Some` binders and a promotion-wrapped arm already lowers. Verify with MIR snapshot tests (a place base must produce a borrow, not a copy, of the scrutinee) |
| **LSP** | Completion after `?.` lists the **payload's** members (the receiver at the cursor is the bind, whose type is `T`). Hover on `?` shows `Optional[T] → T` |
| **Docs** | `docs/language/` (Optional section of types or a new `optionals.md`), `kestrel-hir/docs/desugaring.md`, `docs/language/syntax.md` |

## Error cases

Codes are proposals. Confirm they're still free when implementing (the next
free E1xx was E156 on the base commit, and the pipe design proposes
E156–E157).

| Code | Where | Message |
|---|---|---|
| E158 | type infer | `` `?.` requires an Optional, found `T` `` + note "use `.` for a non-optional value" |
| E159 | analyzer | `` mutating method `push` can't be called through `?.` yet `` + note "unwrap with `if let` and call it on a `var` binding" (removed in phase 2) |
| E160 | analyzer | `` assignment through `?.` isn't supported yet `` (removed in phase 2) |
| E503 | existing | Tail value is neither Copyable nor Cloneable (§4) |
| — | parser | `?` not joined to `.`/`(` in expression position: the existing "unexpected token" error is enough |

## Edge cases

- **Nested optionals:** `a: Int64??`, `a?.description` binds `$o: Int64?` and
  calls `description` on the inner `Optional`. That's correct and needs a test.
- **Chain inside a condition:** `if let n = a?.count { }` relies on the
  `cond_primary` path.
- **Generic tail:** `func f[T](o: Box[T]?) -> T? { o?.value }` promotes
  (rigid `T`). With `T = Int64?` at the call site, the result is
  `Optional[Optional[Int64]]` represented as `T?`. Expected, and it needs a
  test.
- **Never tail:** `a?.crash()` has type `Optional[α]`. Defaults apply.
- **Implicit `it`:** `{ it?.name }` works. The chain is inside the closure body.
- **Static members:** `Type?.x` isn't valid. `?` after a type path in
  expression position is a parse error (same as today).

## Test plan

Tests first, under `lib/kestrel-test-suite/testdata/expressions/optional_chaining/`:

- Execution: field, method, subscript, `?(…)` call, multi-level `a?.b?.c`,
  flattening (`Optional[U]` tail), promotion (`U` tail), `??` fallback, `.None`
  short-circuit skipping side effects in the tail (a counter), rvalue base,
  `if let` with a chain, chain inside a closure.
- Execution: a `Cloneable` tail (`user?.name`) is cloned, and the base is still
  usable after.
- Diagnostics: E158, E159 (`a?.push(1)`), E160 (`a?.x = 1`), E503 on a
  non-Cloneable tail, `consuming` method on a place base.
- MIR snapshot: a place base is borrowed, not copied. An rvalue base is owned
  and dropped in the arm.
- Inference: `let x: Int64? = s.count` (the `mark_synth_pending` fix,
  independent of `?.`).

## Effort

Phase 1, about 1 week: parser (1 day), HIR lowering with the chain frame stack
(1–2 days), inference rule and `mark_synth_pending` fix (1 day), analyzer
diagnostics (1 day), tests, LSP completion and docs (1–2 days).

Phase 2 (0.20), about another week: inferred binder (2–3 days), assignment and
compound assignment through chains (2 days), tests.

Reusing `Match` instead of a new `HirExpr::OptionalChain` node is what keeps
phase 1 near a week. The earlier scoping estimate (1.5–3 weeks) assumed a new
node plus phase 2.

## Alternatives considered

- **New `HirExpr::OptionalChain { base, binding, body }`.** Clearer intent in
  HIR, but every `HirExpr` walker (captures, move tracking, access mode,
  control flow, MIR, LSP) needs a new arm. `Match` + `MatchSource` gets the
  same information with no sweep.
- **Desugar to `a.asRef().flatMap { … }`.** The tail would run in a closure,
  so `return`/`break` in the tail would change meaning, and `flatMap` needs
  the tail to already be Optional.
- **A new token `?.`.** Equivalent, but it interacts with `as?` and with types
  like `T?` followed by `.`. Two joined tokens are simpler.

## Open questions

- **Q1.** Phase 1 only in 0.19 (recommended), or both phases?
- **Q2.** Include `f?(x)` (optional call / direct subscript) in phase 1? It
  costs one extra parser arm. This design says yes.
- **Q3.** Should the `try a?.b()` interaction (§6) get a lint in 0.19, or wait
  until it bites someone?
