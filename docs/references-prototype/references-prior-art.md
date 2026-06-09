# References — Prior Art (Hylo, Swift, Mojo)

## 1. Why this matters

Kestrel is adding *second-class references* (`&T` / `&mutating T`) that reuse the existing `@owned`/`@guaranteed` convention machinery, infer lifetimes rather than spelling them, and require a referenced value to *escape* (outlive the call) before it can be returned — explicitly deferring Rust-style region inference, stored references, and exclusivity. The design space for "ownership without full Rust lifetimes" has been mapped three times by mainstream-adjacent languages, and two of those maps are near-exact precedents for Kestrel's plan. **Hylo** is the closest: it ships the strongest form of the second-class discipline (references are *accesses*, never values; no type ever carries a lifetime) and proves it is a coherent *end state*, not just a stepping stone. **Swift's `~Escapable` + lifetime-dependency** is the closest mainstream analog to the whole proposal — an escapability default-bound plus a single "where does this return's lifetime come from" annotation, deliberately written instead of inferred. **Mojo** is the contrast case: it went first-class (storable `origin` type-parameters threaded everywhere) and enforces mutable-reference exclusivity — i.e. it paid exactly the two costs Kestrel is choosing to avoid. The throughline across all three: every one of them refuses to *fully infer* return-reference provenance, and exactly one of them (Hylo) refuses to put a lifetime on every type — which is the strategic question for Kestrel's Stage 2 (§6).

---

## 2. Hylo / Val — the closest model

Hylo (formerly Val) is built on **mutable value semantics (MVS)**: values are independent, parts can be mutated in place, and in the discipline's strict form references are *second-class citizens* that "are only created implicitly, at function boundaries, and cannot be stored in variables or object fields." That sentence is, almost verbatim, Kestrel's Stage-1 cut. Hylo expresses borrowing through two mechanisms — parameter conventions and projections — and enforces safety with purely local, flow-sensitive analysis. No named lifetimes appear anywhere.

### 2.1 Parameter conventions = borrow / owned / consume / init

Hylo writes the convention *before the parameter type*. There are four, plus `yielded` for subscripts:

| Convention | Maps to | Mutable? | Exclusive arg? | Escapes callee? | Hylo's own Rust mapping |
|---|---|---|---|---|---|
| `let` (default) | shared borrow | no | exclusive *visibility* during the call | **no** | "pass by immutable borrow, with exactly the same guarantees" |
| `inout` | mutable borrow | yes | **yes — strictly unique** | **no** | "pass by mutable borrow, with exactly the same guarantees" |
| `sink` | consume / move | n/a (owned) | yes (ownership transfer) | **yes** | "pass by move" |
| `set` | initialize uninitialized storage | yes (writes) | yes | yes (leaves it initialized) | placement-new on storage that "starts out uninitialized, and ends up initialized" |

Pinning quotes: `let` "does not transfer ownership of the argument to the callee … a `let` parameter can't be returned" without copying. `inout` arguments "must be unique: they can only be passed to the function in one parameter position," and the callee sees them "as though it had been declared to be a local `var`, with a value that is truly independent from everything else in the program." `sink` "indicates a transfer of ownership, so … the parameter *can* escape the lifetime of the callee." The call site marks mutation with `&` (a *marker*, not C's address-of): `f(&x)` "signals … that the argument is to be mutated."

This is a near-perfect philosophical match to Kestrel's `@owned`/`@guaranteed` model: `let` ≈ `@guaranteed` (shared), `inout` ≈ `@guaranteed` mutable (`BeginMutBorrow`), `sink` ≈ `@owned`, `set` ≈ `@owned`-into-uninitialized. Kestrel already threads `ParamConvention::Borrow/MutBorrow` end-to-end, and references.md confirms reference *parameters* are "nearly free." The lesson is to **adapt the surface (`&T` / `&mutating T`) but keep Kestrel's convention machinery underneath** rather than importing Hylo's keyword set.

### 2.2 References *without* first-class lifetimes — projections and yielding subscripts

The headline mechanism: **a subscript projects rather than returns.** "A subscript does not return a value, it projects one, granting the caller temporary read and/or write access to it." Operationally it "temporarily yields control for the caller to access the yielded value":

```hylo
subscript min(_ x: yielded Int, _ y: yielded Int): Int {
  let   { if y > x { x } else { y } }   // immutable projection
  inout { if y > x { &x } else { &y } } // mutable projection
  sink  { if y > x { x } else { y } }   // consuming
}
```

- A single `yield` bisects the body: code *before* `yield` runs when the projection **starts**, code *after* runs when it **ends**. A `let`/`inout` subscript "must have exactly one yield statement on every terminating execution path."
- `yielded` is a *placeholder* convention that resolves "to either `let`, `inout`, or `sink` depending on the way the subscript is being used."
- A projection defines exclusivity *by construction*: "If a projection `p` projects an object `o` mutably, `o` is inaccessible for the duration of `p`'s lifetime"; immutable projection freezes `o` for `p`'s lifetime.

This is "the notation of access": you never name a reference type or a lifetime. Because the projection is *syntactically scoped*, the compiler always has the source object in lexical view — no lifetime variable needs threading through types. **This is Hylo's direct answer to the exact thing Kestrel's MVP defers**: accessor methods returning borrows, and "borrows-of-locals returns." A yielding accessor needs *no* escape checker and *no* conditional-`@guaranteed`-return surgery (references.md's #1 risk — breaking "every fn returns `@owned`"). Kestrel already has `BeginBorrow`/`EndBorrow` scoping; a yield-accessor would lower to begin-borrow-before / end-borrow-after the *caller's* continuation. It is worth evaluating as a lower-risk substitute for return-refs — though it needs **caller-side scoping support Kestrel does not yet have** (the coroutine split point), so it is "adapt," not a drop-in.

### 2.3 The escape rule, enforced *without* named lifetimes

- **The rule:** a projected value / second-class reference *cannot escape*. Per the maintainers: "you can't return a reference to a part of an object. However, you can project it. This mechanism plays a big role in the elimination of lifetime annotations because a projected value can never escape." In MVS terms, "all values form disjoint topological trees rooted in the program's variables."
- **Enforcement is flow-sensitive *local* analysis, not a region solver.** Hylo "uses a flow-sensitive analysis to track which bindings depend on which values," and the can't-escape rule exists precisely so the compiler can "always reason about lifetime and access in a context where the necessary information is available" — "entirely on the local scope of a function." Binding lifetimes are inferred trivially from scope: "The lifetime of a local binding begins when its initialization is complete and ends after its last use."
- **Two concrete enforcement layers** (note the uncertainty in §8):
  - *Formal* (the JOT "Swiftlet" model): **path uniqueness.** A "path" is a name followed by `.member`/`[index]` accessors. Exclusivity = among mutable args at a call, no two paths overlap, tested via a reflexive-transitive subpath relation `⊆`. `T-CALL` requires `∀ i≠j. aᵢ ⊄ aⱼ ∧ aⱼ ⊄ aᵢ`. So `swap(&x[0], &x[1])` type-checks but `swap(&x[i], &x[1])` is *rejected* — the checker "conservatively assumes that `f(0)` could be evaluated as any value, including `1`." This is a *purely syntactic check on paths* — no region inference, no constraint graph.
  - *Real compiler*: an **abstract interpreter over Hylo IR** using `borrow`/`end_borrow`/`access` **ghost instructions** that "only exist for analysis purposes and have no operational semantics." Borrow checking is described "in terms of the language's operational semantics, extended for ghost instructions" — deliberately "farther from the type system, closer to operational semantics" than Rust's constraint collection.

This is the **most actionable architectural parallel for Kestrel.** Kestrel's MIR already has `BeginBorrow`/`EndBorrow`/`BeginMutBorrow` — the structural analogue of Hylo's ghost instructions — and an OSSA verifier (`verify.rs`). Hylo says the right home for the escape/liveness check is **the IR verifier made flow-sensitive, not the type-infer constraint solver.** This directly validates references.md's central insight: Stage 1 needs an escape checker but can *defer region/outlives inference entirely* — Kestrel does **not** need an inequality/outlives constraint class in `constraint.rs` to get a sound second-class system. The "param outlives the call" MVP proof (walk `borrow_source` to a parameter root) is exactly Hylo's "source is always locally available" principle in miniature. **The catch** (references.md flags it): Kestrel's verifier is a single forward BFS with *no fixpoint*, whereas Hylo's abstract interpreter is a proper dataflow analysis. The MVP sidesteps this by re-borrowing params fresh per block; a fuller cross-block borrow story (Stage-1 #4) means upgrading `verify.rs` toward a real abstract interpreter — exactly the work the MVP defers.

### 2.4 Avoiding "a lifetime on every type"

Hylo avoids it *structurally, not cleverly*: because second-class references "can neither be assigned to a variable nor stored in object fields, and all values form disjoint topological trees," **no type ever contains a reference, so no type ever needs a lifetime parameter.** The escape hatch for genuinely-needed reference-like fields is **remote parts**: "Remote parts exist to get around the fact that you can't store references inside data structures," but "the rules for instances of types with remote parts are the same as the rules for bare local '2nd-class references': they aren't allowed to escape their local scope." Closures fold into the same model: "The `let` and `inout` captures of a closure are exactly remote parts. A closure with such captures can't escape its local scope" — a borrowing closure is itself second-class.

This confirms references.md's Stage-2 warning that refs-in-aggregates force a lifetime dimension onto `MonoTypeKey` (the cross-instantiation double-free risk). Hylo's verdict: *don't* — keep refs out of types. And if Kestrel ever attempts Stage 2 anyway, "remote parts" is the precedent for ref-in-struct **without** per-type lifetimes — by making the *whole containing value* second-class (non-escaping) instead of parameterizing it by a lifetime. That is materially cheaper than Rust-style lifetime-on-every-type. (Hylo's closure-capture model is also directly relevant to references.md's flagged collision between ref-capture and the Rc-closure migration: Hylo's borrowing closures are simply non-escaping.)

### ⚠️ Callout: Kestrel's `&mutating`-MAY-ALIAS diverges from Hylo's exclusive `inout` — what that forfeits

This is the single sharpest divergence in the whole study. Hylo's `inout` is **exclusive by mandate** — the **Law of Exclusivity**, verbatim from the MVS paper:

> "to prevent writeback from being discarded, overlapping mutations are prohibited. In other words, `inout` arguments must have independent values. This Law of Exclusivity … creates a crucial optimization opportunity: it is *safe* to sidestep the conceptual copies by allowing the callee to write the argument's memory in the caller's context. In other words, `inout` argument passing can be implemented as pass-by-reference without surfacing reference semantics in the programming model."

Kestrel has *deliberately decided the opposite*: `&mutating` MAY ALIAS, with no exclusivity inference. Two things follow that Kestrel must internalize:

1. **Kestrel forfeits Hylo's entire optimization story.** Exclusivity is what makes "write the argument's memory in the caller's context" (and LLVM `noalias`) *safe*. By allowing aliasing, Kestrel gives that up — references.md §10 already flags "No aliasing-based optimization." (Note: Hylo states this as a design *opportunity*; whether the compiler actually emits `noalias` is unverified — see §8. The directional point holds regardless.)
2. **More importantly, exclusivity is Hylo's *soundness backstop*.** It is the linearity guarantee that, combined with can't-escape, makes the whole system safe with *only local analysis*. Kestrel removes that backstop, so its soundness rests **entirely** on the escape/liveness checker (references.md §10: "no linearity backstop underneath"). This is also why Hylo *can't* express the case Kestrel wants — per a maintainer, "there's no way in Val to say that you definitely won't use parts from `cache` in the projection … that's the kind of hit to expressiveness we're willing to take in exchange for a simpler type system." Hylo *chooses* exclusivity and pays an expressiveness cost; Kestrel *chooses* expressiveness and pays a soundness/verification cost. **Avoid** importing the Law of Exclusivity — but understand that Kestrel could optionally keep a *weakened* form of the cheap path-uniqueness check (e.g. for read-during-write, the verifier's Check 5). Full exclusivity is off the table by decision.

---

## 3. Swift — the mainstream analog

Swift and Mojo took the same two-axis path Kestrel is on: (1) a parameter-convention system naming who owns/borrows, and (2) a *separately staged* answer to "can a borrow-like value escape?" Swift's answer is the closest mainstream precedent to this entire proposal.

### 3.1 Parameter conventions (SE-0377) — Kestrel already mirrors these

Three contextual keywords in the parameter slot (same position as `inout`), mutually exclusive:

- **`borrowing`** — "The callee temporarily uses the parameter while guaranteeing not to release it." The **default** for ordinary functions. = Kestrel `@guaranteed`.
- **`consuming`** — "The callee becomes responsible for either releasing the parameter or passing ownership of it along." **Default for initializers and property setters.** = Kestrel `@owned`.
- **`inout`** (`mutating` for receivers) — an *exclusive* mutable borrow.

`borrowing`/`consuming` make the binding **not implicitly copyable** in the body — you write `copy x` to copy and `consume x` to transfer/end. For *copyable* types the conventions interconvert; for **noncopyable** types "the convention must match exactly" (it becomes ABI/API contract). For a noncopyable type, the **receiver (`self`)** of its methods defaults to `borrowing` unless declared `mutating`/`consuming`; ordinary *parameters* of noncopyable type get no such default — SE-0390 "requires parameters of noncopyable type to explicitly state whether they are `borrowing` or `consuming`."

Kestrel already mirrors this exactly — `borrowing`/`mutating`/`consuming` receiver conventions over `@guaranteed`/`@owned`. SE-0377 is essentially **Kestrel's existing model, independently validated by the largest mainstream ownership rollout.** The one piece Kestrel lacks is the explicit `copy`/`consume` operators that surface the no-implicit-copy contract to users — worth considering as a usability addition, not a requirement.

### 3.2 `~Copyable` and the suppressible-default mechanism (SE-0390, SE-0427)

`~Copyable` *suppresses* the implicit `Copyable` requirement; the value gets unique ownership, a `deinit`, and flow-sensitive last-use / per-branch consumption tracking. Kestrel already has this substrate (the active MIR-3 noncopyable move work), so it is **note, not adopt** — relevant as the ground references sit on.

SE-0427 generalizes the **default-suppressible-protocol pattern** to generics, and this is the part Kestrel's Stage 3 should copy *verbatim*:

- Every struct/enum/**generic param/protocol/associated type conforms to `Copyable` by default** (`<T>` means `<T: Copyable>`).
- `<T: ~Copyable>` *removes* the requirement — it does **not** require noncopyability: "This function imposes no requirements on `T`. All possible types, both Copyable and noncopyable, can be substituted."
- **Progressive disclosure / source compat:** existing code is unchanged because all existing concrete types do conform.
- Protocols must **re-state** `~Copyable` on inheritance (no propagation): `protocol CasinoToken: Token, ~Copyable {}`.
- **Conditional conformance** is restricted to `extension List: Copyable where T: Copyable {}`, and forbidden if the type has a `deinit`.

References.md already identifies the matching Kestrel infra — `implicit_conformance: true` (`builtin.rs:647`), `inject_implicit_copyable_bounds`, `WhereConstraint::NegativeBound` → `NotImplements`. Kestrel **already implements this exact pattern for Copyable.** Stage 3 is literally "add `Builtin::Static` with the same flag." SE-0427 supplies the precise rules to copy: default-present bound, suppression *lifts* rather than *requires*, no inheritance propagation, restricted conditional form. **Adopt the rule set.**

### 3.3 `~Escapable` + lifetime dependency (SE-0446 + `@lifetime`) — THE direct analog

This is Swift's deliberate, minimal alternative to full Rust lifetimes, and the closest mainstream analog to Kestrel's entire proposal.

- `struct NotEscapable: ~Escapable {}` suppresses the implicit `Escapable` protocol (which all types except nonescapable closures get).
- An **escapable** value can "be assigned to global variables, passed into arbitrary functions, or returned." A **nonescapable** value *cannot* be returned, assigned to a binding in a larger scope, put in globals/statics, or captured by escaping closures. It *can* be copied locally and passed `borrowing`/`consuming`/`inout`.
- **Containment rule:** "An escapable struct or enum can only contain escapable values." This is *exactly* Kestrel's Stage-2 "a type that contains a reference is non-Static."
- **To return a nonescapable value you must annotate a lifetime dependency** (SE-0446 alone cannot construct/return them — that is what `@lifetime` adds):
  - `@lifetime(borrow self)` / `@lifetime(borrow pointer)` — result lifetime tied to a *borrowed* source; result "cannot outlive" it and can't enter Escapable containers.
  - `@lifetime(copy value)` — result depends on a value *consumed* into it (e.g. `UnsafePointer` → `Span` init).
  - `@lifetime(immortal)` — no dependency; lives indefinitely (the `Static` case).

```swift
@lifetime(borrow self)
borrowing get { ... }            // returned Span borrows self
@_lifetime(borrow pointer)
init(_ pointer: UnsafeRawPointer?, count: Int)
@_lifetime(immortal)
init(nilLiteral: ())
```

**Lifetimes are explicitly *written*, not inferred** (per the Pitch #3 author). The compiler then enforces the nonescapable result doesn't escape its dependency scope.

The Kestrel alignment is dense:
- Stage 1's "the referenced value must escape (outlive the call) to be returned" = Swift's `@lifetime(borrow source)` where source is a parameter. Kestrel's `borrow_source`-traces-to-a-`Param` escape proof **is** the borrow-dependency check.
- Stage 3's `Static` bound = Swift's `Escapable` default; `not Static` = `~Escapable`; the containment rule = Kestrel's Stage-2 "type containing a ref is non-Static."
- **The single most important transferable caution:** Swift chose to make the dependency **explicit rather than inferred.** Kestrel's Stage 1 plans to *infer* lifetimes. Swift's experience says inferring *which-source-a-return-depends-on* is exactly the part to be conservative about — and the `@lifetime(borrow)` vs `@lifetime(copy)` distinction (borrowed-source vs consumed-source dependency) is a real semantic fork that Kestrel's single `root_provenance` enum should anticipate. The proven-safe cut is an explicit annotation *or* a v1 restriction to single-param-rooted returns — which is precisely what Kestrel's MVP already proposes. (Caveat in §8: Swift may do narrow inference for the obvious self-dependency accessor case; "never inferred" is the design *intent*, not an absolute.)

---

## 4. Mojo — `ref` + origins, the heavier path

Mojo went the way Kestrel is mostly *choosing not to* in Stage 1: first-class-ish lifetimes (`origin`) threaded everywhere, plus enforced mutable-reference exclusivity.

Mojo's four argument conventions (keyword precedes the arg name):
- **`read`** — immutable reference, the **default**. (= `borrowing` / `@guaranteed`.)
- **`mut`** — mutable reference. **Mojo ENFORCES exclusivity:** "if a function receives a mutable reference to a value … it can't receive any other references to the same value—mutable or immutable" (`error: passing my_string mut is invalid since it's also passed read`).
- **`var`** — ownership transfer / unique mutable access; `^` transfer sigil ends the source's lifetime, otherwise Mojo copies. (= `consuming` / `@owned`.) *(Older Mojo says `owned`/`inout`/`borrowed` — the keywords are version-sensitive; see §8.)*
- **`ref`** — parametric-mutability convention tied to an **origin**.

**Origins** are the lifetime machinery: "compiler-level symbolic values that track variable ownership and reference mutability" — answering "what variable owns this value?" and "can it be mutated through this reference?" They are *comptime parameters*, mostly compiler-created, derived via `origin_of(expr)` (operand never evaluated), with `ImmutOrigin`/`MutOrigin`, `StaticConstantOrigin` (program-duration, e.g. string literals), **union origins** (`ref[a, b]`, mutable iff *all* constituents are), and wildcard `Imm/MutAnyOrigin` (last resort).

- A **`ref` return MUST carry an origin specifier** — this is what makes returning a reference safe:
  ```mojo
  def __getitem__(ref self, index: Int) -> ref[self] String:
      return self.names[index]
  ```
- Origins **are storable on types**: `Pointer[String, origin]`, `Span[Byte, origin]` carry the origin as a type parameter — i.e. **Kestrel's Stage-2 "lifetime on every type" as the baseline.**
- Origins are **inferred** from arguments via infer-only parameters, but are **not first-class values you create freely** — you derive them from existing variables.

Two sharp contrasts make Mojo the cautionary case:

1. **Mojo enforces mut-exclusivity (no aliasing) — the opposite of Kestrel's deliberate `&mutating`-may-alias decision.** Kestrel trades Mojo/Rust exclusivity for simplicity, paying by relaxing its read-during-mut-borrow check (Check 5) and resting soundness on the lifetime checker. Mojo's choice is the path Kestrel is rejecting. **Avoid.**
2. **Mojo makes origins first-class type parameters threaded everywhere** — its Stage-2/3 equivalent is the *baseline*, confirming references.md's claim that "lifetime on every type" (Stage 2) is the real cost center. This is why Kestrel's "second-class, inferred" choice is the *lighter* path: it never forces the `MonoTypeKey` lifetime-provenance dimension that Mojo bakes in from day one.

Two transferable specifics, both **note** rather than adopt:
- Requiring an explicit `ref[self]` origin on *every* ref-return mirrors Swift requiring explicit `@lifetime`. **Both mainstream designs refuse to fully infer return-reference provenance** — reinforcing the caution against Kestrel's Stage-1 inference.
- Mojo's `ref[a, b]` **union origin** (mutable iff all constituents mutable) is the production design for references.md's trap #5/#6: `return cond ? &a : &b` (a borrow whose source is one-of-several joined at a branch). Kestrel's MVP rejects this but should keep `borrow_source` able to *represent* a multi-source borrow. Mojo shows the concrete target shape — confirming single-source `borrow_source` is a v1 limitation to design *around*, not bake in.

---

## 5. What transfers to Kestrel

| Idea | Source | Adopt / Adapt / Avoid | Why |
|---|---|---|---|
| Four pre-type conventions (`let`/`inout`/`sink`/`set`, +`yielded`) | Hylo | **Adapt** | Near-perfect match to `@owned`/`@guaranteed`. Adopt the `&T`/`&mutating T` surface but keep Kestrel's `ParamConvention` machinery underneath. |
| Borrowing/consuming param modifiers (default borrow; init/setter consume) | Swift SE-0377 | **Adopt** | Validates Kestrel's existing receiver-convention model. Reference *params* reuse it directly (cheapest win). Consider surfacing explicit `copy`/`consume`. |
| References are second-class: never stored in vars/fields | Hylo | **Adopt** | Exact rationale behind Kestrel's Stage-1 cut. Hylo proves the strongest form is shippable *and* a coherent end state — staying second-class indefinitely is viable, not just transitional. |
| Express "access to part of an object" via yielding subscripts/projections, not returned refs | Hylo | **Adapt** | A lower-risk substitute for return-refs: needs no escape checker and no conditional-`@guaranteed`-return surgery (references.md #1 risk). Needs caller-side coroutine scoping Kestrel lacks. |
| Enforce escape by flow-sensitive **local** analysis, never a region solver | Hylo | **Adopt** | Confirms Stage 1 can defer outlives inference entirely. No inequality constraint class needed in `constraint.rs`. "Param outlives the call" = Hylo's "source always locally available." |
| Run the escape/exclusivity check on SSA IR with borrow/end_borrow ghost insts via abstract interpreter — not the type solver | Hylo | **Adapt** | Kestrel already has `BeginBorrow`/`EndBorrow` + an OSSA verifier — the right home for the check. Adapting fully means upgrading `verify.rs` from single-BFS toward a fixpoint dataflow analysis (the deferred Stage-1 #4 work). |
| `~Copyable` unique-ownership substrate | Swift SE-0390 | **Note** | Kestrel already has non-Copyable types + move lowering; confirms the flow-sensitive drop-flag approach is standard. Ground refs sit on, not new work. |
| Default-suppressible bound on **every** generic param/protocol/assoc-type; suppression *lifts* not *requires* | Swift SE-0427 | **Adopt** | Exact template for Stage 3's default `Static` bound + `T: not Static`. Kestrel already does this for Copyable. Rules to copy: default-present, suppression-lifts, no inheritance propagation, restricted conditional form. |
| `~Escapable` + containment rule (escapable container holds only escapable values) | Swift SE-0446 | **Adapt** | Closest analog to the whole proposal. `Escapable` default = `Static`; `~Escapable` = `not Static`; containment = Stage-2 "type containing a ref is non-Static." Adapt because Kestrel's `&T` is a distinct may-alias type, not a type-level marker. |
| Explicit lifetime-dependency to **return** a non-escapable value (borrow vs copy vs immortal), NOT inferred | Swift SE-0446/`@lifetime` | **Adapt** | `@lifetime(borrow param)` IS Kestrel's "borrow_source traces to Param" proof. The borrow/copy fork is a real distinction `root_provenance` should anticipate. **Key caution: Swift writes it, doesn't infer it** — Kestrel's Stage-1 inference is the risky part; v1 single-param-rooted restriction is the proven-safe cut. |
| Avoid "lifetime on every type" by making it structurally impossible to store a ref; "remote parts" as the only hatch | Hylo | **Adapt** | Confirms Stage-2 `MonoTypeKey` collapse risk. If Stage 2 is ever attempted, "remote parts" = ref-in-struct *without* per-type lifetimes (whole container becomes second-class). Cheaper than Rust-style. |
| Mutable refs require **exclusivity** (Law of Exclusivity) | Hylo `inout` / Mojo `mut` | **Avoid** | The sharp divergence. Kestrel chose `&mutating` MAY ALIAS by decision. Forfeits the optimization story AND the soundness backstop — soundness now rests entirely on the escape/liveness checker. A weakened path-uniqueness for read-during-write (Check 5) is optional; full exclusivity is off the table. |
| Origins as first-class comptime type-parameters threaded everywhere; storable `Pointer[T, origin]`/`Span[T, origin]` | Mojo | **Note** | The "origins on every type" design = Kestrel's Stage 2 = the cost center. Confirms origins-as-type-parameters is the `MonoTypeKey` lifetime dimension that collapses instantiations if omitted. Kestrel's second-class/inferred choice is intentionally lighter. |
| `ref[t1, t2]` union origins (mutable iff all constituents mutable) | Mojo | **Note** | Production design for the `return cond ? &a : &b` multi-source trap. Keep `borrow_source` able to *represent* one-of-several even if v1 rejects it. Single-source is a v1 limitation to design around. |
| Don't allow a returned **mutable** projection whose freezing extends past where the checker can see | Hylo (mutable projections are lexically bracketed) | **Adopt** | Independently corroborates references.md's explicit trap: no `&mutating` return in v1 — a returned mut-ref freezes the source past `Return`, which the per-block verifier (Check 5) can't enforce. Ship shared `&T` returns only. |

---

## 6. The big strategic question — can Hylo's model let Kestrel avoid Stage 2's "lifetime on every type" cost entirely?

Hylo is direct, working evidence that **"references are accesses, not values"** lets a language drop named lifetimes *and* avoid putting a lifetime parameter on any type — forever, not just provisionally. So the honest question is whether Kestrel can declare Hylo's position its *end state* and never pay the Stage-2 cost center (references.md's make-or-break milestone). Both sides:

**Yes — Kestrel can plausibly stop at a Hylo-shaped Stage 1.5 and never build Stage 2.**

- The mechanism that makes "no lifetime on any type" free in Hylo is *structural*, not clever: refs can't be stored, so no type contains one, so no type needs a lifetime slot. Kestrel's Stage-1 cut already enforces exactly this ("defer `MirTy::Ref` as a storable type"). Holding that line indefinitely is coherent — Hylo ships it.
- The day-to-day value Kestrel wants is overwhelmingly Stage-1 value: pass-by-reference without COW clones, the `Str.toBytes()`-class amplifiers, accessor methods returning borrows. references.md §11 already says Stage 1 "delivers most of the day-to-day value … without the pervasive cost."
- Hylo's **projections/yielding subscripts** cover the one expressiveness gap that would otherwise push toward Stage 2 — "return a reference to part of an object" — *without* storing a reference. If Kestrel adopts the projection mechanism (§2.2), the strongest motivation for ref-in-struct (giving out interior references) is satisfied by scoped access instead of storage. That removes the biggest pull toward Stage 2.
- Hylo's **remote parts** show that even the genuine "I need a reference-like field" case can be handled by making the *whole container* second-class (non-escaping) rather than by adding per-type lifetimes. So even a partial Stage 2 need not import the `MonoTypeKey` lifetime dimension — the exact thing references.md warns causes cross-instantiation double-free.

**No — Kestrel's specific commitments make the Hylo dodge weaker than it looks.**

- **The dodge depends on exclusivity Kestrel threw away.** Hylo's local-only soundness rests on *both* can't-escape *and* the Law of Exclusivity — the linearity backstop. Kestrel has removed exclusivity by choosing `&mutating`-may-alias (§2.4 callout, references.md §10). So Kestrel inherits the "no stored refs" half of Hylo's safety argument but *not* the half that makes the aliased-mutation case sound. The moment refs touch heap-shared/`RcBox` state, Kestrel needs a container-outlives-referent proof Hylo never has to make. Adopting Hylo's *structure* does not buy Hylo's *soundness* for Kestrel's chosen semantics.
- **Mojo is counter-evidence that "stop at second-class" is not the only sane equilibrium.** Mojo deliberately went first-class (storable origins) because real systems code wants `Span`/`Pointer` *fields* — reference-bearing structs are a primary use case, not an edge. If Kestrel's users want the same (slices, views, cursors stored in structs), the Hylo dodge becomes a permanent expressiveness ceiling, and the pressure to build Stage 2 returns regardless. Hylo accepts that ceiling as a stated tradeoff ("the kind of hit to expressiveness we're willing to take"); Kestrel must decide whether *its* users will.
- **Even Hylo doesn't fully escape the cost — it relocates it.** "No lifetime on types" is bought with a *more capable analyzer*: a proper fixpoint abstract interpreter over IR with ghost instructions. Kestrel's verifier is a single forward BFS with no fixpoint (references.md, confirmed). So "avoid Stage 2" is partly "pay for a real dataflow verifier instead" — cheaper than lifetime-on-every-type, but not free, and it is precisely the Stage-1 #4 work the MVP defers. The cost doesn't vanish; it moves from the type system to the analyzer. *(Caveat §8: that Hylo's checker is a true fixpoint analysis vs. Kestrel's BFS is inferred from "abstract interpreter" terminology, not from reading Hylo's algorithm.)*

**Adjudication.** Hylo proves the *destination* (no lifetime on any type, second-class forever) is real and shippable, and projections + remote parts give Kestrel concrete tools to satisfy most of what would otherwise demand Stage 2 — so **Kestrel should treat "stop at a Hylo-shaped Stage 1.5 (refs + projections, no stored refs)" as a legitimate end state and the default plan, not merely a stepping stone.** But it cannot claim Hylo's *soundness* for free, because it discarded the exclusivity backstop Hylo's local analysis relies on; the honest version of "avoid Stage 2" is "invest in the escape/liveness analyzer Hylo has (a fixpoint dataflow check) and adopt projections, while accepting a reference-in-struct ceiling that Mojo-style users will eventually push against." If that ceiling proves acceptable to Kestrel's users, Stage 2's cost center can be avoided indefinitely. If not, Hylo's "remote parts" — making the whole container second-class — is the cheaper way into limited ref-in-struct than the Rust/Mojo lifetime-on-every-type design references.md fears.

---

## 7. Sources

**Hylo / Val**
- https://docs.hylo-lang.org/language-tour/functions-and-methods
- https://docs.hylo-lang.org/language-tour/subscripts
- https://docs.hylo-lang.org/hylo-ir/
- https://hylo-lang.org/docs/reference/specification/
- https://github.com/hylo-lang/specification/blob/main/spec.md
- https://github.com/orgs/hylo-lang/discussions/788
- https://github.com/orgs/hylo-lang/discussions/754
- https://www.jot.fm/issues/issue_2022_02/article2.pdf (JOT "Swiftlet" MVS formalization)
- https://2023.splashcon.org/details/iwaco-2023-papers/5/Borrow-checking-Hylo (IWACO 2023)

**Swift**
- https://github.com/swiftlang/swift-evolution/blob/main/proposals/0377-parameter-ownership-modifiers.md
- https://github.com/swiftlang/swift-evolution/blob/main/proposals/0390-noncopyable-structs-and-enums.md
- https://github.com/swiftlang/swift-evolution/blob/main/proposals/0427-noncopyable-generics.md
- https://github.com/swiftlang/swift-evolution/blob/main/proposals/0429-partial-consumption.md
- https://github.com/swiftlang/swift-evolution/blob/main/proposals/0446-non-escapable.md
- https://forums.swift.org/t/pitch-3-compile-time-lifetime-dependency-annotations/84968

**Mojo**
- https://mojolang.org/docs/manual/values/lifetimes/
- https://mojolang.org/docs/manual/values/ownership/

---

## 8. Low-confidence claims (do not present as fact)

**Hylo**
- The JOT paper formalizes **"Swiftlet" (a Swift subset), not Hylo/Val itself.** The path-uniqueness `⊆` relation and `T-CALL` exclusivity rule are verified from the paper's Figures 4–5, but the *current Hylo compiler* may enforce exclusivity via the Hylo IR abstract interpreter rather than this exact syntactic check. The two are philosophically the same (local, no named lifetimes); the precise algorithm in the compiler source was not directly read.
- The Hylo IR **ghost-instruction details** (`borrow`/`end_borrow`/`access`) come from the IWACO 2023 abstract and a search summary, not a full reading of the IWACO PDF or the hylo-ir docs page. The claim that the abstract interpreter is a **proper fixpoint dataflow analysis** (vs. Kestrel's single-BFS verifier) is *inferred from the term "abstract interpreter,"* not from reading the algorithm. The §6 argument that "avoiding Stage 2 means paying for a real fixpoint verifier" rests on this inference.
- Whether Hylo **today** strictly forbids *all* stored references or has relaxed via "remote parts" for some library types is only partially clear. Discussion #754 says remote parts exist to store reference-like data but remain bound by can't-escape. The **exact surface syntax and current stability of remote parts is uncertain.**
- The `sink` subscript "consumes yielded on last use" and `set` "accepts implicit new_value" details come from a **single WebFetch summary** of the subscripts tour; exact current syntax may differ.
- Hylo's **LLVM `noalias` / optimization benefit** is stated in the MVS paper as a *design opportunity* ("creates a crucial optimization opportunity"); it was **not verified that the current compiler actually emits `noalias`.** The directional point (Kestrel forfeits this by allowing aliasing) holds regardless.

**Swift / Mojo**
- **Swift `@lifetime` attribute spelling is in flux.** SE-0446 shipped, but lifetime-dependency annotation is still experimental — seen as both `@lifetime` and `@_lifetime` (underscored). The borrow/copy/immortal distinction is firm; the final non-underscored syntax (and whether `dependsOn(...)` vs `@lifetime(borrow:)` is accepted) was unsettled at the fetched proposal stage. **Treat keyword forms as illustrative, not final ABI.**
- **"Swift never infers lifetime dependencies"** is the design *intent* per the Pitch #3 author, but Swift appears to do **limited inference for trivial single-parameter cases** (e.g. a borrowing getter inferring `@lifetime(borrow self)`). The strong "not inferred" claim should be read as "inference is deliberately minimal," not absolute — relevant to how hard Kestrel's Stage-1 inference caution should be taken.
- **Mojo's API is moving fast.** Docs moved to mojolang.org (modular.com URLs 301-redirect). The `var` owning convention reflects a recent rename from `owned`; older sources say `owned`/`inout`/`borrowed`. **Convention semantics are stable; keywords are version-sensitive.**
- **Whether Mojo origins can be stored as plain struct fields** (not just as type parameters on `Pointer`/`Span`) is **not confirmed** from primary docs. Confirmed: types are parameterized on origin (origin-as-type-parameter is storable). Storing a bare reference/origin as an ordinary field independent of a wrapper type was not verified.
- **Mojo's exclusivity-enforcement completeness is unverified.** Docs state it is compiler-enforced with a concrete error, but it was not confirmed whether enforcement covers all aliasing paths (e.g. through `UnsafePointer`, which can bypass origin tracking) or only direct mut+read argument collisions. The "similar to Rust" framing is a paraphrase, not a claim of Rust-equivalent NLL/two-phase borrows.

**Cross-cutting**
- All **adopt/adapt/avoid stances and Kestrel-effort mappings are synthesis recommendations**, not measured outcomes. The mappings onto Kestrel internals (`verify.rs` being single-BFS/no-fixpoint, `ParamConvention` threading, `constraint.rs` lacking inequality constraints, the `MonoTypeKey` collapse risk) are **taken from references.md as given, not independently re-verified against the current tree** — and references.md itself notes its line anchors churn and its effort numbers are bracketed estimates.
