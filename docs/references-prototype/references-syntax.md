# References — Syntax & Semantics Design

**Status**: Design draft — maintainer decisions required
**Scope**: Stage 1 only (in-function refs + parameter-rooted return refs + inferred lifetimes). Stage 2/3 surfaces noted only where a Stage-1 choice would paint them into a corner.
**Companion**: `docs/references-prototype/references.md` (feasibility — read first for the MVP cut, the traps, and the verified code anchors)

This doc decides the **surface syntax and semantics** of `&T` / `&mutating T` — the calls only the maintainer can make. Each section gives 2–3 concrete options with tradeoffs and a marked **RECOMMENDATION**.

A guiding principle runs through every recommendation: **Kestrel already has a coherent by-reference story** — the `mutating`/`consuming` param keywords, the implicit `borrowing` default, and the in-progress "Design B" that *infers* closure-param conventions from expected type and *deliberately omits exclusivity enforcement*. References should be the **nameable, returnable** extension of that exact model, not a parallel one. Where a new surface would create two ways to say the same thing, we collapse to one.

---

## 1. Reference type syntax — `&T` and `&mutating T`

### Grounding (verified)
- The single `&` character lexes to `Token::Ampersand` (`lexer:729`). In **expression** context it is the binary bitwise-AND operator. It has **zero use in the type parser** — the `base_ty` chain (`some / never / inferred / parens / array_or_dict / path`) never consumes it, and the postfix loop only handles `?` and `throws`. A prefix `&` in type position conflicts with nothing.
- `&=` is a distinct token (`AmpersandEquals`, longest-match), so it can't be confused for `&` + `=Type`.
- `some P and Q` uses `Token::And` (the keyword `and`), **not** `&`. No clash with opaque-type bounds.
- `mutating` already exists as a lexer keyword in three roles (receiver modifier, param access mode, function-type param marker). Adding it after `&` is a fourth position but a separate parser call chain.

### Options

**Option A — `&T` / `&mutating T`** (the proposal)
Reuses the `mutating` keyword Kestrel already has for mutable-by-reference params. The two surfaces (`mutating on grid: Grid` and `&mutating Grid`) share a word, reinforcing "this is the mutable-borrow concept."
- Pro: one mutability vocabulary across params and ref types; `&` is free in type position; no new lexer token.
- Pro: `&mutating` reads in English, matches `mutating func`.
- Con: `&mutating T` is verbose (9 chars of qualifier). Inside a function-type param list, `(&mutating Grid) -> R` requires the emitter to wrap `& mutating T` as **one atomic `Ty` node** so the existing positional `Mutating`-scan in `ast_type_from_cst` (ast_type.rs:85-99) doesn't see a stray `Mutating` token at `TyList` level (the sharpest integration trap from the grounding).

**Option B — `&T` / `&mut T`** (Rust spelling)
- Pro: shortest; familiar to Rust users.
- Con: **introduces a second mutability word.** Kestrel has no `mut` keyword anywhere — mutability is `var` (bindings) and `mutating` (params/receivers). `&mut` would be the *only* place `mut` appears, fragmenting the vocabulary. Requires a new `Token::Mut` or contextual-keyword handling. Rejected on single-source-of-truth grounds.

**Option C — `inout T` / `ref T`** (convention-as-type words)
- Con: Kestrel deliberately has **no `inout`** (the whole reason `mutating` exists per memory). Adding `inout`/`ref` resurrects rejected vocabulary and gives no `&`-sigil affordance for the borrow expression in §2. Rejected.

### RECOMMENDATION ✅ — **Option A: `&T` (shared) and `&mutating T` (mutable).**

`&` is the sigil; `mutating` is the existing mutability keyword. This keeps **one** mutability vocabulary (`var` / `mutating`) and makes the param surface (§4) and the type surface the same concept under one word. Spend the implementation cost on the **atomic-`Ty`-node** requirement: emit `TyRef { amp, mutating?, inner }` as a single CST node so the `mutating` positional scan never misfires. The shared `&T` is the common case and stays terse; `&mutating` paying for its extra length is acceptable because mutation should be visible.

> Forward-compat note: the immutable form is `&T` with **no** keyword (borrowing is the default, exactly as `borrowing` is the implicit receiver default). Do **not** add a `&borrowing T` spelling — it would mirror the nonexistent `borrowing` keyword and create a redundant surface.

---

## 2. Borrow EXPRESSION — explicit `&x` vs inferred-from-context

**This is the central decision.** Does taking a borrow require an operator (`f(&x)`), or is the borrow synthesized by the compiler when an `&T` is expected (`f(x)`)?

### The decisive precedent: Design B + the `mutating` param call-site
Two existing Kestrel facts point hard in one direction:

1. **`mutating` params are invisible at the call site.** Per the verified memory: `func stamp(mutating on grid: Grid)` is called as `stamp(on: grid)` — *the caller passes the value normally; the callee mutates in place.* There is **no `&` marker at the call** today for the one by-reference feature Kestrel already ships. `mutating` is declaration-only.
2. **Design B infers closure-param conventions from the expected function type** — `arr.modify { (x) in x.n += 3 }` upgrades `x` to `MutBorrow` with **no annotation**, by reading `ResolvedTy::Function.conventions`. Convention-inference-from-expected-type is **already the established mechanism** in this codebase.

A reference *parameter* is the same shape as a `mutating` param (both lower to `ParamConvention::Borrow/MutBorrow` → `PassMode::ByRef`). If `mutating on grid` is invisible at the call but `&mutating Grid` demands `f(&grid)`, Kestrel has **two visually different call conventions for one underlying mechanism** — the exact single-source-of-truth violation CLAUDE.md warns against.

### Options

**Option A — Explicit borrow operator `&x` (Rust/Hylo-marker style)**
Caller writes `f(&x)` / `let r = &x;`. The `&` is required wherever an `&T` is produced.
- Pro: borrows are syntactically visible — grep-able, obvious in review, matches Hylo's "`&` signals the argument is borrowed/mutated."
- Pro: simplest inference story — the operator *is* the type signal; no expected-type plumbing.
- **Con (decisive): contradicts the existing `mutating`-param call site**, which takes no marker. Either you retrofit `&` onto `mutating` params (a breaking change to shipped syntax) or you live with the inconsistency. Both are bad.
- Con: at the expression level `&x` is the **real ambiguity site** the grounding flags — the parser must distinguish prefix `&x` (borrow) from infix `a & b` (bitwise-AND). Solvable (prefix vs infix position), but it's net-new expression-parser work and a lexer-context concern that the type-only path entirely avoids.

**Option B — Inferred from expected type (Design B style), no operator**
Caller writes `f(x)`; when the param is `&T`/`&mutating T`, the compiler emits the borrow (`BeginBorrow`/`BeginMutBorrow`) automatically, exactly as it does for a `mutating` param today.
- Pro: **identical to the shipped `mutating`-param call site and to Design B** — one call convention, one mechanism. This is the single-source-of-truth answer.
- Pro: **zero new expression-parser work** — `&` never appears in expression position, so the `a & b` ambiguity never arises. Stage 1 stays type-parser-only, which the feasibility doc explicitly counts as the cheap path.
- Pro: the lowering already exists — `lower_expr_for_borrow` produces a `@guaranteed` value without consuming the source; `prepare_call_arg` (Design B) already passes already-`@guaranteed` values directly for `MutBorrow`.
- Con: borrows are **invisible** at the call site. A reader can't see that `f(x)` borrows vs consumes `x` without consulting `f`'s signature. (Mitigated: the LSP already needs to surface convention — see below.)
- Con: a returned `&T` bound to a `let` (`let r = first();`) gives `r` reference type with no syntactic cue that `r` aliases something.

**Option C — Hybrid: inferred for params, explicit for `let`-bindings**
Params infer (matching `mutating`); but binding a reference to a name requires a cue: `let r = &first();` or `ref r = first();`.
- Pro: keeps call sites consistent with `mutating` while making *named, escaping* references (the genuinely new, dangerous thing) visible at their binding site — which is exactly where the escape checker (#2 in feasibility) does its work.
- Pro: the visible cue lands precisely on the construct that can dangle (a stored reference), not on the cheap/safe construct (a borrow param).
- Con: two rules to learn. The `let r = &expr;` form *does* reintroduce a prefix-`&` expression, so it reincurs the `a & b` disambiguation — though only in the restricted `let`-initializer position, not general expressions.

### RECOMMENDATION ✅ — **Option B for parameters (inferred, no marker), with a Stage-1 restriction that defers named reference bindings — i.e. start at B and grow into C only if/when `let r = ...` of reference type is allowed.**

Rationale:
- **Parameters must infer.** Anything else forks the call convention away from the shipped `mutating` param and from Design B. Consistency here is non-negotiable: `f(x)` borrows when `f` says `&T`, mutates-in-place when `f` says `&mutating T`, consumes when `f` says `T` — *the signature is the single source of truth*, identical to today.
- **The MVP barely needs a binding form at all.** The feasibility MVP ships *return-borrow-of-a-parameter* and *reference parameters* — neither requires the user to ever write a `&` in an expression. A returned `&T` is typically consumed immediately (`use(obj.first())`) or passed onward. **Defer the `let r = &x;` binding** to the same milestone that lifts the "no `&T` in `var`/locals" restriction (Stage 2's storable refs). This keeps Stage 1 **100% type-parser-only** with no expression-`&` ambiguity to solve.
- **When named ref bindings do arrive, adopt Option C's explicit cue** (`let r = &expr;`) — the visible `&` belongs exactly on the construct that can outlive its referent, giving the escape checker a syntactic anchor and the reader a danger flag. This is the Swift/Mojo lesson (both *refuse to fully infer return-reference provenance* and require a written cue at the escape point), applied narrowly to bindings rather than to every call.

**Diagnostics / LSP consequences (must-haves for Option B):**
- The LSP **inlay-hint** must render the inferred borrow at call sites — e.g. show `f(⟨&⟩x)` or annotate the param convention on hover — because the call site is otherwise silent. Kestrel already needs convention-surfacing for `mutating` params and Design B closures, so this is shared infrastructure, not new debt.
- Diagnostics for escape failures (returning a borrow of a local) must point at the **return expression and the local's definition**, since there's no `&` token to anchor on. The `root_provenance` enum (`Param/Static/Local`, mandated by the feasibility MVP) carries exactly the info the diagnostic needs ("`x` is local; a returned `&T` must borrow a parameter").

---

## 3. Reference RETURN types — `func first() -> &T`

### Surface
```kestrel
extend Buffer {
    // shared borrow of an element; lifetime tied to self (the receiver)
    func first() -> &Element;
}
```

### Does the receiver convention imply the return's provenance?
In the MVP, **a returned `&T` must bottom out in a parameter (including `self`)** — that *is* the escape proof (`borrow_source` traces to a `Borrow`/`MutBorrow` parameter root). So provenance is structurally tied to *some* parameter. The question is whether the **receiver convention** alone fixes it.

- A `borrowing` (default) receiver can yield a returned `&T` borrowing `self` — the common accessor case. `func first() -> &Element` on a `borrowing` receiver means "the result borrows `self`; it cannot outlive the `self` you called it on."
- A `consuming` receiver **cannot** return `&T` of `self` — `self` is destroyed at return, so the borrow would dangle. This must be an error (E-class diagnostic), not silently accepted.
- A `mutating` receiver returning `&mutating Element` is the natural "mutable accessor," **but the feasibility doc forbids `&mutating` returns in v1** (Trap §7: a returned mut-ref freezes the source across the return boundary, which the per-block verifier can't enforce). So: `mutating` receiver may return **shared** `&T` in v1; `&mutating` returns are deferred.

### How it reads next to the receiver
```kestrel
extend Matrix {
    func   row(at i: Int) -> &Row;            // borrowing receiver (default), shared borrow of self
    consuming func drain() -> &Row;           // ERROR: consuming self can't yield a borrow of self
    mutating func cell(at i: Int) -> &Cell;   // OK in v1 (shared); &mutating return deferred
}
```

### Options for provenance specification
**Option A — Implicit single-parameter provenance (MVP).** With exactly one reference-eligible source in scope (almost always `self`, or the single `&T` param), the compiler infers which parameter the return borrows. No annotation.
- Pro: zero syntax; matches "lifetimes inferred" Stage-1 goal; matches the MVP's single-source `root_provenance`.
- Con: ambiguous when there are *two* borrowable params (`func pick(a: &T, b: &T) -> &T`) — v1 **rejects** the multi-source case (per Trap §7: keep the model able to *represent* multi-source, but reject it in v1).

**Option B — Explicit provenance annotation** (Swift `@lifetime(borrow self)` / Mojo `-> ref[self]`).
- Pro: the proven-safe path — both Swift and Mojo *require* the source be written, not inferred, precisely for returns. Disambiguates multi-source.
- Con: net-new annotation syntax; contradicts Stage-1's "inferred lifetimes" goal; overkill when 95% of returns borrow `self`.

### RECOMMENDATION ✅ — **Option A (implicit, single-source) for v1; reserve an Option-B annotation slot for the multi-source case.**

- v1: a returned `&T` infers its provenance as **the unique reference-eligible parameter root** (overwhelmingly `self`). The receiver convention **constrains but does not alone fix** provenance: `borrowing`/`mutating` receivers may yield `&T`-of-self; `consuming` receivers may not (hard error).
- **`&mutating` returns are not allowed in v1** (defer per Trap §7); a `mutating` receiver may still return a shared `&T`. *(Superseded 2026-06-09: ban lifted — `references-gaps.md` §10.4; a `&mutating` return must root at a mutable source.)*
- Multi-source returns (`-> &T` with two `&T` params) are a **clean rejection** in v1 with a diagnostic that says "ambiguous borrow source; v1 supports returning a borrow of a single parameter." This holds the line where Swift/Mojo independently landed (refuse to infer multi-source return provenance) without paying for annotation syntax until it's needed. When the annotation is added later, spell it to reuse the receiver/param vocabulary (e.g. a `from:` clause) rather than importing `@lifetime`.

---

## 4. Interaction with existing keywords — reconciling param-convention vs reference-type

This is the most important *coherence* decision. Kestrel will have **two surfaces that both mean "mutable by reference"**:

| Surface | Where | Lowers to | Call site |
|---|---|---|---|
| `func f(mutating on g: Grid)` | param **access mode** keyword (before label) | `ParamConvention::MutBorrow` → `PassMode::ByRef` | invisible (`f(on: g)`) |
| `func f(g: &mutating Grid)` | reference **type** (after colon) | `ParamConvention::MutBorrow` → `PassMode::ByRef` | invisible (`f(g)`) under §2 rec |

**They lower to the identical convention.** This is real redundancy and must be resolved deliberately.

### Grounding on the collision (verified)
- `mutating` is already three-way overloaded: receiver modifier (`mutating func`), param access mode (`mutating x:`), and function-type param marker (`(mutating T) -> R`). Adding `&mutating T` is a fourth role.
- `is_label_keyword()` **excludes** `Token::Mutating` and `Token::Consuming` (lexer:749-794) — this must be preserved so `mutating`/`consuming` can't be parsed as labels.
- The function-type position is where the two surfaces can *visually collide*: `(mutating Grid) -> R` (existing Design B) vs `(&mutating Grid) -> R` (new). The emitter must keep them distinct CST shapes.

### Options for reconciliation

**Option A — Two surfaces, defined equivalence.** Keep `mutating on g: Grid` (the *param access mode*) and add `&mutating Grid` (the *reference type*) as **exact synonyms** for params; document one canonical form. References' real new power is that `&T` is also a **return type and (later) a storable type**, which the param keyword can't express.
- Pro: no breaking change; `mutating` params keep working; `&T` adds returns/storage on top.
- Con: two ways to write a mutable param param. Style drift; "which do I use?" churn.

**Option B — Reference type is the canonical surface; param keyword becomes sugar.** Treat `mutating on g: Grid` as **sugar for** `g: &mutating Grid` (and a bare borrowing param... stays bare, since `&T` param would be the explicit form of the default borrow). One semantic model: a param's convention *is* its type's reference-ness.
- Pro: **single source of truth** — convention lives in the type, exactly as Design B already threads it through `TyKind::Function.conventions` and `ResolvedTy::Function.conventions`. The param keyword is pure surface sugar over the type.
- Pro: matches Swift (`borrowing`/`consuming` modifiers) *and* the type-level escapability marker coexisting; matches Mojo where the convention is the type's origin-ness.
- Con: requires deciding the desugaring direction precisely; `consuming` has no `&`-type equivalent (consume = by-value owned, not a reference), so `consuming` stays a keyword. So it's not a clean "everything is a ref type" — only the *borrow* conventions map to ref types.

**Option C — Deprecate the param keyword for the by-reference case.** Make `&T`/`&mutating T` the *only* way to spell a borrow param; reserve `mutating`/`consuming` for **receivers only**.
- Con: **breaking change** to shipped `mutating on grid: Grid` syntax and to Design B's closure-param work, which is uncommitted-but-complete. Too disruptive; rejected.

### RECOMMENDATION ✅ — **Option B (reference type is canonical; param keyword is sugar over it), with `consuming` remaining a by-value keyword.**

One coherent story:
- A parameter's **convention is a property of its type**: `g: Grid` = borrow (default), `g: &Grid` = explicit shared borrow (same as default), `g: &mutating Grid` = mutable borrow, `consuming g: Grid` = owned/by-value (no `&`, because consuming is *not* a reference — it transfers ownership).
- `mutating on g: Grid` is **defined as sugar for** `g: &mutating Grid` (label `on`, name `g`). Both produce `ParamConvention::MutBorrow`. There is **one** semantic notion (the convention on the type), two spellings, with the type-form canonical for docs/formatting.
- This makes references the *generalization* of the existing keyword: the keyword spells the convention for a param; the type spells the **same** convention but **also** works as a return type and (Stage 2) a field/local type — places a param keyword structurally cannot reach.
- **`consuming` stays a keyword**, not a `&`-type — it's the @owned/by-value transfer, not a borrow. This keeps the `&`-family meaning *exactly* "borrow" (shared or mutable), never "owned." Clean.

**Function-type collision handling (must-do):** in `(... ) -> R`, the existing Design B marker `(mutating T)` and the new `(&mutating T)` must be **reconciled to one canonical emission**. Recommend: the function-type param list standardizes on the **type form** `(&mutating T)`, with `(mutating T)` accepted as sugar, both emitted as the *same* `TyRef`-wrapped child so the positional `Mutating`-scan in `ast_type_from_cst` continues to work unchanged. This avoids a third interpretation of `mutating` inside `TyList`.

> Captured pattern worth recording (ask before adding to an AGENTS.md): *"convention lives on the type; param keywords are sugar over the type's reference-ness; `consuming` is the lone by-value exception that is never a `&`-type."*

---

## 5. Place / lvalue rules — what you can take a `&` / `&mutating` of

A borrow needs a *place* (an addressable location) as its source. Stage-1 rules:

| Source | `&T` (shared) | `&mutating T` |
|---|---|---|
| `var x` (mutable local) | ✅ | ✅ |
| `let x` (immutable local) | ✅ | ❌ — can't mutably borrow an immutable binding (mirrors E200/E203 mutability checks) |
| `self.field` (on `mutating`/`var` receiver) | ✅ | ✅ if the field is in a mutable place |
| `self.field` (on `borrowing`/`consuming` receiver) | ✅ | ❌ |
| subscript `a(i)` | ✅ | ✅ if the subscript yields a mutable place |
| **temporary** (`&makeThing()`, `&(a + b)`) | ⚠️ see below | ❌ |
| parameter (the `&T`/`&mutating T` param itself) | ✅ (re-borrow) | ✅ (re-borrow) |

### Decisions

**Mutable-place requirement for `&mutating`.** `&mutating` of a `let`, of a field on a non-mutable receiver, or of an immutable subscript is an **error**, reusing the existing mutability classification (`is_mutable_base` / `classify_mutability`, the E200/E201/E203 machinery Design B already relaxes for inferred mutating params). This is *not* a borrow-checker — it's the same lvalue-mutability check Kestrel already runs for assignment. **Crucially: this is mutability-of-place, not exclusivity.** Per the explicit project decision ("we don't need exclusivity for mutating references"), there is **no** check that the place isn't *also* borrowed elsewhere — may-alias means aliasing the same mutable place is permitted.

**Temporaries.** Borrowing a temporary (`&makeThing()`) needs the compiler to materialize the temporary into a scope-lived slot, then borrow it. The feasibility doc flags `&(if c {a} else {b})` as landing in the `mir3_ossa_inline_if_operand_ice` hole (operand-position `@owned` expressions ICE today). So:
- **Shared `&temp`** — *defer to post-MVP.* It requires temporary-lifetime-extension the verifier doesn't model, and the inline-if-operand ICE blocks it. In the MVP, `&` sources must be **named places** (var/let/field/subscript/param), not arbitrary expressions.
- **`&mutating temp`** — **never** (a mutable borrow of a temporary is meaningless; the mutation would be discarded).

**Returnable subset (MVP).** Of the places above, only those whose **root is a parameter** can back a *returned* `&T` (§2/§3). Borrowing a local for *in-function use* is fine; borrowing a local and *returning* it is the escape error.

### RECOMMENDATION ✅
- **`&` / `&mutating` sources are named places only in the MVP** (var, let [shared only], field-of-mutable-place, subscript-yielding-place, the ref params themselves). No temporary borrows in v1.
- **`&mutating` requires a mutable place**, enforced by reusing the existing lvalue-mutability checks (no new analysis) — *place-mutability, not exclusivity.*
- **Returnable ⊆ places rooted at a parameter** (the escape proof). Local-rooted borrows are usable in-function but not returnable in v1.

---

## 6. Mutability & aliasing surface — how MAY-ALIAS shows up (and the call-site question)

### The MAY-ALIAS decision is *mostly invisible in syntax* — by design
The project has decided `&mutating` **may alias** (no exclusivity), confirmed twice: the feasibility §10 ("relax the verifier's existing read-during-mut-borrow check") and the Design B decision ("we don't need exclusivity for mutating references… misuse is the programmer's responsibility, like unchecked `Pointer`"). The surface consequences:

- **No exclusivity annotations.** There is no `&uniq`, no two-phase-borrow marker, no `noalias`-style opt-in. The aliasing model is "anything goes for mutable places," so there is **nothing to spell**. This is the opposite of Hylo's `inout` (exclusive by mandate) and Mojo's `mut` (exclusive, compiler-enforced) — and that absence *is* the surface.
- **The only verifier rule that changes** is Check 5 (`assert_readable`, read-during-mut-borrow), which is **relaxed**, not exposed. No syntax.
- **Soundness consequence to document (not surface):** with exclusivity gone, soundness rests entirely on the escape/lifetime checker — there is "no linearity backstop underneath." The Stage-2 storage restrictions (no aliased `&mutating` into RcBox/escaping closures) must be written before reference-in-aggregate ships. None of this is Stage-1 surface, but it belongs in the design record so Stage 2 doesn't ship a UAF.

### Is `&mutating` visible at the call site?
Per the verified memory rule — **"mutating is declaration-only; omit at call site"** — and the §2 recommendation (params infer, no marker), the answer must be **consistent**:

**RECOMMENDATION ✅ — `&mutating` is NOT visible at the call site. Calls look identical whether the param is `&T`, `&mutating T`, or by-value.**

```kestrel
buffer.fill(with: source);   // 'with' could be &T, &mutating T, or T — signature decides
```

This is the only choice consistent with both (a) the shipped `mutating`-param call site and (b) Design B's inferred conventions. The cost — call sites don't show mutation — is paid down by the **LSP inlay hint / hover** surfacing the convention (shared infrastructure with `mutating` params and Design B, §2). A reader who wants to *see* mutation at the call site gets it from the editor, not the source text. This matches Kestrel's existing stance and avoids a third call-convention dialect.

> Contrast for the record: Hylo *does* mark the call (`f(&x)`) and Mojo enforces exclusivity with a hard error. Kestrel forgoes both — buying the simpler timeline (no exclusivity inference) at the cost of call-site visibility (delegated to tooling) and aliasing-based optimization (forfeited `noalias`). This is the deliberate trade already adjudicated in feasibility §10.

---

## 7. Worked example — recommended syntax, end to end

A small but real snippet: a ring buffer with a reference *parameter* (mutable, in-place, no COW clone) and a *returned* shared reference rooted at the receiver. All in the recommended surface (semicolons required; subscripts use parens; single-name params positional, two-name labeled; `mutating` invisible at call sites).

```kestrel
struct RingBuffer {
    private var slots: [Int];
    private var head: Int;
}

extend RingBuffer {
    // Shared borrow returned from a `borrowing` (default) receiver.
    // Provenance inferred: the result borrows `self`; it cannot outlive
    // the RingBuffer it was called on. No annotation needed (single source).
    func peek() -> &Int {
        return self.slots(self.head);   // &Int rooted at `self` (a parameter) → escapes OK
    }

    // `&mutating Int` PARAMETER (canonical reference-type form of `mutating value: Int`).
    // Passed by-ref; mutated in place; no buffer clone. May alias — no exclusivity check.
    mutating func push(_ value: Int, into sink: &mutating Int) {
        sink = self.slots(self.head);   // write through the mutable borrow
        self.slots(self.head) = value;  // field-subscript set (in-place, COW-mutated via Design B)
        self.head = (self.head + 1) % self.slots.count;
    }
}

func demo() {
    var ring = RingBuffer(slots: [0, 0, 0], head: 0);
    var carry = -1;

    // Call site is convention-blind: `carry` looks passed-by-value, but the
    // signature says `&mutating Int`, so the compiler borrows it. No `&` written.
    ring.push(7, into: carry);          // carry is mutated in place; no marker, no clone

    // Returned shared reference, used immediately. `head` element of `ring`.
    let current = ring.peek();          // current: &Int, borrows `ring`
    print("current=\(current), carry=\(carry)");
}                                       // `ring`'s borrow ends at scope exit; nothing dangles
```

What each line exercises, mapped to decisions:
- `func peek() -> &Int` — §3 return ref, provenance = `self` inferred, escape proof = parameter-rooted.
- `into sink: &mutating Int` — §1 type syntax, §4 canonical ref-type form of the `mutating` param.
- `ring.push(7, into: carry)` — §2 inferred borrow (no `&` at call), §6 invisible `&mutating`.
- `sink = ...` — §5 `&mutating` of a mutable place (`carry` is `var`); place-mutability satisfied; no exclusivity check.
- `let current = ring.peek();` — a returned `&T` bound to a `let`. *(Note: per §2 the MVP may restrict named ref-bindings; if so, `peek()` would be required to be consumed inline — `print("\(ring.peek())")` — and `let current = ...` lands in the deferred binding form. Shown here as the target end-state.)*

---

## 8. Open questions for the maintainer

The genuinely undecided calls, each blocking a concrete implementation choice:

1. **Named reference bindings in Stage 1 — in or out?** §2 recommends deferring `let r = &x;` to keep Stage 1 type-parser-only and avoid the `a & b` expression-ambiguity. But the worked example's `let current = ring.peek();` is natural and useful. **Decision: does the MVP allow binding a returned `&T` to a `let`, or must returned refs be consumed inline?** (If allowed, we incur the prefix-`&` expression disambiguation now and adopt the §2-Option-C visible cue for bindings.)

2. **`mutating` param keyword: sugar (§4 Option B) or coequal surface (§4 Option A)?** The recommendation is "type is canonical, keyword is sugar," but that requires committing to a desugaring and updating Design B's function-type emission to the canonical `(&mutating T)` shape. **Decision: do we formally make `mutating on g: G` desugar to `g: &mutating G`, or keep them as two documented-equivalent surfaces?** (Affects formatter, docs, and whether `kestrel-doc` shows one form.)

3. **Shared `&temp` (borrow of a temporary) — defer or design now?** §5 defers it (blocked by the inline-if-operand ICE). But `&(a + b)` / `&makeThing()` as a *parameter* argument is a plausible early ask. **Decision: is "named places only" an acceptable MVP limitation, or must temporary-borrow be in v1?** (If in v1, it's coupled to clearing `mir3_ossa_inline_if_operand_ice` first.)

4. **Multi-source return provenance — reject (v1) vs annotate (now).** §3 recommends rejecting `func pick(a: &T, b: &T) -> &T` in v1. **Decision: is the clean rejection acceptable, or do we need the annotation (`from:`-style) in the first release?** Swift and Mojo both *require* the annotation; this is the one place the "inferred lifetimes" goal collides with reality.

5. **`&mutating` return — confirm the v1 prohibition.** Feasibility Trap §7 says no `&mutating` returns in v1 (the per-block verifier can't enforce the cross-return freeze). A mutable accessor (`func cell(at:) -> &mutating Cell`) is a *very* common ask. **Decision: confirm `&mutating` returns are deferred, accepting that mutable accessors must instead take a `&mutating` out-param or use the Design-B `modify { }` closure pattern in the interim.** *(Answered 2026-06-09: prohibition rejected by the maintainer — `&mutating` returns are allowed, gated by the mutable-root rule. See `references-gaps.md` §10.4.)*

6. **Call-site visibility — fully invisible (recommended) vs an optional opt-in marker.** §6 recommends no call-site marker, matching `mutating` params. **Decision: do we want an *optional* explicit `&` at call sites for readability (`f(&x)` accepted but not required), or strictly no marker?** An optional marker is a middle path but reintroduces the expression-`&` parse concern and a "which style?" lint question.

7. **Does `consuming` ever get a `&`-type form?** §4 keeps `consuming` a pure keyword (it's by-value, not a borrow). **Confirm:** `&`-types mean *only* borrow (shared/mutable), never owned — i.e. there is deliberately no `&consuming T` / `&owned T`. (Recommended yes; flagged because it's a vocabulary-closure decision that's hard to reverse.)

---

*Recommendations summary (all marked ✅ above): `&T` / `&mutating T` type syntax (§1); params infer the borrow, no call-site marker (§2, §6); returned `&T` infers single-parameter provenance, no `&mutating` returns in v1 (§3); convention-lives-on-the-type with `mutating` param as sugar and `consuming` staying by-value (§4); named-place sources only, place-mutability not exclusivity (§5). The four load-bearing open questions for the maintainer are #1 (named bindings), #2 (sugar vs coequal), #4 (multi-source returns), and #5 (`&mutating` returns).*

Source files referenced (absolute paths): `/Users/dino/Documents/Projects/kestrel/docs/references-prototype/references.md`, `/Users/dino/Documents/Projects/kestrel/lib/kestrel-parser/src/ty/mod.rs`, `/Users/dino/Documents/Projects/kestrel/lib/kestrel-parser/src/common/parsers.rs`, `/Users/dino/Documents/Projects/kestrel/lib/kestrel-ast-builder/src/ast_type.rs`, `/Users/dino/Documents/Projects/kestrel/lib/kestrel-lexer/src/lib.rs`. Design-B precedent grounded in user memory `mutating_closure_params_feature.md` and `kestrel_mutating_callsite.md`.
