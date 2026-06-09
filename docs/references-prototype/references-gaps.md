# References — Gap Analysis (Third Audit)

**Status**: Research / feasibility companion
**Date**: 2026-06-09
**Companions**: [`references.md`](references.md) (feasibility), [`references-syntax.md`](references-syntax.md),
[`references-plumbing.md`](references-plumbing.md), [`references-tests.md`](references-tests.md),
[`references-prior-art.md`](references-prior-art.md)

This is a third source-grounded audit, six days after the first two. It does three
things: (1) re-verifies the load-bearing anchors against the current tree, (2)
checks the four companion docs against *each other* — they were written by
different passes and disagree in places — and (3) hunts for design and
implementation holes none of them cover. Findings are ranked by how much they
change the plan.

## TL;DR — what changes

1. **The flagship use case is not implementable with the MVP as specified.**
   `Array.first() -> &Element` cannot pass the "borrow_source traces to a Param"
   escape proof, because element access necessarily goes through a raw
   `Pointer` load that erases provenance. The MVP actually ships *only*
   `&self.field`-style direct projections. Fixing this needs an unsafe
   Pointer⇄reference bridge — direction adopted 2026-06-09:
   `Pointer(to:)` / `pointer.value` / `pointer.mutatingValue` (§3.1).
2. **The core use-semantics of a reference value are undefined.** No doc decides
   how you *read through* a `&T` — there is no deref operator, member resolution
   is nominal-only with zero see-through machinery, and the test matrix invents
   `*r` syntax that doesn't exist in Kestrel (§4).
3. **Function types cannot carry a return convention** — so a
   reference-returning function used as a *value* (`let f = peek;`) silently
   gets the wrong ABI. v1 must reject this; no doc mentions it (§5.1).
4. The docs **contradict each other** on surface syntax: the test matrix uses
   explicit `&x` expressions and `&`-before-name params that the syntax doc
   later rejected, and the syntax doc's own worked example violates its §5
   place rules (§2).
5. The plumbing checklist has **no LLVM backend stages** (the backend landed
   in-tree after the audit) and **no implementation sites for any of the
   negative rules** that make Stage 1 safe (§6, §7).
6. Good news: the **prerequisite bug list is mostly cleared** (tuple drop/copy
   elaboration fixed in 22084a42; the expand.rs copy-elaboration cluster is
   committed), all major verifier anchors re-verified with only small line
   drift, and overload collision is a non-problem — `f(x: T)` vs `f(x: &T)`
   already collide under label-based duplicate detection, which is the safe
   default (§8).

---

## 1. Anchor re-verification (2026-06-09)

All structural claims in `references.md` Appendix A were re-checked. Everything
holds; only line numbers drifted. Updated table for the ones that moved:

| Anchor | Was | Now |
|---|---|---|
| `verify.rs` `try_consume` early-return / blocking loop | 322 / 328-334 | 321-325 / 329-334 |
| `verify.rs` `assert_readable` (Check 5) | 373 | 373-393 |
| `verify.rs` `Return` arm (no-op for `@guaranteed`) | 989 | 984-990 |
| `verify.rs` Check 4 forwarded `@guaranteed` block args | 1012-1033 | 1009-1030 |
| `mir-lower/body/mod.rs` copy-at-return guard | 477-489 | 479-486 |
| `mir-lower/body/expr.rs` second copy-at-return | 257-265 | 260-269 |
| `mir-lower/body/mod.rs` `alloc_guaranteed` | 663 | 666 |
| `mir-lower/body/mod.rs` `set_terminator` force-EndBorrow | ~1820 | 1854-1882 |
| `add_guaranteed_block_param` | panic-string only | still panic-string only (`builder.rs:95,125`) |

Confirmed unchanged: verifier is a single forward BFS, one visit per block, no
fixpoint (`verify.rs:42-77`, with a comment asserting per-block isolation is
intentional); `ValueDef.borrow_source` is single-hop `Option<ValueId>`
(`value.rs:4-22`); all six borrow instructions exist; `MirTy` has no `Ref`
variant; `Constraint` has no inequality/outlives shape (13 variants now, but
all membership/equality-style).

One nuance worth recording: `assert_live`'s pass-through for values not in
`self.owned` is *implicit* (it only pattern-matches `Some(Consumed)`), so once
`ret_borrow` returns are allowed the verifier gives literally zero signal on
them — confirming the docs' "no backstop" claim at the code level.

---

## 2. The docs contradict each other

The four companion docs were written by different passes and were never
reconciled after the syntax decisions landed. Three concrete conflicts:

### 2.1 The test matrix uses syntax the syntax doc rejected

`references-tests.md` is written entirely in **explicit-borrow** style:
`read_value(&t)`, `increment(&mutating x)`, params spelled `&tok: Token` /
`&mutating counter: Int64` (sigil before the *name*), and dereference via `*r`.
The syntax doc's §2/§6 recommendation is the opposite — borrows are **inferred**
at call sites, no expression-level `&` exists in Stage 1, and the canonical
param form is `tok: &Token` (sigil on the *type*). And `*r` is not Kestrel
syntax under any proposal — `*` is only infix multiplication. **Every test file
in the matrix needs rewriting against the syntax decisions before it can pin
anything.** This matters beyond housekeeping: rewriting them forces the deref
question (§4), which is currently unanswerable.

### 2.2 The plumbing checklist assumes the syntax design that lost

Plumbing Stage 9 says "a reference expression `&x` produces a `TyKind::Ref`"
and "no coercion constraint is needed." Under the adopted Option B (inferred
borrows), there *are no* `&x` expressions — instead, every use of a returned
`&T` value needs an implicit **`&T → T` use-site coercion** (deref-read), and
if reference params are represented as `Ref`-typed (rather than as
conventions, see §5.3), every argument position needs the **`T → &T`**
direction too. `Coerce` is directional (`from → to`, `constraint.rs:25-30`)
and solves unify-first-then-`FromValue`-promotion (`solver.rs:1295-1450`), so
the mechanism can host this — but it's net-new solver work the checklist
counts as zero.

### 2.3 The syntax doc's worked example violates its own §5

```kestrel
func peek() -> &Int {
    return self.slots(self.head);   // claimed: &Int rooted at self
}
```

`self.slots(self.head)` is a **subscript witness call returning an owned
temporary** (see §3) — so the implied borrow is a borrow of a temporary, which
§5 of the same doc explicitly defers ("`&` sources must be named places…
no temporary borrows in v1"). The §5 place table's "subscript a(i) ✅" row
overstates what exists: subscripts are method calls, not places; there is no
"subscript yielding a place" in the compiler outside the setter-assign special
form. The only §5-legal returnable borrow in the MVP is a **direct field chain
on a parameter** (`&self.field.subfield`).

---

## 3. The provenance gap: the flagship use case doesn't work

This is the most consequential new finding. The motivating examples throughout
the doc set — `Array.first() -> &Element`, Dict lookup without clone (the
kestrel-wall `getValue` deep-clone class), `&arr(0)` — all require returning a
reference to a **heap element**. Verified against the stdlib:

- `Array[T]` wraps `CowBox[ArrayStorage[T]]`; element access is
  `storage.valuePtr()` → raw `Pointer[T]` → `.offset(by: i).read()`
  (`lang/std/collections/array.ks`, `slice.ks`, `cowbox.ks:84-86`,
  `rcbox.ks:98-101`). Every element read **copies** via `Pointer.read()`.
- `Pointer` is a plain value — no ownership, no `borrow_source`, nothing the
  escape checker can walk. The moment an access path passes through a
  `Pointer` load/offset, **the provenance chain is severed**.
- Subscript calls lower as ordinary witness method calls returning by value
  (`SeqIndex.readSeq`); there is no concept of a call returning a *place*.

Consequence: the MVP's escape proof ("`borrow_source` traces through
projections to a Param root") covers `StructExtract`/`BeginBorrowAddr` field
chains only. It can **never** reach an Array/Dict/String element, because the
stdlib's own representation interposes a raw pointer. So the MVP as specified
ships:

- reference parameters (already free), and
- returned borrows of **direct fields of parameters** — and nothing else.

That is a much thinner feature than the docs' examples advertise, and notably
it does *not* address the COW-clone amplifiers cited as motivation.

### 3.1 The missing piece: the Pointer⇄reference bridge (direction adopted 2026-06-09)

Maintainer direction: make `Pointer` itself the bridge, unsafe by doc-comment
contract exactly like the rest of its API — no new attribute, no `unsafe`
keyword, consistent with the existing "misuse is the programmer's
responsibility, like unchecked `Pointer`" stance:

```kestrel
// Capture the address of a borrowed place. UNSAFE: the pointer
// is a plain value that outlives the borrow; validity past the
// borrow is the caller's responsibility.
init(to value: &T)
init(mutating value: &mutating T)   // two labels, not an overload — (name,
                                    // labels) duplicate detection means `to:`
                                    // can't be overloaded by ref-ness (§5.6)

// Reinterpret the pointee as a reference. UNSAFE: valid only while
// the pointee's storage is. Unlike read(), requires no T: Copyable —
// which is the point.
var value: &T
var mutatingValue: &mutating T
```

The two halves are asymmetric in cost:

- **`Pointer(to:)` is nearly free and independent of all return-ref
  machinery.** It is a borrow *parameter* (`PassMode::ByRef` already delivers
  the address) plus an address-capture intrinsic. Provenance is *dropped* at
  this boundary, which is fine — pointers are untracked values, same as
  `valuePtr()` today. This half can ship in Stage 0.5 and gives Kestrel
  Swift's `withUnsafePointer` without the closure.
- **`.value` / `.mutatingValue` is where the design decision lives.**
  Mechanically the result is an ordinary borrow **of the receiver pointer**
  (normal `BeginBorrow` machinery, result registered against the receiver —
  no special lowering). But the escape walk then bottoms out at a
  Pointer-typed value, so `root_provenance` needs a fourth root kind,
  **`PointerDerived`**, that the escape checker allows to escape: the
  reference *inherits the pointer's safety contract* (§10.3 explains why
  this cannot be upgraded to a checked root short of Stage 2). (This
  supersedes the earlier `asBorrow(of: owner)` sketch — an owner argument
  the compiler can't verify is documentation pretending to be a check.)

The resulting language guarantee is exactly Rust's factoring: **references
that never pass through `Pointer` are compiler-checked; `Pointer`-derived
references are precisely as unsafe as the pointer they came from.** The
unsafety also encapsulates the way `Vec`'s does: `Array.first() -> &T`
fabricates internally (storage pointer → `.value`), but the *caller-side*
result is still registered `@guaranteed` with `borrow_source` = the receiver
argument (`alloc_guaranteed`), so callers get the safe discipline — the
returned ref is scoped to `arr` — while the unchecked step stays inside the
stdlib, where "storage outlives self" is locally verifiable by a human.

Two consequences to fold into the design:

1. **The v1 "no `&mutating` returns" ban is lifted entirely** (maintainer,
   2026-06-09 — §10.4 has the derivation). Its replacement is one uniform
   rule for *all* reference returns, shared or mutable: the root must be a
   parameter (a **mutable** one for `&mutating`) or `PointerDerived`.
   Caller-side writes through a returned mut-ref are covered by existing
   place-mutability checks plus the may-alias decision — but mutable
   accessors overlap the existing subscript-setter lowering
   (`field_subscript_set` / `try_lower_setter_assign`), and `arr.at(i) = v`
   must reconcile to *one* lowering path, not two (§10.4).
2. **Property getters become ref-returning functions.** `.value` as a
   computed property means the `ret_borrow` bit must flow through
   property/member resolution (`MemberResolution.return_type`,
   `LowerCallableReturnType`) from day one — and any user type can then
   declare `var x: &T { ... }`, so the same body rules (param-rooted or
   Unchecked-rooted, no named bindings) apply there too.

One footgun to document on `Pointer(to:)`: composing it with `.mutatingValue`
is a const-cast (`Pointer(to: sharedBorrow).mutatingValue` writes through a
shared borrow). That's Pointer-class unsafe by the contract above, but the
doc comments should call it out explicitly.

Note the COW interaction is actually benign for *shared* refs: a `&element`
fabricated from `CowBox` storage is kept alive by the borrowed Array param
(refcount held); a writer sharing that storage copies *itself* away. It
becomes unsound only for `&mutating` into shared storage — already rejected
in v1 — and for any future relaxation of the named-place rule.

### 3.2 The alternative the prior-art doc undersells: Kestrel already has half a projection

`references-prior-art.md` §2.2 recommends evaluating Hylo-style projections as
a lower-risk substitute. Worth stating concretely: **the mutation half already
ships.** Design B's `(mutating T) -> R` closures + `storage.modify { (mutating
s) in ... }` (used pervasively in `array.ks` — e.g. lines 617, 651, 706) *are*
inverted-control projections: in-place access to heap interior with no
reference type, no escape checker, no return-convention surgery. The missing
halves are (a) the ergonomic surface (`arr.with(at: i) { ... }` reads as
closure noise) and (b) the *shared-read* equivalent (`Pointer.with` exists but
isn't surfaced on collections). A small stdlib+sugar investment here delivers
most of the collection-access value with **zero** new compiler analysis, and
should be costed against the provenance-intrinsic path before Stage 1 is
scheduled.

---

## 4. Undefined core semantics: what does *using* a `&T` mean?

No document decides the read-through story, and the compiler offers no free
ride:

- **Member resolution is nominal-only.** `solve_member` resolves via the
  receiver's nominal entity (`solver.rs:2620-2750`); Tuple/Function receivers
  hit a hard NoMember error; there is **no auto-deref or see-through machinery
  anywhere** — not for Optional, not for Pointer. `TyKind::Ref.entity()` would
  be `None`, so `r.count`, `r.method()`, `r != 99`, and `"\(r)"`
  (interpolation needs a conformance) all resolve to *nothing* without new
  machinery.
- The syntax doc's worked example silently assumes **transparent-place
  semantics** (`sink = …` writes through a `&mutating` param; `\(current)`
  reads through a `&T`) without ever stating the rule.

The decision has three candidate shapes:

- **(a) Transparent place (C++-reference-like).** A ref-typed value used as an
  rvalue auto-reads (bitwise copy for Copyable pointees; error for
  non-Copyable unless member-projected); member access projects through;
  assignment through a `&mutating` writes through. Matches the worked example
  and the invisible-call-site philosophy. Cost: an implicit `&T → T` coercion
  applied at *every* use site (a pervasive insertion pass akin to literal
  defaults) plus member-resolution see-through — the single largest
  unaccounted inference work item.
- **(b) Explicit access method** (`r.value` / `r.deref()`): cheap to build,
  but it makes refs second-class in *ergonomics* too, and `deref` on a `&T`
  whose pointee is non-Copyable has no good answer.
- **(c) No ref-typed values reachable in v1**: returned refs may only appear
  as receivers of member access or as arguments that re-borrow — i.e. the
  ref evaporates at its first use. This is the cheapest sound cut and pairs
  with the "no named bindings" MVP restriction, but must then be stated as a
  rule with a diagnostic.

**Recommendation: (a) as the target, (c) as the MVP**, with (a)'s coercion
designed up front so (c)'s diagnostic doesn't paint it out.
*(Superseded 2026-06-09: a verified feasibility pass found (a) is only
~2-4 wk over (c) — member dispatch is a single funnel and MIR already
handles borrowed receivers transparently. **Decided: (a) transparent place,
no (c) detour — §10.5.** Rules + anchors:
`docs/plans/references/stage1/semantics.md`.)* Either way this is
a maintainer decision that belongs alongside open questions #1-#7 in the
syntax doc — arguably *before* #1, since the named-bindings question is
downstream of it.

---

## 5. Holes no document covers

### 5.1 Function values: there is nowhere to put `ret_borrow`

`TyKind::Function` / `ResolvedTy::Function` carry per-param `conventions`
(Design B) but the return is a bare `ret: TyVar`/`TyId` — **no return
convention exists in any function-type representation**, including
`MirTy::FuncThin/FuncThick`. The feasibility doc deliberately makes
`ret_borrow` a `MonoFunction` bit (not a type), which is right for direct
calls — but a reference-returning function taken as a *value* (`let f =
peek;`, a closure returning `&T`, a protocol witness pre-Stage-3) erases the
bit, and an indirect call through it uses the wrong return ABI: caller treats
the returned pointer-sized value as the pointee (or vice versa). Silent
miscompile, no diagnostic.

**v1 rule needed:** a function whose return type is a reference may not be
used as a value, captured, stored, or passed — only called directly. One
check at the FuncRef→FuncThick coercion site + closure return-type validation.
(Stage 2+, if function values must support it, the convention has to thread
through all three type representations and both backends — budget it then.)

### 5.2 `throws -> &T` puts a ref inside an enum payload

`throws` lowers structurally to `Result[T, E]` (sugar expansion in
`hir-lower/ty.rs:141-148`; MIR sees an ordinary enum return). So
`func f() throws -> &T` lands a reference in an enum payload — Stage-2
territory (lifetime-carrying aggregates, `MonoTypeKey` collapse) reachable by
one keyword. Same applies to `-> &T?` if optional-sugar composes with ref
types. **v1 rule needed:** reject `throws` + reference return and `Ref`
inside any sugar wrapper, at signature lowering.

### 5.3 Generic type arguments are a storage backdoor

Nothing in the plumbing prevents `&T` from binding to a type variable:
`Optional[&Int64]`, `Array[&Int64]` (then `.append(r)` *stores* a ref),
`id[T](x: T)` with `T := &Int64` flowing into arbitrary generic code that
stores/returns it. Stage 1's entire safety story rests on refs being
non-storable, and generic instantiation circumvents it. There is no existing
"this type may not be a generic argument" structural check to clone — Copyable
bounds are the nearest precedent and the
`copyable_mono_substitution_gap` memory shows that mechanism *already* can't
fully reject bad substitutions at the frontend (Inv-3b ICEs are the only
catch). Refs would inherit a strictly worse version: not an ICE but a UAF.

**v1 rule needed:** a structural "no `Ref` in generic argument position /
nominal type args / tuple elements / closure captures" check, enforced at
`kind_to_resolved` (the TyVar-resolution choke point) *and* at the
HIR type-annotation level. This is the blunt precursor of Stage 3's `Static`
bound; the docs jump from "no storable refs" (stated) to Stage 3 (principled)
without ever listing the Stage-1 enforcement mechanism.

Related decision with real ergonomic stakes: **`Optional[&T]` is banned by
this rule**, which means the natural shape of every lookup API
(`find(...) -> &V?`) is inexpressible until Stage 2. That strengthens §3.2's
case for the projection/closure surface as the collection-access answer.

### 5.4 Pattern matching on a ref scrutinee

`match ring.peek() { ... }` — patterns destructure through the scrutinee;
borrow-aware match (Rust's match ergonomics) is a whole subdesign. **v1 rule:**
reject ref-typed scrutinees (or auto-deref-copy for Copyable pointees under
§4(a)). Currently in no doc; the match analyzer (E310-E315) would need the arm.

### 5.5 Two sources of truth for the parameter convention

Today `ParamConvention` is computed from `AstParam.is_mut/is_consuming` flags
at signature lowering (`function_sig.rs:91-96`). The syntax doc's §4
recommendation makes `g: &mutating G` canonical with `mutating on g: G` as
sugar — meaning the convention is *also* derivable from the param's type. Two
derivation paths = drift. **Decide the normalization point**: recommended —
HIR signature lowering folds ref-typed params *into* `ParamConvention` and
strips the ref wrapper, so the rest of the pipeline (inference, MIR, both
backends) sees exactly today's convention-based representation, and
`TyKind::Ref` **only ever flows out of calls (return positions), never into
parameter types**. This single rule eliminates the `T → &T` argument coercion
(§2.2) entirely, keeps `bind_arguments` untouched, and shrinks the inference
blast radius to return-position refs. The same normalization should be a named
query (`FunctionRetConvention`-style) so MIR lowering, `MonoFunction`
creation, witness lowering, and member resolution all read **one** source for
`ret_borrow` instead of re-deriving it.

### 5.6 Overload collision — resolved, the safe way (good news)

Duplicate detection keys on `(name, labels)` only
(`duplicate_callable.rs:56-80`); parameter types don't participate. So
`func f(x: T)` and `func f(x: &T)` collide as duplicates today — no dialect of
convention-blind call sites can become ambiguous. Nothing to build; just
document it.

---

## 6. The LLVM backend doubles the codegen surface

The plumbing checklist predates the LLVM backend landing in-tree (2026-06-05)
and has zero LLVM stages. Verified: the two backends share **no**
classification/ABI code (only `kestrel-codegen/src/target.rs` is common) —
every Cranelift site in plumbing Stages 17-19 needs a twin:

| Plumbing stage | LLVM twin site |
|---|---|
| 17 classify | `kestrel-codegen-llvm/src/ty.rs:173` (add `Ref/MutRef` to the `Pointer` scalar arm) |
| 18a `return_mode` | `kestrel-codegen-llvm/src/abi.rs:40` (takes only `repr` today — `is_main` is handled at declaration time in `context.rs:294-296`, so the signature change is *simpler* than Cranelift's) |
| 18b `build_signature` | `abi.rs:61-88` |
| 19a `resolve_scalar` | `func.rs:63-80` (same load-through-ByRef-pointer behavior as Cranelift — same miscompile trap) |
| 19b `compile_return` | `terminator.rs:147,151` |

Small (~50 LOC) but **dangerous to forget**: the test suite executes via
Cranelift, so a missed LLVM twin is a wrong-ABI miscompile invisible to the
harness. The test plan needs at least one `KESTREL_BACKEND=llvm` execution run
of the ref-return tests (the harness currently has no backend-matrix concept —
add it to the references-tests.md proposed-extensions list next to ASan).

---

## 7. The missing half of the plumbing checklist: negative-rule enforcement

The checklist is entirely *positive* plumbing (new variants, new arms). Most of
Stage 1's actual safety comes from **rejections**, and not one has a listed
implementation site or diagnostic code. Inventory, consolidated from all docs
plus this audit:

| Rule | Natural home |
|---|---|
| returned ref must root at a param or `PointerDerived` (the escape check) | `verify.rs` Return arm + `root_provenance` |
| `&mutating` return must root at a *mutable* source (the blanket ban is lifted, §10.4) | same escape walk, one extra root predicate |
| no ref in `var`/`let` annotations, fields, tuple elements, closure captures | HIR type-position walk (pattern: `contains_opaque`) |
| no ref as generic type argument (incl. `Optional[&T]`, `throws -> &T`) | `kind_to_resolved` + signature lowering (§5.2, §5.3) |
| no multi-source returns (`cond ? &a : &b`) | escape checker (reject joined provenance) |
| `consuming` receiver can't return a ref of self | signature/receiver check |
| no `&mutating` from heap-shared (RcBox/CowBox) storage | `BeginMutBorrow` source classification |
| ref-returning function not usable as a value (§5.1) | FuncRef/FuncThick coercion site |
| no ref-typed match scrutinee (§5.4) | match analyzer |
| no ref crossing a block merge / loop | `set_terminator` (already force-ends; needs a diagnostic instead of silent drop) |

That's ~10 new diagnostics needing E-code allocation (per the analyzer
AGENTS.md conventions) and roughly a new analyzer's worth of work — call it
2-3 weeks — that appears in no estimate.

---

## 8. Prerequisite refresh & revised effort

`references.md` §11's "clear these first" list, updated:

- **Cleared since the audit:** tuple drop/copy elaboration (committed
  22084a42, symmetric copy/clone included); the expand.rs copy-elaboration
  cluster (`asslice_return_self_double_free`,
  `expand_not_copyable_nominal_collapse`, issue #125) is committed; `expand.rs`
  is no longer in the uncommitted working set.
- **Still open:** `diamond_conditional_move_let_drop_timing` (backlog; a
  conditionally-consumed `let r = &x` inherits it), the Rc-closure migration
  (planned, collides with Stage-2 ref capture), `06_match_arm_moves_payload`
  (needs borrow-aware move check — same machinery family as ref-aware match,
  §5.4).

Revised Stage-1 estimate, folding in this audit:

| Item | Delta |
|---|---|
| Original Stage-1 (10-14 wk) | baseline |
| Use-site deref semantics + member see-through (§4, if option (a)) | +2-4 wk |
| Negative-rule analyzer + diagnostics (§7) | +2-3 wk |
| Provenance intrinsic design + stdlib accessor work (§3.1) — *required for the feature to be worth shipping* | +2-3 wk |
| LLVM twins + backend-matrix test runs (§6) | +1 wk |
| Function-value rejection, throws/generic-arg guards (§5) | +1 wk |

**Realistic Stage 1: ~16-22 weeks**, and that buys field-accessor returns plus
collection accessors *only if* the unsafe intrinsic is accepted. The
alternative cut — skip return-refs entirely in v1, ship reference parameters +
a projection-style stdlib surface over the existing Design-B closures (§3.2) —
delivers the COW-clone-elimination value at a fraction of the cost and defers
every item in §§3-5. That option should be on the maintainer's table as
"Stage 0.5".

## 9. Additions to the maintainer's open-question list

Continuing the numbering from `references-syntax.md` §8:

8. **Deref/read-through semantics** (§4): ~~transparent place, explicit
   access, or evaporate-at-first-use?~~ **Decided 2026-06-09: transparent
   place (a), no (c) detour — §10.5.**
9. **The Pointer⇄reference bridge** (§3.1): fully decided —
   `Pointer(to:)` / `Pointer(mutating:)` inits and `.value` /
   `.mutatingValue` accessors, unsafe by doc-comment contract, public from
   day one (§10.2); pointer-derived refs may escape returns under the
   inherited-contract model (§10.3); the `&mutating` return ban is lifted in
   favor of the mutable-root rule (§10.4).
10. **Stage 0.5** (§8): ship ref params + projection sugar first, defer all
    return-ref machinery? This is the cheapest path to the kestrel-wall-class
    wins and matches Hylo's model more closely than Stage 1 does.
11. **Param representation** (§5.3): confirm refs normalize into
    `ParamConvention` at HIR lowering (TyKind::Ref in return positions only).
12. **Backend test matrix** (§6): is one `KESTREL_BACKEND=llvm` execution lane
    a release requirement for any ABI-touching feature?

---

## 10. Decisions adopted after this audit (2026-06-09)

Maintainer decisions, recorded here so the companion docs can be reconciled
against them.

### 10.1 `&mutating T` is assignable to `&T` (one-way)

A mutable reference may be used anywhere a shared reference is expected; the
reverse is an error (that direction is a const-cast and exists only via the
`Pointer` bridge, §3.1). Implementation notes:

- **This is a `Coerce` arm, not a unify arm.** The plumbing checklist's
  Stage 6 rule ("Ref ≠ MutRef — they don't unify") stays correct: unification
  is symmetric and must keep them distinct. The one-way direction lands in
  `solve_coerce` (before the `FromValue` promotion fallback) as
  `MutRef{T} → Ref{T}`.
- **At MIR level the downgrade is free *because of* the may-alias decision.**
  Both are pointer-width `@guaranteed` values; `borrow_source`/provenance
  carry over unchanged. Under exclusivity (Rust) a mut→shared downgrade needs
  reborrow bookkeeping — the mutable loan is suspended while shared reborrows
  live; with no exclusivity there is no loan to suspend, so the coercion is a
  bit-copy plus provenance carry. Another spot where may-alias buys
  simplicity.
- Under the conventions-normalized param representation (§5.3), the
  argument-position case (a `&mutating` param passed onward to a `&T` param)
  is already a plain re-borrow and needs no type-level coercion at all; the
  Coerce arm matters only where `TyKind::Ref`/`MutRef` actually appear —
  return positions and (later) bindings.
- Variance through compound types (function types, generic args) is moot in
  Stage 1 (refs are banned there, §§5.1-5.3); decide it when Stage 2 lifts
  those bans.

### 10.2 The Pointer bridge is public from day one

`Pointer(to:)` / `Pointer(mutating:)` / `.value` / `.mutatingValue` ship
public, not `lang`-gated. Rationale: `Pointer.read/write/offset` are already
public and equally dangerous; Kestrel's safety boundary is "did you touch
`Pointer`", not "are you the stdlib". Consequences:

- The escape checker's `Unchecked`-may-escape rule (§3.1) is user-reachable
  from day one, so the checked/unchecked line must be crisp in diagnostics
  and docs ("this reference originates from a `Pointer`; the compiler does
  not verify its lifetime").
- All four members must carry full `# Safety` doc sections (the stdlib
  doc-comment formula already defines the section), including the const-cast
  footgun (`Pointer(to: shared).mutatingValue`).
- Since Kestrel has a single write-capable `Pointer[T]` (no
  `UnsafeMutablePointer` split), the `to:`/`mutating:` init pair is
  intent-documentation plus a call-site place-mutability check, not a
  capability split. With §10.1's coercion, `init(to: &T)` alone would
  technically suffice; `init(mutating:)` is kept so that write-intent
  requires a mutable place and the const-cast stays an explicit opt-in
  rather than the default path.

### 10.3 Pointer-derived references: "borrows the pointer", inherits its contract

The maintainer framing: `pointer.value` **borrows the receiver pointer for as
long as the reference lives**. That is exactly how it lowers — an ordinary
receiver borrow, result registered `@guaranteed` against the receiver, normal
scope machinery, nothing special intra-function.

What that borrow can and cannot give:

- It ties the reference to the pointer **value's** scope. It cannot tie it to
  the **pointee storage's** validity — and only the storage can dangle.
  `Pointer` is a plain Copyable scalar whose own lifetime has no relationship
  to what it points at; `read()` after the pointee dies is already UB under
  the existing contract, and `.value` changes nothing about that.
- The compiler cannot tell a sound pointer-rooted return from a dangling one.
  In `Array.first()`, the pointer is a local temporary copied out of RcBox
  storage; in `func dangle() -> &Int64 { var x = 42; Pointer(to: x).value }`
  the pointer is *also* a local temporary. Identical shapes to the checker —
  the difference (heap storage kept alive by the borrowed receiver vs. a
  dying stack slot) lives in the pointee's provenance, which a bare pointer
  value does not carry.
- Upgrading this to a *checked* root requires the pointer itself to carry its
  origin's lifetime — `Pointer[T, origin]`, which is exactly Mojo's design
  and Stage 2's lifetime-on-types cost center, and which would break
  `Pointer`'s role as a plain storable field (`ArrayStorage.ptr`) and as the
  type of heap/FFI pointers that have no origin borrow at all.

Hence the root kind is named **`PointerDerived`** (not "Unchecked"), with the
semantics: *the reference inherits the pointer's safety contract*. Nothing
becomes less checked than today — `Pointer.read()` already carries the exact
same responsibility; `.value` is the same trust point returning a non-copying
view. References that never pass through `Pointer` remain fully verified.

Optional hardening (post-MVP): an intra-function lint for the provably-silly
case — a returned ref whose pointer was constructed by `Pointer(to:)` on a
local of the same function. Catches `dangle()` above; claims nothing beyond
it.

### 10.4 No `&mutating` return ban

The v1 prohibition on `&mutating` returns (feasibility Trap §7; syntax doc §3
and open question #5) is **dropped**. The original rationale dissolves under
decisions already made:

- The stated reason was "Check 5 cannot enforce the source's
  frozen-for-mutation state past `Return`." But the may-alias decision
  *already relaxes Check 5* — there is no freeze to enforce. The ban was
  protecting an invariant the language deliberately gave up.
- Lifetime safety for a mut return is the *same* escape proof as for a
  shared return — identical root walk, identical codegen (a pointer comes
  back either way). Mutability adds no new dangling mode.
- In v1, returned refs are expression-scoped (no named bindings) and cannot
  cross block merges, so a returned mut-ref's entire life sits inside the
  caller's current block chain — the region the verifier's existing
  intra-block machinery covers.

What replaces it — the **mutable-root rule**: `-> &mutating T` requires the
escape walk to bottom out at a *mutable* source — a `&mutating` param, a
`mutating` receiver, or `.mutatingValue` (`PointerDerived`). Deriving a mut
return from a shared `&` param stays an error (that would be a const-cast,
which exists only via the Pointer bridge). Same walk, one extra predicate on
the root — strictly simpler than the ban plus the carve-out it replaces.

Two notes for the record:

- **Accepted behavior, not a bug:** a returned `&mutating` into COW storage
  combined with sharing the container inside the same expression
  (`f(arr.at(0), arr)`) can make a write observable through the copy — a
  value-semantics violation. This is the same class the no-exclusivity
  decision already accepted for Design B closures; recorded here so it is
  deliberate.
- **Usefulness vs. surface:** with no named bindings, a returned `&mutating`
  is consumed by passing it onward to a `&mutating` param — that works day
  one. Using it as an assignment *target* (`arr.at(i) = v` through a mut-ref
  accessor) needs call-expression-as-place grammar and must reconcile with
  the existing subscript-setter lowering into a single path. That is
  scheduling, not a reason to ban returns.

### 10.5 Q8 decided: transparent place — option (a), no MVP detour

The use-semantics question (§4, open question 8) is **decided: (a)
transparent place**, chosen 2026-06-09 after a feasibility pass verified the
cost is ~2-4 wk over the evaporate cut, concentrated at 4 sites (one
`solve_member` peel, two `solve_coerce` arms, one `classify_mutability`
extension, plus the pre-existing `codegen_byref_scalar_deref` bug fix).
Option (c) is skipped outright — its only rationale was the scattered-
auto-deref fear, which the single-funnel finding dissolves. The full
semantics (rvalue copy-out, member see-through, write-through, binding
decay, match-scrutinee dissolution) and the implementation anchors live in
`docs/plans/references/stage1/semantics.md`; that file is the single source
of truth for the rules.

### 10.6 No reference types in parameter position — conventions are the only spelling

Decided 2026-06-09: `x: &T` and `x: &mutating T` are **disallowed,
permanently** — not deferred. Parameter passing keeps exactly one spelling
per mode: `x: T` (borrow, the default), `mutating x: T`, `consuming x: T`;
function types likewise (`(mutating T) -> R`, shipped #106). This reverses
the Stage-0.5 draft below (§11), which had the type-form as canonical and
`mutating on` as sugar. Rationale:

- `x: &T` would be *semantically identical* to `x: T` (borrow is already
  the default) — a sigil carrying zero meaning, plus a duplicate-detection
  collision (`f(x: G)` vs `f(x: &G)`).
- `x: &mutating T` would be a second spelling of `mutating` — two ways to
  say one thing, both legal forever.
- Nothing downstream needs the type form: the Stage-1 root rule keys on
  parameter index/convention, not spelling; function types already have
  convention syntax; storable-ref params only matter if Stage 2 is ever
  re-litigated (default: don't build, §11) — decide then, with the option
  of deprecating one form, rather than carrying two from day one.

Consequence: `&T` / `&mutating T` is **return-position syntax** (Stage 1).
Stage 0.5 shrinks to front-end plumbing + reject-everywhere diagnostics +
the §10.2 capture inits. Where §§10.1-10.5 say "`&mutating` param", read
"`mutating` param" — same convention, decided spelling.

---

## 11. Revised staging (post-§10 decisions)

The original Stage 1/2/3 ladder, restructured around what the §10 decisions
changed. This supersedes the feasibility doc's §1 staging.

### Stage 0.5 — syntax reservation + the pointer-capture half (~2-3 wk)

Ships with **zero new analysis** — no escape checker, no verifier changes, no
`ret_borrow`. Revised per §10.6: the original parameter-syntax half
(`&T`-canonical, `mutating on` as sugar, §5.3 normalization) is **dropped** —
conventions stay the only parameter spelling, so there is nothing to
normalize and no second spelling to keep in sync:

- Front-end plumbing: SyntaxKind/parser/AST/HIR variants (plumbing Stages
  0-4) and nothing past them — `&T` parses so it can be *rejected well*
  (real diagnostics + LSP recovery) and so Stage-1 returns slot in.
- The negative rules for ref types in **every** position, parameters
  included (one type-position walk + diagnostics) — cheap now, load-bearing
  forever; Stage 1 carves out the return position only.
- `Pointer(to:)` / `Pointer(mutating:)` — borrow param + an address-capture
  intrinsic (`withUnsafePointer` without the closure).

### Stage 1 — returnable references (~14-20 wk after 0.5)

The escape-checker milestone, now richer than the original MVP:

- **Both** `-> &T` and `-> &mutating T` returns (ban lifted, §10.4), gated by
  the uniform root rule: root ∈ {Param (mutable for mut), Static,
  PointerDerived}.
- `root_provenance` enum (`Param(idx)/Static/Local/PointerDerived`) stamped
  O(1) through projections; the return-site escape check in `verify.rs`.
- The `ret_borrow` bit: `MonoFunction` field, the two copy-guard gates, the
  `set_terminator` carve-out, and codegen in **both** backends (§6).
- `Pointer.value` / `.mutatingValue`, public, `# Safety`-documented (§10.2,
  §10.3) → stdlib accessors (`Array.first/at`, Dict internals) become real.
- `&mutating T → &T` coercion arm (§10.1).
- Use semantics: **transparent place (§10.5)** — the `solve_member` peel,
  the `&T → T` copy-out coercion arm, and the ref-aware
  `classify_mutability` extension; refs work as receivers for every
  dispatch form (members, operators, subscripts, for-in, interpolation).
- Negative rules continued: no named bindings, no cross-merge refs, no
  function-value / `throws` / generic-arg leakage (§5), no ref-typed match
  scrutinees.

### Stage 1.5 — ergonomics follow-ons (~4-6 wk, schedule on demand)

- Call-expression-as-place: `arr.at(i) = v` writing through a mut-ref
  accessor, reconciled with the existing subscript-setter lowering to one
  path (§10.4).
- Named ref bindings (`let r = &expr;` with the §2-Option-C visible cue),
  still block-local.
- The `Pointer(to: local)` same-function dangle lint (§10.3).
- Shared-read projection sugar over `Pointer.with` / Design-B closures for
  the `Optional[&T]`-shaped lookup APIs Stage 1 can't express (§5.3).

### Stage 2 — storable refs (unchanged cost, reduced pressure; default: don't)

The §10 decisions take collection accessors — the biggest original pull
toward Stage 2 — off its motivation list. Remaining pressure: `Optional[&T]`
lookups, ref-bearing fields (Span/cursor types), bindings crossing scopes.
The prior-art adjudication stands: treat a Hylo-shaped stop at Stage 1.5 as a
legitimate end state; re-litigate only if users hit the ceiling.

### Stage 3 — `Static` bound (unchanged; only meaningful after Stage 2)

---

*Provenance: third audit, 2026-06-09 — four parallel source-grounded probes
(MIR/verifier anchors; type-infer constraint/member/convention machinery;
stdlib collection internals; LLVM backend ABI), cross-checked against the four
companion docs and the working tree at 74f4b70c.*
