# Bidirectional type checking for `kestrel-type-infer` — design

**Status:** design exploration, not implemented. Language rulings D2, D3, D7
were made on 2026-10-06; see *Maintainer rulings* below. Prototype hooks exist only on
the local branch `bidi-design` (never pushed).
**Base:** `arch/fixes` @ `ae1aeedaa33cd983c4d186c733224b261547fecf`
(`ae1aeeda test: pin memory-safety holes, miscompiles and front-end bugs from
the architecture review`). Every measurement below was taken on a fresh
`cargo build --release --bin kestrel` of that commit plus the prototype hooks
in §15, which are inert unless their `KESTREL_DEBUG` category is set.

**Evidence tags.** **[M]** = measured; the command and base are in §15.
**[R]** = reasoned, not measured. **[M-toy]** = checked by the standalone toy
checker in §15.3, which models the rules but not the real compiler.

---

## Maintainer rulings (2026-10-06)

These override the recommendations below wherever they disagree. The rest of
the document is unchanged and still argues for the original recommendations;
read it with this section in mind.

| # | Ruling | Replaces |
|---|---|---|
| D2 | **Rust-style numeric literals.** An integer or float literal is an open var for the whole body: any later line may determine it, and if nothing does it takes its default (`Int64` / `Float64`) at the end of the body. No statement regions. | §3: end-of-statement defaulting |
| D2′ | **Collection and string literals default at their first use as a receiver.** `var out = []` stays open; at `out.append(x)` its head becomes `Array` (likewise `String`, `Dictionary`) and its *element* stays a body-scoped general var, so `x` still decides it. Rust has no equivalent (its `Vec::new()` / `""` are never polymorphic). | — |
| D3 | **A member access needs its receiver's head type (Rust E0282).** If it is still unknown after flushing obligations, error with an "annotate" fix-it. A still-open **numeric** literal as a method/field receiver (`let x = 7; x.abs()`) is that error too (Rust E0689); it is never defaulted on the spot. | §6 (unchanged, now also covers numeric literals) |
| D7 | **(i): `FromValue` promotes at every check leaf**, `Optional` and `Result` alike. Nested types (`T??`, `Result[Result[…]]`) wrap into the innermost type that fits. | §9 open question |
| — | **Overflow is a separate design.** Under D2, `let x = 100; takes8(x); let w = x * 2` types `x : Int8` and, because integer arithmetic wraps by documented design, `w` is `-56` (N4). Rust's safety net is a debug-build overflow panic; whether Kestrel adopts one is a follow-up design, not part of this checker. N4 gets a pinned test documenting today's behaviour. | §3.4 |

**Measured cost of D2 + D2′ + D3** **[M]**: `bidi-recv` alone (statement
regions off) on `6f838c5c`'s release build, stdlib + 17 `lang/` packages +
13 examples, all units compiling with 0 errors:

| Receiver at a method/field site | stdlib | `lang/` + examples | Outcome |
|---|---:|---:|---|
| open numeric literal var | 0 | 0 | D3 error; costs nothing |
| open `[]` / `""` / `[:]` literal var | 1 | 232 | D2′ defaults the head; costs nothing |
| unknown, rooted in a literal | 4 | 49 | to classify in P2 (mostly chains off the row above) |

Operators on open literal vars are 159 (stdlib) and 510 (packages) sites;
they need the operator rule below rather than a known receiver.

**What changes in the plan.**
- **P2 shrinks.** No statement regions and no 9 stdlib annotations; the 4
  testdata files of §13.6 keep their current verdicts. P2 is now Rule L (N3),
  D2′, and the *operator rule for open numeric vars*: as in Rust's built-in
  binop rule, the two operands of a homogeneous arithmetic/comparison operator
  unify, and the protocol conformance is a queued obligation settled when the
  var is determined or defaulted. Shifts and other heterogeneous protocols
  are excluded, as in Rust. Whether every numeric `Addable` etc. has
  `Output = Self` (needed to type `x + 1` before `x` is known) must be
  checked first.
- **N4 is not fixed by the checker**; see the overflow ruling.
- D3's E106 now also fires for numeric literal receivers (0 sites measured).

## Implementation status (2026-10-07)

**P0 — written, on branch `bidi-p0`** (base `arch/fixes` @ `3b05a25d`):

| Part | What | Verified |
|---|---|---|
| P0a | Fx hashing in `kestrel-type-infer`; `report_unresolved_slots` in source order; `Compiler::diagnostics()` sorted; driver tracks printed diagnostics by key | full `.ks` suite = baseline (3856 / 26 known); determinism tests fail before ("run 1 differs from run 0") |
| P0b | `InferError::code()` — one exhaustive match, E102–E118 (table in `docs/error-codes.md`); E100 only on the never-shown `FromHir` | full suite = baseline |
| P0c | `?` and the `Error` placeholder print as `_` | pending |
| P0d | `Reason` on `Equal` / `Coerce` / `EqualDecayed`, carried into `TypeMismatch` by the dispatcher; secondary label "expected because of this annotation / return type / parameter / first branch / first element / target" | pending |

**Deviation in P0c.** §10's "a type containing `Error` suppresses the
diagnostic" is **not** implemented: `InferCtx::poison` creates `Error`
without reporting, so a mismatch mentioning `Error` can be the *only*
diagnostic (e.g. `closure_arity_mismatch_too_few.ks`) and suppressing it
would make a wrong program compile. The rule needs §10's invariant —
`report_error` is the only producer of `Error` — which P3 establishes.
Until then the placeholder is rendered as `_`.

**Not yet carrying a reason:** literal-vs-annotation mismatches (they surface
as conformance errors, E107/E114), assignments to locals (`AssignTarget`),
and closure returns.

## 0. Summary

### Recommended design in ten points

1. **A Rust-shaped checker, not a Swift-shaped solver.** One left-to-right walk
   over each body with two judgments, `synth(e) → T` and `check(e, E)`, plus
   one coercion judgment, `subsume(T ≤ E)`. Unification variables survive, but
   only to model the unknowns the language really has. Pending protocol
   obligations sit in a small queue, which is flushed at defined points
   (Rust's `FulfillmentContext`).
2. **Two scopes for unknowns.** *General* variables (generic instantiation,
   the element type of `[]`, unannotated closure parameters) are **body-scoped**,
   so `var r = []; r.append(x)` keeps working (47 such sites in `lang/`
   **[M]**). *Literal* variables are **statement-scoped**: a literal that
   nothing in its statement pins takes its default (Int64 / Float64 / String /
   …) when the statement ends.
3. **The literal rule users learn:** *"A number literal takes the type its
   statement demands — from an annotation, a parameter, or the other operand of
   an operator — and if the statement demands none, it is `Int64` (or
   `Float64`) from then on."* This fixes N3 and N4. In the prototype it costs
   exactly **9 annotations in the stdlib, 0 in the 17 `lang/` packages and 13
   examples, and 4 of 3,716 testdata files (all of them the N4 shape)
   [M]**.
4. **Operators stay protocol-per-operator.** `a op b` synthesizes `a`, and
   `a`'s conformances to the operator protocol (`Addable[Other]`, …) decide.
   Two literal rules complete it. **Rule L:** a literal left operand adopts the
   right operand's type (Rust's built-in binop rule). **Rule C:** a tree of
   operators over literals only is resolved against the expected type, by
   trying the nominal types the expectation mentions plus the default. Rule C
   replaces the operator-shape projection, the context-literal pass and the
   relax loop.
5. **A member access needs its receiver's head type.** The checker first
   flushes pending obligations. If the receiver is still unknown, that is a
   specific error at the use site with a fix-it (Rust E0282). It is not a
   deferred constraint that some later statement may or may not unblock.
   Across 37,981 member, field and operator sites in the stdlib, the 17
   `lang/` packages and the 13 examples, **0** receivers are genuinely unknown
   at the point a left-to-right checker reaches them **[M]**. Every receiver
   that today's solver leaves unknown at that point traces back to a literal,
   a closure parameter, a desugaring temporary, or a call waiting on a literal
   argument. The design resolves all four classes by construction (§6.1).
6. **Overloads are resolved in fixed steps, with no backtracking.** Within
   one scope Kestrel overloads **by label** (E426). The one exception is
   several implementations of *one generic protocol requirement*, such as
   `UInt32`'s seven `init(from:)` coming from `Convertible[Int8]` …
   `Convertible[UInt64]`. These resolve through the protocol, and the
   checker infers the protocol argument from the call's argument — the same
   mechanism as heterogeneous operators. That leaves this order: labels →
   protocol-instantiation inference → receiver applicability (three-valued) →
   extension specificity → argument types, synthesized once and never
   re-checked → a literal-default tie-break → a named ambiguity error.
   Free-function calls that needed argument *types* to choose an overload:
   **0** in the stdlib, `lang/`, examples and the no-stdlib testdata **[M]**.
7. **One three-valued entailment kernel** (`Holds / Fails / Unknown`) replaces
   the 5 where-clause evaluators, mono's included. It lives in a new leaf crate
   `kestrel-entail`, using the `kestrel-copy-fold` pattern: one kernel, a
   trait for each type representation, and adapters for HIR, the solver and
   MIR. *Unknown* never means "permit". In the checker it becomes a pending
   obligation, which fixes N1. Equality clauses are goals like any other, and
   in the checker they are unifications, which fixes N2.
8. **Expectations flow down, and coercions happen at the leaves.** `.Some` /
   `.Ok` promotion (`FromValue`), ref decay, `!`, and closure kind and
   convention all live in `subsume`. `subsume` runs at *every* check-mode leaf:
   arms, array elements, block tails, closure bodies, returns, arguments. So
   promotion no longer depends on position. Four promotion shapes are
   rejected today **[M]** and accepted under the design (**[M-toy]** for
   three of them). The 5 ref-decay registration sites go away.
9. **Each closure argument is checked once its expected type is known.** Its
   kind and parameter types come from that type. Closure arguments are checked
   after the other arguments of the same call (Rust and Kotlin do the same), so
   `apply(with: { it + 1 }, to: 5)` still works.
10. **Diagnostics name the reason for every expectation.** Every expectation
    carries a cause: an annotation, a parameter, an operator operand, the first
    arm, or the return type. Errors are emitted in source order. Inference
    errors get specific codes in the free E102–E120 range. Printed types never
    contain `?` or `Error`; an unknown prints as `_` inside an "add an
    annotation" message.

### The five decisions with the most consequence

| # | Decision | Recommended | Main alternative | Why |
|---|---|---|---|---|
| D1 | Core algorithm | Rust-style expectation checker + body-scoped vars + obligation queue | Swift per-statement constraint system with scored disjunctions | Kestrel overloads by label and routes operators through protocols. So Swift's disjunction machinery, its main cost, buys almost nothing here [R]. Predictability and diagnostics are better [R]. Measured compatibility: §13. |
| D2 | When literals default | End of the enclosing **statement** | End of body (Rust, today's solver) | N4 is a body-scope artifact: `-56` instead of `200` **[M]**. Statement scope costs 9 stdlib annotations, 0 in `lang/` and examples, and 4 testdata files **[M]**. |
| D3 | Unknown receiver at a member access | Flush obligations, then error with a fix-it | Defer the member (today) | Deferral is the root of the relax loop, the stall breakers and the order-dependent decay sets [R]. Genuinely unknown receivers in the stdlib, `lang/` and examples: 0 of 37,981 **[M]** (§6.1). |
| D4 | Where-clause evaluation | One three-valued kernel crate shared by solver, analyzer and mono | Patch each of the 5 evaluators | N1, N2 and G14 are all "evaluators disagree" or "Unknown treated as permit" bugs [R + pinned tests]. |
| D5 | Migration | Incremental strangler inside the crate, with a fixed `TypedBody` contract and a differential corpus harness | Big-bang rewrite | The prototype shows statement regions work *inside the existing solver* with 0 `lang/` regressions **[M]**. The output contract has only 9 consumer fields, which makes differential testing cheap [M grep]. |

The other decisions, each with its options and tradeoffs in its section:

| # | Decision | Section |
|---|---|---|
| D6 | closures: kind and parameters from the expected type; closure arguments postponed | §8 |
| D7 | promotion: `FromValue` at every check leaf; an open language question for `Result` | §9 |
| D8 | diagnostics: reasons, codes, ordering, poisoning | §10 |
| D9 | operators: protocol dispatch plus Rules L and C | §4 |
| D10 | overloads: labels, then protocol instantiation, applicability, specificity, synthesized argument types | §5 |

### Migration in one paragraph

- **P0:** deterministic diagnostic order, specific error codes, expectation
  reasons. Does not depend on the new checker.
- **P1:** the entailment kernel, and the 5 evaluators routed through it. Fixes
  N1, N2 and G14.
- **P2:** statement regions and Rules L and C inside today's solver (the
  prototype, productionized), plus the 9 stdlib annotations. Fixes N3 and N4.
- **P3:** convert `generate.rs` into `check.rs` one HIR form at a time, so it
  resolves while it walks: receivers first, then calls, closures, arms and
  literals.
- **P4:** delete the deferral machinery.
- **P5:** shrink the solver to the obligation queue.

Every phase ships on its own and is gated by the corpus and a `TypedBody`
differential run. Estimated change: about 9–12k lines touched, net −4 to −6k
[R]. Biggest risks: §14.

---

## 1. Where the current checker stands

These facts are about `arch/fixes @ ae1aeeda`. The docs have drifted, so they
come from the code.

- `solve()` (solver.rs:36) runs these stages in order:
  1. `fixpoint` (capped at 256 rounds; the cap is hit silently).
  2. A relax loop that runs `apply_operator_shape_projections`, then
     `apply_literal_defaults` at levels 0–2, then `apply_ref_decay_defaults`.
  3. `break_stalled_assign_targets`, and the pattern-binder gate is dropped.
  4. `fixpoint` again.
  5. `apply_type_param_defaults`.
  6. `report_unresolved_type_params`.
  7. `default_never_fallback`.
  8. `report_unsolved`.
  9. `validate_ref_placement`.
  10. `report_unresolved_slots`.
- There are **16** `Constraint` variants (constraint.rs) and **28** `InferError`
  variants. Only 3 of the error variants have codes other than E100 (E624,
  E491, E492).
- Many side sets keyed by `HirExprId` or `TyVar` change *how* constraints
  solve: `scrutinee_exprs`, `binding_init_exprs`, `assign_target_exprs`,
  `always_decay_exprs`, `direct_callee_exprs`, `operator_members`,
  `protocol_dispatch_members`, `poison_protocol_call_recv_on_failure`,
  `closure_flex`, `closure_it`, `kind_flex`, `closure_literal_exprs`,
  `pattern_binder_tvs`, `never_fallback_targets`, `errored_coerce_exprs`,
  `wildcard_tvars`, and more. Each one records a fact that a check-mode walker
  would simply *have* at the point it needs it, namely "this position expects a
  value of type E".
- Expectations already leak into generation ad hoc. `expected_array_elem` and
  `expected_dict_entry` are hints for annotated array and dictionary literals
  (generate.rs `HirStmt::Let`). The design generalizes this and does not invent
  it from nothing.

Current behaviour, from probes **[M]** (§15.1):

| Program | Today |
|---|---|
| `let small: Int8 = 10; let b = 100 + small` (N3) | E100 "expected Int64 got Int8" |
| `let x = 100; takes8(x); let w = x * 2` (N4) | compiles; prints `w=-56` |
| `let a = none()` with `func none[T]() -> T?` (N5) | E100 "`? !: Copyable`" |
| `let x: Int64? = if c { 5 } else { .None }` | E100 "`.None not found on Error`" |
| `let xs: [Int64?] = [1, .None, 3]` | 2× E100 "expected Optional[Int64] got Int64" |
| `func opt(c: Bool) -> Int64? { if c { 5 } else { .None } }` | E100 "`.None not found on Error`" |
| `func f(n: Int64) -> Int64 throws E { n * 2 }` | E100 "expected Result[Int64, E] got Int64"; `let r: Int64 throws E = 42` **is** accepted |
| `let m: Int32 = 7.abs()` | E100 "expected Int32 got Int64" |
| N1 pinned test (`Box(v: annotated).describe()` / `Box(v: [NoShow(n: 2)]).describe()` under `where T: Show`) | only the annotated line is rejected; the inline literal is accepted |
| N2 pinned test (`Box(v: "str").plusOne()` under `where T = Int64`) | no diagnostic at all (silent miscompile per the test header) |
| `var r = []; r.append(3)` / `var q = []; q.append(u8)` | compiles; `Array[Int64]` / `Array[UInt8]` |
| `apply(with: { it + 1 }, to: 5)` (closure before its type source) | compiles |
| `let d: Bool = 3 + true` | **two** contradictory errors on one span: "expected Int64 got Bool" and "expected Bool got Int64" |
| 5 independent errors in one body | emitted in the order line 4, 8, 5, 7, 7, 6 |
| `let f = { (x) in x };` (testdata `cannot_infer_without_context_error.ks`), same binary run 8× | **the set changes, not only the order**: 1 copy of "could not infer type" in 4 runs, 2 copies in 4 runs. `cannot_infer_it_type_without_context.ks` likewise gives 1 or 2. The cause is `report_unresolved_slots` iterating `HashMap`s, with span-keyed dedup between expression and local slots |

The promotion rows show position dependence. The same `.None` or `5` is
accepted at a `let` with an annotation, and rejected one level deeper in an
`if` arm or an array element. The cause is that `Coerce`, which can promote,
is emitted only at let, argument, return and assignment boundaries, while arms
and elements use `Equal`.

---

## 2. Core algorithm (D1)

### 2.1 Judgments

```
synth(Γ, e)        ⇒ T            -- infer a type for e
check(Γ, e, E@r)   ⇒ ()           -- e must be usable where E is expected; r = reason
subsume(T ≤ E@r)   ⇒ Coercion     -- the ONE place coercions are decided
```

`check` dispatches on the form of `e` when `e` is *introduction-shaped*:
literals, closures, `if`/`match`/blocks, array, tuple and dictionary literals,
`.Case`, and `throw`/`return`. These push `E` inward. For every other form,
`check` falls back to `synth` followed by `subsume`. This is the standard
Pierce–Turner / Dunfield–Krishnaswami split. The table is the design's
specification:

| HIR form | synth | check against E |
|---|---|---|
| `Literal` | fresh literal var `ℓ: ExpressibleByK`, recorded in the current region | E concrete: `E: ExpressibleByK`, else `subsume` (promotion). E a var: unify (the var becomes literal-flavored) |
| `Local`, `Def` | its type (instantiate a generic `Def` with fresh *general* vars) | synth + subsume |
| `Call` | callee synth → instantiate → args in two phases (§8) → ret | synth with E used as a hint for return-type-only type params (`none()` against `Int64?`), then subsume |
| `MethodCall`, `Field` | receiver synth → **structurally resolve** (§2.3) → lookup (§5) → args → ret | synth + subsume |
| `ProtocolCall` (operators) | §4 | §4 (Rule C uses E) |
| `ImplicitMember` `.Case(args)` | **error** "cannot infer the enum for `.Case`" (E104) | look up `Case` on E, or on its promotion target (`.None` against `Int64?`) |
| `If` / `Match` | first arm synth; later arms *checked* against it (Rust `CoerceMany`) | every arm checked against E |
| `Block` | stmts; tail synth (unit if none, `!` if it diverges) | tail checked against E |
| `Array [e…]` | `[]`: `Array[α]`, α general. Otherwise first element synth, the rest checked against it | E = `Array[T]` (or any `ExpressibleByArrayLiteral` E): each element checked against `Element(E)` |
| `Dict` | as Array, for key and value | as Array |
| `Tuple` | element-wise synth | element-wise check when E is a tuple of the same arity |
| `Closure` | params: annotation or general var; kind `normal` | params, kind and return from E (§8) |
| `Borrow &e` | `Ref{synth(e)}` | as synth + subsume (ref-to-ref rules) |
| `Return e` | checks e against the return type; type `!` | as synth |
| `Break`, `Continue` | `!` (break value checked against the loop's result) | as synth |
| `Assign` | target synth (place); value checked against it (store-through for `&mutating`) | — |
| `Sugar{kind, inner}` | transparent; the reason carries `kind` for diagnostics | transparent |
| `TypeRef` | the type's metatype view (receiver or callee position only) | — |
| `Let x: A = e` | `check(e, A@annotation)` | — |
| `Let x = e` | `synth(e)`, then the **region ends** (§3) | — |

### 2.2 Unknowns: general vars, literal vars, obligations

- **General var `α`.** Introduced by generic instantiation, the element of
  `[]`, the type parameters of a struct initializer, unannotated closure
  parameters with no expectation, and unannotated `var x;`. Its scope is the
  **body**. It is resolved by unification anywhere later in the body. If it is
  still open at the end of the body, that is error E105 "cannot infer type
  parameter `T` of `none`", which names the origin recorded on the var. This
  fixes N5.
- **Literal var `ℓ_K`.** Carries its literal protocol and default. Its scope is
  the **statement** (the region). Unifying `ℓ` with `α` makes `α`
  literal-flavored, so the default reaches it too: `var r = []; r.append(3)`
  gives `Array[Int64]` at the end of the `append` statement **[M-toy, M]**.
- **Obligations.** These are `T: P[args]`, `Proj(T, P.A) == U`, extension and
  generic where clauses, and `Static`/`Copyable` formation checks. They are
  queued when created and *selected* as soon as the entailment kernel answers
  `Holds` or `Fails` (§7). `Unknown` keeps them in the queue. The queue is
  flushed at these points:
  - a structural-resolution point (§2.3);
  - the end of a region, after literal defaulting;
  - the end of the body, where whatever is left becomes "cannot infer" or
    "unsatisfied" errors, which are never silent.

This is exactly Rust's `select_where_possible` / `select_all_or_error`
discipline. The current 16-variant constraint pool shrinks to about 4
obligation kinds; `Member`, `Call`, `OverloadedCall`, `Implicit`,
`ImplicitPat`, `TupleIndex`, `TupleRestPat`, `EqualDecayed`, `AssignTarget`,
`BorrowPointee` and `InterpolationLink` become direct checking code.

### 2.3 Structural resolution — when a type *must* be known

Some places need the **head** of a type at the moment they are checked:
member lookup, a call through a value, tuple index, patterns that destructure,
operator dispatch on a non-literal receiver, `for` iteration, and `try`/`??`.
These call `structurally_resolve(T)`:

1. Zonk `T`. If its head is known, return it.
2. Flush the obligation queue (an associated-type projection may now reduce),
   then zonk again.
3. If it is a literal var and the position is not an operator: default it now.
   This is Rule R in §3.
4. Otherwise report **E106** "the type of `x` must be known here", with a
   fix-it naming the binding to annotate. Then poison the result.

Step 4 is where this design is *less* expressive than today's solver. Today
the member is deferred and may be unblocked by a later statement. §6.1
measures how often that happens: never, in the stdlib, `lang/` and examples
**[M]**.

### 2.4 Order

The walk is **left to right in evaluation order**, with two exceptions taken
from Rust and Kotlin:

- **Postponed arguments.** Closure arguments, and `.Case` arguments whose
  parameter type is still an open var, are checked *after* the other arguments
  of the same call (§8).
- **Receiver before arguments.** For `x.m(args)` and `a op b`, the receiver
  comes first, which matches evaluation order anyway.

### 2.5 Alternatives considered for D1

| Option | Expressiveness | Predictability for users | Diagnostics | Implementation cost | Compatibility with `lang/` |
|---|---|---|---|---|---|
| **(a) Status quo**: global fixpoint over a constraint pool, relax loop | Highest in principle: any later constraint may help | Low. Back-flow across statements (N4), position-dependent promotion, order-dependent decay | Poor. Errors appear where the solver *stalled*, not where the user erred. Contradictory duplicate pairs, `?` in messages, E100 everywhere **[M]** | Sunk, but every fix adds a pass or a side set: 16 constraint kinds, ~15 side sets | 100% by definition |
| **(b) Swift**: per-statement constraint system, disjunctions for overloads and literal bindings, scoring, solver scopes and backtracking | High within a statement, none across statements (`var r = []` is illegal in Swift) | Medium. Statement-local, but the outcome depends on scores | Swift's own history: "ambiguous", "unable to type-check in reasonable time"; it needed a dedicated diagnostic pass (`ConstraintFix`) [R] | Very high: a scoring solver, plus a diagnostics framework | Breaks the 47 `var x = []` sites **[M grep]**. Disjunctions are rarely needed because overloading is by label |
| **(c) Rust-style** (recommended): expectation walk, body-scoped general vars, obligation queue, statement-scoped literals | High. Rejects only receivers that a *later* statement resolves: 0 such sites in the stdlib, `lang/` and examples **[M]** (§6.1) | High. Information flows only *down* (expectations) and *forward* (earlier statements) | Errors at the first point of conflict, with a stated reason | Medium. Most of `resolve.rs`, `unify.rs`, `where_clauses.rs` and `result.rs` survive | 9 stdlib annotations for the literal rule; 0 receiver annotations **[M]** |
| **(d) Pierce–Turner / pure local** (Scala 2-like): type arguments solved per application, no vars escape a call | Low. No `var r = []`, and `let x = none(); takes(x)` fails | Very high | Good | Low | Breaks the 47 `[]` sites, and every generic call whose type argument comes from a later use [R] |
| **(e) Dunfield–Krishnaswami** "complete and easy": ordered contexts, existential vars, polymorphic subtyping | Built for higher-rank polymorphism, which Kestrel lacks. Has no protocols, overloading, literals or promotions | High | Good | High: the hard parts (protocols, literals) are outside its scope | Used here as the **specification style** only (§2.1's judgments) |

**Why (c) over (b), concretely.** Swift needs disjunctions because `+` is
dozens of overloaded *functions* and literals can bind to any conformer.
Kestrel's `+` is **one** protocol requirement, `Addable[Other].add`. The only
types in `lang/` that conform to one operator protocol **more than once** are
`String`, `StringSlice` and `datetime.Instant` **[M grep, §15.2]**. And
duplicate labelled signatures in a scope are rejected (E426), so free-function
"overload sets" differ by label. The disjunction search Swift pays for has
almost nothing to search here. Scala 3, Kotlin and Rust reach the same
conclusion: local, expectation-driven checking plus a small constraint store
per call or body.

**Why (c) over (a).** The current solver *is* (b) without scoring and
without statement scoping. Every defect the review pinned is a symptom of "the
order in which information arrives is not the program order":

- **N3:** the receiver literal is defaulted before its operand is consulted.
- **N4:** information flows back from later statements.
- **N1:** an Unknown is resolved as "permit" because it was asked too early.
- **Promotion:** depends on which constraint kind an expression happened to
  get.
- **N6:** diagnostic order is the solver's queue order.

A checker that walks in program order makes the order of information *the
specification*.

---

## 3. Literals and defaulting (D2)

### 3.1 The model

- A literal synthesizes `ℓ_K`, where K is Integer, Float, String, Char, Bool,
  Null, Array, Dictionary or Interpolation. `ℓ_K` carries the protocol
  `ExpressibleByK` and the default `@builtin(DefaultKLiteralType)`.
- **Check mode resolves a literal immediately.** For `check(lit, E)`:
  - E concrete and `E: ExpressibleByK`: the literal *is* E. Range checking
    (E121) happens here, at the literal, with E named.
  - Otherwise `subsume` tries promotion: `E: FromValue[X]` and `X: ExpressibleByK`.
  - Otherwise the error is "`E` cannot be written as an integer literal",
    E-code, with the reason for E.
- **Unification with a var** merges flavors. Integer ⊔ Float = Float; any
  other mixed pair is an error at the second literal.
- **Defaulting points**, in priority order:
  1. **Rule R — receiver.** A literal used as the receiver of a non-operator
     member (`7.abs()`, `1.0.squareRoot()`) takes its default at that moment.
     It must, because member lookup needs a head (§2.3). This matches Rust,
     where `2.0.powi(2)` is E0689, and Kotlin. Today's behaviour is the same:
     the `7.abs()` probe is rejected today **[M]**.
  2. **Rule L — operator operand.** §4.
  3. **Rule C — expectation candidates.** §4.
  4. **Region end.** When a statement finishes, every literal var created
     inside it that is still open takes its default. Then the region's pending
     operator results are computed, and obligations are flushed.

**A region is one statement** (`let`, expression statement, assignment), *not
counting* the statements nested inside closures in that statement. Those are
checked when their closure is checked, and by then their expected types are
known. It also does not count the statements a desugaring synthesizes inside
an expression: string interpolation and the `$iter` statement of a `for`
loop. The interpolation is a literal of its own. The `for` header is its own
region, so `for i in 0..<n` types `i` from `n` and never from the loop body.
Function and closure *tail* expressions are regions too.

### 3.2 The rule, in one sentence a user can learn

> **A number literal takes the type its statement demands — from an
> annotation, a parameter, or the other operand of an operator — and if the
> statement demands none, it is `Int64` (or `Float64`) from then on.**

**Why literals and general variables get different scopes (the principle).**

- A general var has no default. Unification is order-independent, so any
  later use can only *determine* it. Uses that disagree produce an error, and
  a var that is never determined produces an error. There is never a second,
  silently different answer, so body scope is safe.
- A literal *has* a default, and applying a default is a guess. A guess is
  only sound if it is made before code that could contradict it is consulted.
  Otherwise the meaning of line 1 depends on line 3, which is N4.

So the scopes follow from the semantics:

- **anything with a default** (literals, type-parameter defaults such as
  `H = DefaultHasher`, never-fallback) is resolved at the end of its
  statement;
- **anything without a default** lives for the body.

The second user-facing sentence is: *"The unknown part of a type, such as the
element of `[]` or the `T` of `none()`, is filled in by the first later line
that determines it, and never changes after that."*

**Corollary, no-stdlib code.** With `stdlib: false` there is no
`DefaultIntegerLiteralType` builtin, so a literal has *no* default and behaves
like a general var: it is body-scoped and determined by its uses. This is
what today's no-stdlib tests rely on. 66 no-stdlib testdata files bind
`var i = 0` and let a later `lang.i64_add(i, 1)` fix it to `lang.i64`
**[M, `bidi-n4`]**. The prototype follows the corollary automatically,
because it can only default where a default exists. It changes **0** verdicts
in the 2,249 no-stdlib files; the one stable diagnostic change is a removed
cascade (§13.6).

What users should take from the literal rule: a literal's type never travels
*backwards* from later lines. To make `let x = 100` an `Int8`, write `let x: Int8 = 100`. Swift and
Kotlin users already expect this, and `docs/language/types.md` §"Inference
from Literals" already claims it ("`let x = 42; // Inferred as Int64`"). The
current compiler contradicts its own documentation.

### 3.3 N3 under the design

```kestrel
let small: Int8 = 10;
let b = 100 + small;
```

1. `100 + small` desugars to `ProtocolCall(100, Addable, add, [small])`.
   `synth(100) = ℓ` (Integer).
2. The receiver is a literal and the right operand is not, so synthesize the
   right operand: `Int8`. `Int8: ExpressibleByIntegerLiteral` and
   `Int8: Addable`, so **Rule L** unifies `ℓ := Int8`.
3. Dispatch on `Int8`. It has one `Addable[Other]` conformance, with
   `Other = Int8` and `Output = Int8`. `subsume(Int8 ≤ Int8)` succeeds, so
   `b : Int8 = 110`.

The prototype gives `b=110` **[M]**, and the toy gives `b : Int8` **[M-toy]**.
Today it is rejected **[M]**.

### 3.4 N4 under the design

```kestrel
let x = 100;          // region ends: ℓ defaults → x : Int64
let _ = takes8(x);    // check(x, Int8 @ param 1 of takes8) → subsume(Int64 ≤ Int8) fails
let w = x * 2;        // Int64 * ℓ → ℓ := Int64; w = 200
```

The toy reports this **[M-toy]**:

```
E109 type mismatch at argument 1: expected Int8, found Int64 (expected because
of parameter 1 of `takes8`); note: the literal was defaulted to Int64 at the
end of statement 1
```

The prototype reports E100 "expected Int8 got Int64" on `takes8(x)`, the same
error at the same span **[M]**. The proposed rendering:

```
error[E109]: type mismatch: expected `Int8`, found `Int64`
 --> takes8(x)
     ^ expected `Int8` because of this parameter (func takes8(v: Int8))
note: `x` is `Int64` because the literal `100` was not given a type in its statement (line 5)
help: annotate the binding: `let x: Int8 = 100`
```

Today the program compiles and prints `w=-56` **[M]**.

### 3.5 Options considered for D2

| Defaulting point | N4 | N3 | `var r = []` | Cost in corpus | Notes |
|---|---|---|---|---|---|
| **End of body** (today; Rust's integer fallback) | `w=-56` silently | needs Rule L anyway | works | 0 | Rust also infers `i8` here, and relies on overflow traps or lints to catch the consequence [R]. Kestrel has neither on this path, so today the result is a silent wrong value **[M]**. |
| **End of statement** (recommended; Swift, Kotlin) | type error with a "because" note | needs Rule L | works (α is general, only the literal is regional) | **9 stdlib `let`s, 0 lang, 0 examples, 4 testdata files [M]** | The cost is exactly the "non-default literal let" population (§13.1). |
| **At the binding**: a literal defaults only when it flows into an unannotated `let` | same as statement end for N4 | needs Rule L | works | ≈ same | Harder to state, and leaves `foo(x.bar(1), y)`-style literals to a later rule anyway. |
| **Immediately at synth** (no literal vars) | error | **broken**: `small + 100` would need check-mode only | works | large: every `x + 1` where `x: Int8` would need the argument checked, which it is, but `-1` into an `Int8` slot would need special cases | Literal vars are what make operators work. |

---

## 4. Operators (D9)

The stdlib declares every operator as one protocol method:

```kestrel
protocol Addable[Other = Self] { type Output; consuming func add(consuming other: Other) -> Output }
protocol Equal[Other = Self]   { type Output; func equal(to other: Other) -> Output }
protocol ClosedRangeConstructible[Other = Self] { type Output; func inclusiveRange(to end: Other) -> Output }
```

The numeric types conform with `Other = Self`, and Output is `Self` or `Bool`
(per-type `type Addable.Output = Int8`). Heterogeneous conformances exist only
on `String` and `StringSlice` (`Equal`/`NotEqual` × {`String`,
`StringSlice`}), `Instant` (`Subtractable[Duration]`,
`Subtractable[Instant]`, `Addable[Duration]`) and `Duration`
(`Multipliable[Int64]`, `Divisible[Int64]`) **[M grep]**.

The same "one requirement, several protocol instantiations" shape is how
conversions are overloaded. Each integer type conforms to `Convertible[X]` for
7 source types (each float type for 2), so it has 7 `init(from:)` with the
same label. Today `resolve.rs::try_resolve_through_protocol` resolves them as
the *abstract* requirement `Convertible[α].init(from: α)`, with `α` inferred
from the argument. The design makes this the single rule for both. A member
call that matches several implementations of one generic protocol
requirement checks its arguments against the requirement with fresh protocol
arguments `α`, then queues the obligation `Receiver: P[α]`. The obligation
selects as soon as `α` is known. A literal argument with several fitting
`α`s takes the literal default.

**Checking `a op b`.** It desugars to `ProtocolCall{receiver: a, protocol: P, method, [b], from_operator}`.

1. `A = synth(a)`.
2. **Literal receiver (Rule L).** If `A` is a literal var `ℓ`, compute
   `B = synth(b)`:
   - `B` concrete, `B: ExpressibleByK(ℓ)`, and `B: P`: unify `ℓ := B` and
     continue at step 3.
   - `B` is another literal var: unify the two (flavor join), and make the
     result a **pending operator result** `ρ` with obligation
     `OpLit(op, ℓ, ρ)`. In check mode, Rule C resolves it right away.
     Otherwise the region end resolves it with the default.
   - Otherwise default `ℓ` (Rule R) and continue at step 3.
3. **Known receiver.** `structurally_resolve(A)`, then collect `A`'s
   conformances to `P`, as `(Other, Output)` pairs from
   `ConformingProtocolInstantiations`:
   - exactly one: `check(b, Other @ "right operand of +")`; the result is `Output`;
   - several: synthesize `b` and keep the conformances where
     `can_unify(B, Other)`. If more than one survives and `B` is a literal,
     prefer the one whose `Other` is the literal's default. Otherwise report
     E103 ambiguous, listing the conformances. `s == "x"` needs no tie-break:
     `StringSlice` is not `ExpressibleByStringLiteral`, so only
     `Equal[String]` fits **[M grep]**;
   - none: E102 "`A` does not support `+`", with the protocol named.
4. `Output` is an associated-type projection. When `A` is a param or a
   projection, this projection becomes an obligation.

**Rule C (expected-type candidates).** A pending `OpLit(op, ℓ, ρ)` checked
against `E`:

- The candidates are the nominal heads *mentioned in `E`*, plus `ℓ`'s default.
- Keep each candidate `c` with `c: ExpressibleByK`, `c: P[Other = c]`, and
  `Output(c)` unifiable with `E`.
- If exactly one survives, it wins. If several survive, the default wins. If
  none survives, the default is taken and `subsume` reports the mismatch
  against `E`, with its reason.

Results:

| Expression | Result |
|---|---|
| `let y: Int8 = 100 + 27` | `Int8` |
| `let r: ClosedRange[Int16] = 100..=200` | `Int16` |
| `let c: Bool = 1 == 2` | `Int64` (the default; its `Output` is `Bool`) |
| `let z = 1 + 2` | `Int64` at region end |

All four are **[M-toy]**.

The candidate set is *syntactic and local* (§11.2): no "all conformers of
`ExpressibleByIntegerLiteral`" enumeration, so no global query dependency.

What Rule C replaces, all of which are inside the relax loop today:

- `apply_operator_shape_projections`, which generalizes the default type's
  return by entity substitution;
- the context-literal pass, "operator result already concrete → receiver
  adopts";
- relax levels 0–2.

**Unary operators.** `-ℓ` is `Negatable` with `Output = Self` for every
numeric type, so it keeps the literal var: `let n: Int8 = -5` checks `ℓ`
against `Int8`. `!b` and `~x` work the same way.

**Compound assignment.** `x += e` desugars to `AddAssign` on a *place*. The
receiver is always known (it is a variable), so step 3 applies.

**Options considered:**

| Option | Gets right | Gets wrong or costs |
|---|---|---|
| (a) Receiver-first only (today, minus the hacks) | most code | N3 (`100 + small`), `2.0 * f32` **[M pinned test]** |
| **(b) Rule L + Rule C** (recommended) | N3, range literals, `-1` into `Int8`, `1 == 2` | `7.abs()` into `Int32` stays an error (Rule R; same as today) |
| (c) Swift-style disjunction over every `ExpressibleByK` conformer, with scoring | everything (b) does, plus `let m: Int32 = 7.abs()` | Global conformer enumeration (an incremental-compilation hazard), exponential in operator chains, and ambiguity diagnostics |
| (d) Require annotations whenever the left operand is a literal | simple | user-hostile: `2 * x` is ubiquitous (`array.ks: let left = 2 * idx + 1`) |

The prototype implements Rule L inside today's solver (§15.4) and turns the
pinned N3 shape from E100 into `b=110` **[M]**. Rule C is modelled only in the
toy **[M-toy]**. The prototype keeps today's shape projection for that role.

---

## 5. Member lookup and overloads (D10)

### 5.1 Lookup

`x.m(args)`:

1. **Receiver.** Synthesize `x`, then `structurally_resolve` it (§2.3).
   - A ref receiver `&T` peels to `T` (the transparent place).
   - A projection `T.Item` reduces through obligations. If it stays abstract,
     lookup goes through the bounds that name *this* base (G17).
2. **Candidates.** The members named `m` on the head, from direct children,
   applicable extensions, and protocol requirements through bounds. This is
   today's `resolve_member` / `TypeResolver`, kept as it is.
3. **Labels.** Filter with `arg_binding::bind_arguments`, which stays the
   single source of truth.
4. **Applicability (three-valued).** For each candidate from an extension,
   ask `entails(receiver_env, extension.where_clauses[σ])`, where σ maps the
   extension's target arguments to the receiver's arguments. The result is
   one of:
   - `Fails`: drop the candidate. This is the N2 path: `Box[String]` against
     `where T = Int64` fails.
   - `Holds`: keep it.
   - `Unknown`: an argument is still an open var, as in `Box[α]`. Keep the
     candidate, *tagged*.
5. **Specificity.** Among the survivors, keep the most specific ones. One rule
   is shared with mono's `witness_more_specific`: the implementing type is
   strictly an instance of the other's, then more where constraints wins. It
   becomes a kernel function in §7 and stops being two copies.
6. **Commit.**
   - One survivor: select it. If it is tagged Unknown, its where clauses become
     **obligations**. Equality clauses become unifications (`α := Int64`).
     Bounds stay queued and are re-evaluated when `α` resolves. They fail
     loudly, and are never permitted forever. This fixes N1.
   - Several survivors whose difference is only Unknown applicability: flush
     obligations and retry once. If they are still several: E103 ambiguous,
     listing the candidates and the clause that could not be decided, with
     "annotate `x`".
   - None: E102 "no member `m` on `T`". If a candidate was dropped by
     `Fails`, add a note naming the failed clause and the extension that
     declared it. Today N1 and N2 have no such note.

**N1 under the design.**

1. `[NoShow(n: 2)]` is an *array literal*: a literal var `ℓ_Array` with
   element `NoShow`. Any `ExpressibleByArrayLiteral` type could take it.
2. Checking it against `Box.init`'s parameter `T` makes the receiver
   `Box[ℓ]`. Its head is known, so lookup proceeds.
3. Step 4 asks whether `ℓ: Show` holds, and the answer is `Unknown`.
4. `describe` is the only candidate, so it is committed *tagged*, and the
   obligation `ℓ: Show` is queued.
5. At the end of the region `ℓ` defaults to `Array[NoShow]`.
6. The obligation evaluates to `Fails`.

The failure of a tagged applicability obligation is *rendered as the lookup
failure it stands for*: E102 "no member `describe` on type
`Box[Array[NoShow]]`", plus the note "`extend Box[T] where T: Show` does not
apply: `Array[NoShow]` does not conform to `Show`". That keeps the pinned
test's wording. Today `reify_tv` turns `ℓ` into `HirTy::Error`,
`type_satisfies` treats Error as a permit, and the check is marked solved
forever. Only the annotated twin on line 35 is rejected **[M]**. The N2 test
produces no diagnostic at all today **[M]**. Under the design its receiver
`Box[String]` is ground, so the equality goal `String = Int64` is `Fails` at
step 4.

### 5.2 Overloads

Kestrel overloads **by label** within a scope. Two callables with the same
`(name, labels)` are E426 duplicates, whatever their types (the
`duplicate_callable.rs` module doc; probe `overload_lit.ks` **[M]**). The one
exemption is several implementations of the *same protocol requirement*
(`is_protocol_method_impl`). That is the `Convertible[X]` /
`Equal[Other]` shape, handled by protocol-instantiation inference (§4).
Method ambiguity in today's solver is already settled using the receiver
alone (`solve_member`'s `Ambiguous` arm filters by extension applicability and
specificity, and never looks at argument types). So *type-directed* overload
resolution is needed in only three cases:

- (i) free functions that share labels across modules or imports
  (`OverloadSet` → `solve_overloaded_call` → `types_compatible`);
- (ii) default parameters making label sets overlap;
- (iii) several instantiations of one generic protocol requirement: operators
  and `Convertible.init(from:)` (§4). These are not overloads in the design's
  sense.

(i) and (ii) together: **0** calls reached the argument-type stage of
`solve_overloaded_call` in the stdlib, the 17 `lang/` packages, the 13
examples, or the 2,249 no-stdlib testdata files **[M, `bidi-ovl`]**.

The resolution order for `f(args)` with candidates C:

1. Label binding filter.
2. If one candidate remains: instantiate it and check the arguments against
   its parameters. This is the common case, with no speculation.
3. If several remain: **synthesize** each non-postponed argument once (no
   expectation). Literals stay literal vars, and closures and `.Case`
   arguments count as "compatible with any function or enum parameter". Keep
   the candidates where every argument `can_subsume` its parameter. This is a
   trial: unification is rolled back with a snapshot of the var table, which
   is cheap because it is body-local.
4. If several still remain: prefer exact matches over promotion, then
   parameter types equal to the literal defaults (Swift's literal scoring,
   reduced to a tie-break), then extension specificity.
5. If several still remain: E103, listing the candidates with their
   signatures.
6. Once one candidate is chosen, *re-use* the synthesized argument types:
   `subsume` each of them into its parameter. Postponed arguments (closures)
   are then checked against the instantiated parameter types.

**No backtracking beyond step 3's trial unifications.** There is no search
over combinations of nested calls. A nested call's type is synthesized once,
before the outer call is resolved. That is the price compared with Swift. The
example it rejects is `f(g(1))`, where *both* `f` and `g` are type-overloaded
and only the combination is unambiguous. §13.4 measures zero
type-overload-selected calls in `lang/`, so this cannot occur in today's code
there.

**Options:** (a) Swift disjunctions with scoring; (b) the recommended
labels → applicability → specificity → synthesized types → tie-break;
(c) forbid type-based overloads entirely, i.e. make cross-module same-label
collisions an error at the call site. (c) is a reasonable *language* decision.
It costs every site counted in §13.4 and removes step 3 completely. (b) keeps
today's language unchanged.

---

## 6. Unknown receivers (D3)

### 6.1 How often a receiver is unknown at its use

**Method.** The `bidi-recv` probe (§15.4) runs inside today's generator. With
statement regions on, just before each `MethodCall`, `ProtocolCall`
(operator or sugar) or `Field` lookup is emitted, it:

1. solves everything generated so far;
2. applies Rule L and ref-decay eagerly, as a walker would have;
3. classifies the receiver.

An unknown receiver is traced back through locals' initializers, receivers,
callees and arguments to its first unknown cause **[M]**:

| Receiver at lookup time | stdlib | 17 `lang/` pkgs + 13 examples (beyond stdlib) | Design outcome |
|---|---:|---:|---|
| known head | 10,717 | 25,211 | proceeds |
| literal var (`7.abs()`, `"a" + s`, `1 << n`) | 129 | 481 | Rule R (method) or L/C (operator) |
| unknown ← rooted in a literal (operator chain on a literal) | 18 | 54 | known once Rule L/C apply at synth |
| unknown ← closure parameter (directly or via a local) | 286 | 84 | known: closures are checked against their expectation (§8) |
| unknown ← desugaring temp (`$dsi` interpolation accumulator, `$iter`) | 10 | 939 | known: an interpolation is a literal checked in place |
| unknown ← call/member waiting on a literal argument (`state(0) + a`, `line.chars(checked: 0)`) | 3 | 47 | known: overload step 3 decides by literal compatibility |
| unknown ← pattern binder (from the row above) | 0 | 2 | known: the scrutinee is synthesized first |
| **genuinely unknown** (needs a later statement) | **0** | **0** | would be E106 |
| total sites | 11,163 | 26,818 | |

**Caveat [R].** The "known by construction" verdicts are reasoning about the
design, not a run of it. In particular "closure parameter → known" assumes
the parameter's type is fixed by the receiver or by a non-closure argument
(two-phase, §8). With `func f[T](g: (T) -> ())` and no other source of `T`,
the parameter is a general var. Passing it *as an argument* inside the body
still works (`f(g: { (x) in takesInt(x) })` unifies `T := Int64`). Using it
*as a receiver* before anything determines it (`{ (x) in x.count }`) is
E106. Today the same receiver use defers forever and ends as "could not
infer" [R].

The classes are:

- **known**: the checker proceeds.
- **literal**: handled by Rule R, or by Rule L for operators.
- **closure parameter**: the prototype's generator emits closure bodies
  *before* the call that supplies their expected type. The design checks
  closures after their expectation is known (§8), so these are known in the
  design.
- **pattern binder**: the binder's type comes from a deferred `ImplicitPat`.
  The design checks patterns against an already-synthesized scrutinee, so
  these are known.
- **genuinely unknown** (a local, call result or field still an open var after
  solving everything to its left): **these are the E106 sites**, i.e. the
  breakage of D3.

### 6.2 Options for D3

| Option | Behaviour on a genuinely unknown receiver | Cost |
|---|---|---|
| **Error with fix-it (Rust E0282)** (recommended) | "the type of `xs` must be known here; annotate `let xs: [T] = …`" | 0 sites in the stdlib, `lang/`, examples and no-stdlib testdata **[M]**; the stdlib-using testdata was not traced |
| Defer the member and retry at region end | accepts receivers resolved by a *later part of the same statement* | re-introduces a (statement-bounded) deferral loop; useful only for same-statement cases; §13.2 found none |
| Defer to body end (today) | accepts anything eventually resolvable | the relax loop, stall breakers and order-dependent decay sets |

---

## 7. Where clauses and entailment (D4)

### 7.1 Today: five evaluators, three answers to "unknown"

| Evaluator | Crate | Representation | Equality clauses | Unknown or abstract position |
|---|---|---|---|---|
| `conformance::extension_bounds_hold` / `type_satisfies` | type-infer | `HirTy` | **skipped as satisfied** (conformance.rs:352, the N2 cause) | **permit** (`Infer`/`Error`/`Param` → `true`, the N1 cause) |
| `solver::extension_where_clauses_satisfied` | type-infer | `TyKind`, reified to `HirTy` via `reify_tv` (unresolved → `HirTy::Error`) | inherits the skip | inherits the permit |
| `solver::receiver_conforms_to_protocol_concretely` + `extension_type_args_compatible` | type-infer | `TyKind` | — | "truly unresolved — allow" |
| `conformance_completeness::protocol_default_method_matches` / `extension_clauses_entailed` | analyze | `WhereClause` vs a context | `entailment.rs`: equal only if syntactically the same | **reject** (abstract subject → `false`) |
| mono `witness_constraints_hold` | kestrel-mir | `MirTy` / `WhereConstraint`, lowered **from the AST** by `function_sig.rs::lower_where_constraint` | **dropped** at lowering (`Equality { .. } => {}`) | **permit** (an unbound param → `true`) |

The same clause gets a different verdict depending on who asks and when (G14).

### 7.2 The kernel

```rust
// crate kestrel-entail — a dependency-graph leaf (kestrel-hecs only), like kestrel-copy-fold
pub enum Entail { Holds, Fails(Why), Unknown(Blocker) }
pub enum Goal<T> { Conforms(T, ProtoRef<T>), NotConforms(T, Entity), Equal(T, T), ProjEq(T, Entity /*assoc*/, T) }

pub trait EntailLayer {
    type Ty: Clone;
    fn view(&self, t: &Self::Ty) -> Head<Self::Ty>;     // Nominal{entity,args} | Param(e) | Proj{base,assoc}
                                                         // | Var (→ Unknown) | Structural(..) | Poison
    fn assumptions(&self) -> &[Goal<Self::Ty>];          // Γ: the asking site's where clauses + param bounds
    fn conformance_sources(&self, nominal: Entity, proto: Entity) -> Vec<Source<Self::Ty>>;  // decl | extension{target_args, clauses}
    fn assoc_binding(&self, nominal: &Self::Ty, assoc: Entity) -> Option<Self::Ty>;
    fn copy_semantics(&self, t: &Self::Ty) -> CopySemantics;   // routes Copyable/Cloneable through kestrel-copy-fold
    fn same(&self, a: &Self::Ty, b: &Self::Ty) -> Tri;         // structural equality, Var → Unknown
}
pub fn entails<L: EntailLayer>(l: &L, g: &Goal<L::Ty>, depth: u32) -> Entail;
pub fn more_specific<L: EntailLayer>(l: &L, a: &Source<L::Ty>, b: &Source<L::Ty>) -> bool;  // shared with mono
```

The semantics are a three-valued AND/OR over conformance sources: `Fails` only
when every source fails; `Holds` as soon as one source holds; otherwise
`Unknown`.

- **`Var` → `Unknown`.** Unknown is *never* "permit".
- **`Poison` → `Holds`.** This means "already reported". It is suppressed, but
  marked so it is never cached as a real answer.
- **`Param(p)` holds iff Γ entails it.** Γ includes the param's declared
  bounds and refinement closure, from today's `entailment.rs`. "Abstract →
  permit" disappears, because every caller now supplies a real Γ.
- **Equality goals:** `same(a, b)`, recursing into arguments. For `ProjEq`,
  reduce the projection with `assoc_binding` and compare.

The adapters:

- **HIR adapter** (`kestrel-semantics`). Used by analyze and conformance
  completeness. Γ comes from the conformance site. This replaces
  `type_satisfies` and `entailment.rs`.
- **Solver adapter** (`kestrel-type-infer`). `view` on an open var returns
  `Var`, and therefore `Unknown`. Γ is the body owner's `WhereClausesOf`. This
  replaces `extension_where_clauses_satisfied`, `reify_tv`'s Error trick and
  `receiver_conforms_to_protocol_concretely`.
- **MIR adapter** (`kestrel-mir` mono). Everything is ground, so `Unknown` is
  unreachable and is asserted to be. **Data change:** MIR's `WhereConstraint`
  gains `Equal`/`ProjEq` and projection subjects, lowered from the front end's
  resolved `WhereClausesOf` (not re-resolved from the AST in
  `function_sig.rs`). That removes the fifth evaluator's private name
  resolution too.

**Why a leaf crate.** `kestrel-mir` depends only on `kestrel-copy-fold`,
`kestrel-hecs`, `kestrel-span` and `kestrel-debug`; the measured `Cargo.toml`
graph is in §15.2. Putting the kernel in `kestrel-semantics` or
`kestrel-type-infer` would pull the front end into mono's build closure.
`kestrel-copy-fold` is the precedent: "ONE kernel, N data sources", with 5
`CopyLayer` impls today.

**Where `Unknown` goes, by caller:**

- **Checker:** member applicability (§5.1 step 4), obligations (re-queued),
  and the end of the body (error E107 "cannot prove `T: P`, because the type
  of `x` was never determined").
- **Analyzer and completeness:** Γ is always complete, so `Unknown` can only
  come from Poison, and is suppressed.
- **Mono:** unreachable, asserted.

**Options for D4:**

- (a) Patch the five sites so they agree. That is how G14, G17 and G25 have
  been worked so far, and the evaluators still disagree after three landed
  stages (fragility-audit.md G14).
- (b) **One kernel and adapters** (recommended).
- (c) One HIR-only evaluator, with the solver reifying to HIR. This keeps the
  `reify_tv` Error/Unknown confusion: HIR has no way to say "unknown var"
  except `Infer`, and today `Infer` reads as a permit.

---

## 8. Closures (D6)

- **Checking against a function type E** (after `structurally_resolve`, which
  only needs the head *Function*):
  - **Arity:** explicit parameters must match. `it` requires arity 1
    (E108 otherwise). A parameterless closure adapts to any arity and ignores
    the arguments. That is today's `closure_flex`, now a check rule.
  - **Parameters:** each annotated parameter `A_i` must equal `E.param_i`
    (no subsumption on parameters; contravariance is not worth it). An
    unannotated parameter *takes* `E.param_i`, even when that is still an open
    var.
  - **Conventions and kind:** taken from E. A `mutating` parameter upgrades
    the convention. The closure literal is **built at E's kind**: normal,
    `mutating`, `consuming` or `escaping`. That removes
    `reconcile_fn_kinds`' retrofit, `kind_flex` and `closure_literal_exprs`.
  - **Body:** `check(tail, E.ret)`. `return e` inside the body checks against
    `E.ret`.
- **Synth mode** (`let f = { (x: Int64) in x + 1 }`): annotated parameters are
  used. Unannotated parameters get general vars, which are body-scoped like
  any other general var (§3.2). So `let f = { (x) in x }; f(1)` types `x`
  from the later call. A var still open at the end of the body is E105
  "cannot infer the type of closure parameter `x`", reported at the closure,
  which is the same verdict as today's `cannot_infer_without_context_error.ks`
  test. Using the parameter as a *receiver* inside the body before anything
  determines it is E106. The kind is `normal`.
- **Closures passed to generic functions.** In `xs.map(as: { it * 2 })` with
  `map[U](as f: (Item) -> U)`, `Item` comes from the receiver and `U` is a
  fresh general var. The closure is checked against `(Int8) -> α`: `it: Int8`,
  the body synthesizes `Int8` (Rule L does not apply because `it` is not a
  literal, so `2` is checked against `Int8`), and `α := Int8` **[M-toy]**.
- **Postponed closure arguments** (Rust's two-pass `check_argument_types`,
  Kotlin's postponed lambdas):
  - Phase 1 checks the non-closure arguments, so `apply(with: { it + 1 }, to: 5)`
    learns `T` from `5` first.
  - Phase 2 checks closures in source order, each against its parameter type
    zonked at that moment.
  - The result is `a : Int64` **[M-toy]**. Today this works through the global
    solver **[M]**.
  - Measured need: closure arguments that are *not* last, in calls where a
    later argument supplies a type param the closure's parameters need. This
    is rare in `lang/` [R]; the stdlib convention is closure-last, trailing.
- **Trailing closures** are the last argument; nothing changes.

---

## 9. Promotions and coercions (D7)

All coercions are decided by `subsume(T ≤ E)`, tried in this order:

1. `unify(T, E)`: equality, the common case.
2. **Never:** `!` ≤ anything. A var that only ever met `!` falls back to `!`
   at the end of its region, because the fallback is a default (§3.2). Today
   this is `never_fallback_targets`, applied at the end of the body; in the
   design it is a flag on the var.
3. **Ref decay:** `&T ≤ E` with E not a ref → `T ≤ E` (record copy-out).
   `&mutating T ≤ &T`. A value `T ≤ &T` only in return position (implicit
   borrow; the escape checker judges it).
4. **Promotion:** `E: FromValue[X]` and `T ≤ X` → record a promotion at this
   expression. This covers `Optional[T]` from `T` and `Result[T, E]` from `T`.
5. **Existential:** E is a protocol (or `Self`) and `T: E` → `Holds`.
6. **Closure kind passing table** (closures.md): normal → `mutating` via
   adapter, `escaping` → normal, and so on, as data.
7. Otherwise the error is "expected E (because R), found T".

Because `check` carries E into arms, block tails, elements, closure bodies,
`return` and arguments, **step 4 happens at the leaf**. So these four become
legal, uniformly:

- `let x: Int64? = if c { 5 } else { .None }`
- `[1, .None, 3]` against `[Int64?]`
- `func opt(c) -> Int64? { if c { 5 } else { .None } }`
- the tail `n * 2` of a `throws` function

All four are rejected today **[M]**; three are verified **[M-toy]**.

**Language decision required.** `docs/language/error-handling.md` says a bare
success value in the tail of a throwing function is a type mismatch, "with one
exception: an annotated binding promotes". That exception is an artefact of
which constraint kind the binding got. The options are:

| Option | Rule | Cost |
|---|---|---|
| (i) | `FromValue` promotes at **every** check leaf (recommended; the library defines `FromValue`, so the checker should not special-case `Result`) | doc change; behaviour becomes more permissive only |
| (ii) | Promote `Optional` everywhere, `Result` nowhere | `let r: Int64 throws E = 42` becomes an error; `.Ok` is always explicit |

Both are position-independent. Today's behaviour is neither.

**Ref decay.** The five decay registration sites (the four sets `scrutinee_exprs`,
`binding_init_exprs`, `assign_target_exprs`, `always_decay_exprs`, plus the
return-tail site) become
step 3 of `subsume`, together with one synth-mode rule: *an unannotated `let`
decays*, because a binding is a value context. The AGENTS.md invariant ("every
position where a ref-returning call's value is consumed as its pointee must
record its expr id") disappears, because positions no longer need registering.

**`throws`, `try`, `??`.**

- `try e`: `synth(e)` gives `Result[T, Er]`, `Optional[T]` or any `Tryable`.
  The type of the expression is `T`, and an obligation is queued:
  `ReturnType: FromResidual[Residual(e)]`. `try` requires the head of `e`
  (structural resolution).
- `a ?? b`: synthesize `a`, then `structurally_resolve`, then check `b`
  against `Wrapped(a)` (through `Coalesce`).
- `throw e`: `check(e, ErrorType(ReturnType))`; the type is `!`.

**`some P` returns.** Inside the defining body the return type is a general
var with bound obligations, and the first `return` fixes it (as today, minus
the deferral). Callers see `Opaque{origin, bounds}`.

---

## 10. Error recovery and diagnostics (D8)

**Poisoning.**

- `TyKind::Error` stays the absorber, and `report_error` stays the only way to
  create it.
- The checker's rule: **an expression whose synthesized type contains Error
  produces no further diagnostics about its own type**, at any parent.
  Parents still check *their other* children. This replaces
  `errored_coerce_exprs`, `poison_protocol_call_recv_on_failure` and the
  per-span dedup in `report_unresolved_slots` with one structural rule.
- `3 + true` gives one error ("`Int64` has no `+` taking `Bool`"), not today's
  contradictory pair **[M]**.
- A *statement* is the cascade boundary. An error inside statement *k* poisons
  only the bindings it defines; later statements are checked normally, against
  `Error`-typed locals, which absorb.

**Deterministic order.**

- Diagnostics are accumulated with `(span.start, emission seq)` and sorted
  before they leave the query.
- `TypedBody` maps that are iterated to produce output become `IndexMap` or
  are sorted.
- The current out-of-order emission (lines 4, 8, 5, 7, 7, 6 in one probe
  **[M]**) is solver-queue order. In a walker, emission order *is* source
  order, nearly for free; the sort is a backstop.
- Today even the *set* of diagnostics is nondeterministic. Two testdata files
  produce 1 or 2 copies of "could not infer type" from run to run of the same
  binary **[M]**. In the design, "cannot infer" errors are emitted once per
  var, at the var's origin, while walking. Nothing iterates a hash map to
  find them.
- Cross-body ordering is the driver's job: sort by file, then offset.

**No `?` or `Error` in messages.**

- The renderer takes resolved types.
- An open var prints `_`, and only inside "cannot infer" / "add an
  annotation" messages.
- A type containing `Error` suppresses the diagnostic.
- Literal vars print as "integer literal".
- N5 becomes E105 "cannot infer type parameter `T` of `none`", with
  help "write `none[Int64]()` or annotate `let a: Int64? = …`" **[M-toy]**.

**Expected versus found, with the reason.** Every `check` carries
`Expectation { ty, reason }`, where `reason` is one of:

- `Annotation(span)`
- `Param { callee, index, decl_span }`
- `Return(decl_span)`
- `OperatorOperand { op, other_span }`
- `FirstArm(span)`
- `Element(first_span)`
- `Assign(target_span)`
- `ClosureReturn(span)`
- `Condition`

Rendering: the primary label is "expected `Int8`, found `Int64`", and the
secondary label at the reason's span says "expected because of this
parameter". This is the Rust/Elm pattern.

**Error codes.** `docs/error-codes.md` has E102–E109 and E113–E120 free. A
proposed allocation:

| Code | Error |
|---|---|
| E102 | no member / unsupported operator |
| E103 | ambiguous overload, conformance or extension |
| E104 | cannot infer enum for `.Case` |
| E105 | cannot infer type parameter or closure parameter |
| E106 | type must be known here (unknown receiver) |
| E107 | unsatisfied where clause / conformance |
| E108 | closure arity / `it` |
| E109 | type mismatch (expected / found / because) |
| E113 | wrong labels / arity |
| E114 | literal not expressible as `T` |

E101 (condition must be `Bool`) and E121 (out of range) stay. E100 is retired, or kept as an alias for one
release. The AGENTS.md "three-file" rule for new `InferError` variants is
unchanged; only the code strings change.

---

## 11. Interaction with the rest of the compiler

### 11.1 Output contract

`TypedBody` is consumed by `kestrel-analyze` (3 files), `kestrel-mir-lower`
(1), `kestrel-compiler` (1) and `kestrel-lsp` (5). The fields read outside the
crate are `resolutions` (34 reads), `expr_types` (29), `errors` (11),
`local_types` (7), `kind_coercions` (2) and `promotions` (1) **[M grep]**.
`type_args`, `field_subscripts`, `indirection_peels` and
`opaque_concrete_type` are read through accessors in mir-lower.

The new checker **produces the same struct**. This is what makes P3/P4
incremental and differential-testable: run old and new on the corpus, then
diff `expr_types`, `resolutions`, `promotions` and `type_args` per body.

One semantic addition: `promotions` and decays become *complete*. Today a
promotion inside an arm cannot exist, so MIR never lowers one there.

- mir-lower's `lower_expr` already applies `apply_promotion(expr_id, …)` to
  *every* expression it lowers (`kestrel-mir-lower/src/body/expr.rs:32`),
  keyed by expr id **[M grep]**.
- Lowering paths that bypass `lower_expr` (the `lower_expr_no_promote` and
  tail/move variants) must be audited in P3, so that an arm-tail or element
  promotion is not dropped [R].

### 11.2 Queries and incrementality

- `InferBody { entity, root }` stays the unit of memoization. Bodies still
  never read other bodies' `InferBody`. An omitted return type becomes
  exactly `()` inside the body as well (the tail is checked against unit).
  Today it is a fresh var inside and unit to callers. No body in the measured
  corpus relies on the difference **[M]** (§13.5).
- The checker's dependencies are the same signature-level queries as today
  (`LowerCallableTypes`, `ExtensionsFor`, `ConformingProtocolInstantiations`,
  `WhereClausesOf`), plus `Entails` for ground goals. That one *can* be a
  memoized query (`EntailsGround { ty: HirTy, goal }`), shared by analyze and
  checker. Today `type_satisfies` is re-run uncached.
- Rule C's candidate set comes from the expectation, not from a global
  conformer list. That avoids a "body depends on every conformance in the
  program" edge.

### 11.3 LSP: hover, inlay hints, completion

- **Completion on partial code** benefits most. In `foo.|`, the receiver is
  *synthesized before* the (missing) member is looked up, so its type is in
  `expr_types` no matter what fails later in the statement. Today a later
  stall or poison can leave the receiver `Error` or unresolved, and completion
  falls back to the CST locator (`completion.rs::member_completion`).
- **Inlay hints** for `let x = 100` show `Int64` at the end of the statement
  and never change because of later code: the hint *is* the rule.
- **Signature help** can show the expected parameter, because `check` knows
  which parameter is being filled.

### 11.4 Performance

Today:

- up to 256 fixpoint rounds × the full constraint list;
- the relax loop re-scans every constraint per level;
- `apply_literal_defaults` scans every TyVar slot on every iteration.

The checker:

- is one walk;
- flushes obligations per region (small, local queues);
- uses snapshot-and-rollback for trial unification only in multi-candidate
  overload steps.

The expected effect is a large constant-factor reduction in inference time,
and more importantly no superlinear tail on big bodies [R]. Measure in P3 with
`kestrel dump diagnostics` timings on the stdlib. A stdlib-including file
takes about 4.2 s end to end today **[M]**; how much of that is inference is
not measured.

---

## 12. Migration plan (D5)

An incremental *strangler*. The crate keeps its name, its query and its
output. Each phase is shippable and gated by:

- **Gate 1:** the full `.ks` corpus through `/triage`: zero verdict changes,
  except the pinned tests the phase targets.
- **Gate 2:** for P3/P4, a **differential harness**: old and new checker on
  all corpus bodies, diffing `TypedBody` (normalized: resolved types and
  entities, sorted).
- **Gate 3:** `lang/` packages and examples build and their tests run. The
  prototype's harness (§15.4) already does this for diagnostics in about 10
  minutes.

| Phase | Content | Fixes | Size [R] | Risk |
|---|---|---|---|---|
| **P0** | Sort diagnostics. Specific error codes. `Expectation.reason` threaded through the existing `Coerce`/`Equal` (they gain a reason field). Never print `?`/`Error`. | N6, N7 (most), E100 | 0.8–1.2k | low |
| **P1** | `kestrel-entail` crate + 3 adapters. Route the 5 evaluators through it. MIR `WhereConstraint` gains equality/projection, lowered from `WhereClausesOf`. In the solver, Unknown re-queues `Conforms` instead of solving it. | N1, N2, G14, G25 residue | 2–3k (kernel ~0.8k, adapters 3×0.3k, deletions) | medium. Mono changes; the stdlib has the 5 `extend Iterator where <assoc>: P` sites (G14). |
| **P2** | Statement regions in today's solver: fixpoint + literal defaulting per statement (the prototype, cleaned). Rule L. 9 stdlib annotations. Rule C replaces the shape projection. | N3, N4 | 0.3–0.6k | low–medium. Measured: 0 `lang/` regressions, 4 testdata files [M] (§13.6). |
| **P3** | Convert `generate.rs` to `check.rs` **form by form**, solving as it walks:<br>(a) receivers: `MethodCall`/`Field`/`ProtocolCall` call `structurally_resolve` and resolve the member *immediately* when the head is known; the `Member` constraint is emitted only when it is not<br>(b) calls: instantiate + check args against params (two-phase)<br>(c) closures in check mode<br>(d) `if`/`match`/block/array/tuple expectations<br>(e) literals in check mode<br>(f) patterns against synthesized scrutinees | position-independent promotion, E106, better spans | 4–6k changed | **high**: largest surface; the differential harness is essential |
| **P4** | Delete what P3 made dead: the decay sets, `closure_flex`/`closure_it`/`kind_flex`/`closure_literal_exprs`, `pattern_binder_*`, `break_stalled_assign_targets`, the relax loop, `apply_operator_shape_projections`, the context-literal pass, the `Member`/`Call`/`OverloadedCall`/`Implicit*`/`TupleIndex`/`TupleRestPat`/`EqualDecayed`/`AssignTarget`/`BorrowPointee`/`InterpolationLink` constraints. | — | −4 to −6k | medium. Each deletion is gated by a "never fires" `ktrace` count over the corpus first (the G26 method). |
| **P5** | The solver becomes `obligations.rs`: a queue of `Conforms`/`ProjEq`/`Equal` (where-clause) goals, flushed per region via the kernel. `solve()` disappears. | — | net −1k | low |

**Can it be done incrementally?** Yes, and the prototype is the evidence. The
existing solver tolerated being driven *during* generation: `fixpoint` plus
literal defaulting at every statement boundary. It produced zero new
diagnostics across all 17 `lang/` packages and 13 examples beyond the 9
predicted stdlib sites **[M]**. P3 is the same move at finer grain: per member
access instead of per statement.

**The corpus as safety net.**

- Today `/triage` covers about 3,716 testdata files plus the stdlib.
- The harness in §15.4 adds `lang/` packages and examples, using `dump
  diagnostics` and sorted diagnostic sets. This works around G23, the
  testdata harness hiding diagnostics anchored in the stdlib.
- Recommendation: promote the harness into `triage` as a "lang" lane before
  P2.

**What should *not* be done incrementally:** the decision of the language rules
(D2, D3, D7(i/ii)). These need a maintainer ruling *before* P2 and P3, because
they change which programs are legal.

---

## 13. Breakage inventory

### 13.1 Literal rule (D2 + Rules L/C)

**[M]** Prototype `bidi-stmt,bidi-op` against baseline, diffing sorted
diagnostic sets:

- **stdlib: 14 new errors in 5 functions, from 9 `let` bindings.** All of
  them are hash and PRNG constants that today type as `UInt64` through
  back-flow:
  - `collections/hashing.ks` `DefaultHasher.write` (`let mult`) and `.finish`
    (`let m1`, `let m2`);
  - `memory/pointer.ks` `RawPointer.hash` and `Pointer.hash` (`m1`, `m2` each);
  - `numeric/random.ks` `Lcg64.nextUInt64` (`let a`, `let c`).

  Six of the nine bindings (three copies each of `m1 = 18397679294719823053`
  and `m2 = 14181476777654086739`, the Murmur3 finalizer constants) **do not
  fit in Int64**. Under the rule they get 6 E121 errors on top of the 8
  mismatches. The fix is the annotation
  `let m1: UInt64 = …`. The `bidi-n4` probe independently counts exactly these
  9 as the only non-default literal `let`s in the stdlib (out of 26 numeric
  literal `let`s).
- **17 `lang/` packages: 0.** Grep finds no unannotated numeric-literal `let`
  outside `lang/std` at all.
- **13 examples: 0.**
- **Testdata:** see §13.6.

### 13.2 Unknown receivers (D3)

**0** in the stdlib, the 17 `lang/` packages and the 13 examples, out of
37,981 member, field and operator sites **[M]**; the table is in §6.1. The
no-stdlib testdata contributes 1,151 sites: 1,149 known and 2 already `Error`.

### 13.3 Bindings whose type is still open at the end of their statement (D2: body- vs statement-scoped general vars)

With statement regions on, `bidi-csid` reports every unannotated `let`/`var`
whose type still contains an unknown after its statement was solved and its
literals defaulted **[M]**:

| Shape | `lang/` + examples | stdlib |
|---|---:|---:|
| `Array[_]` from `var x = []` | 45 | 0 |
| `Dictionary[K, V, _]`: only the hasher type param open; its default `H = DefaultHasher` is applied at body end today | 15 | 0 |
| `Dictionary[_, _, _]` from `[:]` | 1 | 0 |

All 46 element-type cases need **body-scoped** general variables. This is why
D2 scopes only *literals* to the statement. Pure Swift-style per-statement
inference would reject all 46. The 15 hasher cases are type-parameter
*defaults*. The design applies those at region end, like literal defaults
[R]. That changes behaviour only if a later statement pins `H` to something
other than the default, which was not measured.

### 13.4 Type-directed free-function overloads

**0** calls reached the argument-type stage of `solve_overloaded_call`
(`bidi-ovl`) in the stdlib, `lang/`, examples or no-stdlib testdata **[M]**.

### 13.5 Omitted return types with a tail value

**0** bodies in the stdlib, `lang/`, examples or no-stdlib testdata have an
omitted return type whose tail actually flows a value into the fresh return
var **[M, `bidi-ret`]**. The probe also flags opaque `some P` returns,
associated-type returns and non-exhaustive tails, because they share the
fresh-var shape. Every flagged case was one of those three on inspection.
So the design can type an omitted return as exactly `()` and check the tail
against it. There is no need to preserve "value tail silently discarded".

### 13.6 Testdata (full corpus A/B)

**[M]** All 3,716 testdata files, each compiled with
`dump diagnostics` under the baseline and under `bidi-stmt,bidi-op` (the
stdlib-using ones against `std_patched`). The comparison is on sorted
non-stdlib diagnostic sets.

Every unit that differed was re-run 3–6× per config (`confirm.py`), because
today's output is itself nondeterministic (§10). Units whose baseline
reproduces the proto outcome are discarded as flakes. Two no-stdlib units
were discarded this way, both `cannot_infer_*` closure tests.

| Stable change | Files | Verdict |
|---|---:|---|
| Pinned N3 test `inference/literal_left_operand_adopts_right_operand_type.ks` | 1 | **fixed** (both `E100`s gone; it now compiles) |
| N4-class: a literal `let` fixed by a *later* statement. `let cp = 65; Char(unchecked: cp)` ×4 in `stdlib/char/char_init_value_and_value.ks`; `let arr = [1, 2, 3]; let x: [lang.i64] = arr` in `inference/mod/infer_array_from_elements.ks`; `let x = 2; x` returned as `lang.i64` in `statements/missing_semicolon/call_then_let_requires_semicolon.ks` | 3 | **new error**: the intended breakage. Fixed by annotating the `let` |
| N4-class, error *moves*: `inference/mod/infer_array_element_type_mismatch.ks`. The mismatch was reported at the literal `[1, 2, 3]` (line 7) and is now reported where the later statement uses it (line 8) | 1 | still an error; the test's `// ERROR` line pins today's position |
| Cascade or wording improvements: `expressions/calls/method_calls/static_method_on_type_param_as_value.ks` ("no method 'init' on type 'Error'" → "… on type 'T'"); `types/type_operators/result_operator/optional_result_precedence.ks` (2 cascade errors gone; a `// skip:` test); `patterns/.../tuple_arity_mismatch.ks` (a cascade printing `(Error, Error)` gone) | 3 | better |
| everything else (3,709 files) | — | identical |

Two units that deliberately shadow the prelude
(`builtins/literal_protocols/_prelude_literal_protocols.ks`,
`declarations/modules/user_module_named_int64_does_not_break_stdlib.ks`)
have their stdlib-anchored error sets shift (about 80 errors in baseline, 35
and 55 changed). The test harness discards stdlib-anchored diagnostics (G23),
and neither file's verdict depends on them.

**Total cost of the literal rule** across stdlib, `lang/`, examples and
testdata:

- 9 stdlib `let`s;
- 4 testdata files (3 new errors, 1 moved error);
- 0 elsewhere.

It also fixes 1 pinned test. Every breakage is the N4 shape, and the fix is
the same one-token annotation.

---

## 14. Risks

1. **Hidden reliance on back-flow beyond literals.** §13 measures literals,
   receivers and open `let`s. Other flows also run "backwards" today:
   - closure-parameter types fixed by a later call (`let f = { (x) in … }; f(1)`);
   - `.Case` resolved by later use;
   - ref-decay races.

   Mitigation: the P3 differential harness surfaces every changed body. Only
   2 testdata files use the closure shape, and both expect an error **[M grep]**.
2. **Mono and analyze changes in P1.** The kernel changes mono's answers when
   they were "permit on an unbound param". Some of those permits were masking
   G14-class bugs that the stdlib currently relies on (the
   `extend Iterator where Item: …` sites). Mitigation: the G14 plan's corpus
   probes; land the solver adapter first, mono last.
3. **Rule C candidate set too small.** If an expected type hides the operand
   type behind an alias or projection (`let r: MyRange = 1..=9` with
   `type MyRange = ClosedRange[Int16]`), the candidate set must be computed
   *after* alias reduction. Specify "mentioned heads of the fully reduced E".
4. **Promotion everywhere changes overload and arm typing.** With promotion
   at leaves, `if c { x } else { .None }` against `T??` could promote
   differently. Exact-first ordering in `subsume` keeps today's answers
   wherever today accepts. New acceptances only [R].
5. **P3 is large, and two checkers coexist.** Mitigation: convert per HIR
   form behind the same `TypedBody`, keep the old path as a fallback per form
   until the differential harness is clean, and gate every form on the full
   corpus.
6. **Rule R (`7.abs()` into `Int32`) frustrates users.** It is rejected today
   too **[M]**. A future extension could treat literal receivers of members
   declared on a literal protocol's extension the way Rule C does. Not
   proposed now.
7. **Diagnostic wording is pinned by about 850 testdata annotations.**
   Codes are not the problem: 175 annotations pin a code (`// ERROR(E494)`,
   …) and **none** pins E100 **[M grep]**. So moving inference errors to
   specific codes breaks nothing. But the 850 `// ERROR:` annotations match
   message *text*, and some of that text is today's solver vocabulary:
   "type mismatch" (25), "no matching overload" (14), "does not conform to
   protocol" (10), "`!: Show`" (9). P0 must keep these substrings, for
   example "type mismatch: expected X, found Y", or else get a
   maintainer-approved mechanical annotation rewrite. CLAUDE.md forbids
   editing tests to make them pass.

---

## 15. Appendix: evidence

### 15.1 Probes

Source: `scratchpad/bidi/probes/*.ks`. Run with
`python3 run_probes.py <names>`, which compiles with `target/release/kestrel
build` and runs the binary if one is produced. Baseline =
`ae1aeeda` + inert hooks.

The results are the table in §1. Relevant raw outputs:

- `n4`: `[ran] exit=0 out='w=-56'`
- `n3`: `error[E100]: type mismatch … expected Int64 got Int8`
- `n5`: `? !: Copyable`
- `diag_order`: errors at lines 4, 8, 5, 7, 7, 6
- `closure_order`: `6 6 3`
- `empty_arr`: `1 1`

### 15.2 Greps (`ae1aeeda`)

- Unannotated empty-array `let`/`var` (`(let|var) x = [];`): **47** in `lang/`
  (flock 27, clutch 7, http 4, perch 3, talon-sqlite 3, swoop 2, jessup 1), 0
  in `lang/std`, 0 in testdata.
- Unannotated numeric-literal `let`: 30 grep lines, all in `lang/std`.
- Closures with unannotated parameters `{ (x) in`: 119 in `lang/` + examples,
  against 16 with annotated parameters. `{ … it … }` closures: 78.
- Unannotated `let f = <closure>` with unannotated parameters: 2, both in
  testdata, both expecting "could not infer type".
- Heterogeneous operator conformances: `String`, `StringSlice`
  (`Equal`/`NotEqual` × 2), `Instant` (`Subtractable` × 2), `Duration`
  (`Multipliable[Int64]`, `Divisible[Int64]`).
- Crate graph (`lib/*/Cargo.toml`): `kestrel-mir` depends on
  `kestrel-copy-fold`, `kestrel-hecs`, `kestrel-span` and `kestrel-debug`
  only.

### 15.3 Toy checker

`scratchpad/bidi/toy` (`cargo test`) is a standalone ~650-line Rust model of
§2–§9 over a miniature type language. It has 9 tests, all passing: N3, N4
(with the "because" note), Rule C (`Int8`, `ClosedRange[Int16]`, `Bool`),
position-independent promotion, `[]` element inference, two-phase closures,
heterogeneous operators (`Instant - Instant`, `Instant - Duration`,
`Duration * 2`, `String == literal`), N5's named error, and Rule R.

### 15.4 Prototype hooks in the real compiler

`lib/kestrel-type-infer/src/bidi_probe.rs` (local branch only), wired from
`generate.rs` (statement wrapper, closure and interpolation depth) and
`solver.rs`. Each hook is inert unless its `KESTREL_DEBUG` category is set:

| Category | Kind | Effect |
|---|---|---|
| `bidi-stmt` | behaviour | statement regions: at each statement boundary outside closures, run `fixpoint`, the shape projection, Rule L, the context pass and graduated literal defaulting restricted to the statement's vars, then ref-decay defaults |
| `bidi-op` | behaviour | Rule L |
| `bidi-n4` | measurement | reports non-default literal `let`s |
| `bidi-recv` | measurement | `fixpoint` before each member emission, then classify the receiver |
| `bidi-csid` | measurement | unannotated `let`s still open at the end of their statement |
| `bidi-ovl` | measurement | free-function calls that needed type-based overload selection |
| `bidi-ret` | measurement | omitted return type with a value tail |

Harness: `scratchpad/bidi/corpus.py <config> <KESTREL_DEBUG> <all|lang|std|nostd>`.

- It runs `kestrel [--no-std|--std std_patched] dump diagnostics` on every
  testdata file, on each `lang/` package closure, on each example's package
  closure, and on an empty program (to attribute stdlib output).
- Diagnostics are normalized as `sev[code] msg @ path:line:col | label` and
  compared as **sorted sets**, because of N6.
- `std_patched` is `lang/std` plus the 9 annotations, so that testdata diffs
  isolate testdata effects.
- Supporting scripts:
  - `corpus_inc.py`: the same harness, incremental, writing JSONL.
  - `diff_results.py`: per-unit diff of two result files.
  - `confirm.py`: re-runs each changed unit N× per config and marks it
    `REAL` only when no proto outcome equals any baseline outcome.
  - `analyze_traces.py`: aggregates the `bidi-*` traces, subtracting the
    stdlib's own traces with a multiset difference.
- Cost: one config over all 3,716 testdata files took about 3.5 h on a shared
  4-core container (stdlib-using files take ~4 s each, because every
  invocation re-checks the whole stdlib). `lang/` + examples take about 5
  minutes.
- The prototype patch (`prototype.patch` + `bidi_probe.rs`) and the raw
  results (`results/*.json[l]`) are kept next to this document's scratchpad
  copy.

**Limits of the prototype as evidence.**

- It models D2 (statement-scoped literal defaulting) and Rule L faithfully
  *inside the existing solver*.
- It does **not** implement Rule C, the check-mode walker, two-phase
  arguments, three-valued entailment or leaf promotion. Those claims rest on
  the toy (§15.3) and on reasoning.
- The `bidi-recv` classification of closure parameters, interpolation temps
  and literal-argument calls as "known by construction" is reasoning about
  the design (§6.1 caveat).
