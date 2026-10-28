# Second-Class References (`&T` / `&mutating T`)

**Status**: Research / feasibility (not scheduled)
**Scope**: language feature spanning lexer → parser → AST → HIR → type-infer → MIR-lower → MIR/OSSA verifier → codegen
**Date**: 2026-06-03

This document captures a full feasibility analysis of adding *second-class
references* to Kestrel. It is the output of two source-grounded audits of the
compiler (MIR-3 era). Every `file:line` anchor below was verified against the
tree at time of writing — treat them as starting points, not gospel, since the
MIR/expand code is actively churning.

### Related documents

- [`references-prior-art.md`](references-prior-art.md) — how Hylo/Val, Swift
  (`~Copyable`/`~Escapable` + lifetime-dependency), and Mojo (origins) solve this;
  what transfers to Kestrel, and whether Hylo's "references are accesses, not
  values" model lets Stage 2 avoid putting a lifetime on every type. (External
  claims fact-checked.)
- [`references-syntax.md`](references-syntax.md) — surface-syntax & semantics
  design: `&T`/`&mutating T` spelling, explicit-borrow vs inferred-from-context,
  reference return types, reconciliation with the existing
  `borrowing`/`mutating`/`consuming` keywords. Options + a marked recommendation
  per decision (maintainer decides).
- [`references-plumbing.md`](references-plumbing.md) — the front-end plumbing
  checklist: every exhaustive-match site that needs a new arm for
  `AstType::Ref`/`HirTy::Ref`/`MirTy::Ref`, as file:line tick-boxes, plus the
  incremental-compilation / rowan `u16` note and a suggested green-build order.
- [`references-tests.md`](references-tests.md) — the Stage-1 test matrix: the
  `.ks` programs that pin each behavior and deliberately provoke each silent-UAF
  path, plus how a UAF actually surfaces under the test harness.
- [`references-gaps.md`](references-gaps.md) — third audit (2026-06-09):
  anchor re-verification, contradictions between these docs, and holes none of
  them cover — the provenance gap that blocks the flagship collection-accessor
  use case, the undefined deref semantics, the function-value `ret_borrow` ABI
  hole, generic-argument storage leaks, the missing LLVM-backend plumbing
  stages, and the missing negative-rule enforcement inventory. **Read alongside
  this doc** — it revises the Stage-1 estimate to ~16-22 wk and proposes a
  cheaper "Stage 0.5" cut.

---

## 1. The proposal

Add two reference types:

- `&T` — a shared (immutable) borrow.
- `&mutating T` — a mutable borrow.

Design constraints set by the proposal:

- References **reuse the existing guarantee/convention system** — the
  `@owned` / `@guaranteed` / `@borrow` machinery in MIR-3, not a new mechanism.
- `&mutating` references are **allowed to alias** (unlike Rust `&mut`). No
  exclusivity / two-phase-borrow / non-aliasing inference.
- "Second-class" means references are **scoped**, not freely storable in
  arbitrary long-lived locations.

### Staging (as proposed)

**Stage 1** — references within a function; return references; lifetimes
inferred; the referenced value must escape the function (outlive the call) to be
returnable.

**Stage 2** — every type carries a lifetime (while a value is alive its children
are too); references can be captured by closures, placed inside tuples, and
inside structs/enums.

**Stage 3** — references on protocols; a default `Static` protocol bound on every
type param; `T: not Static` allows non-static (referential) lifetimes.

---

## 2. Bottom line

This is an **8–12 month, redesign-scale feature for one expert**, and the
difficulty is **not** where the staging implies.

The MIR substrate is far more reusable than the proposal assumes — the entire
borrow instruction set, `ParamConvention::Borrow/MutBorrow` threaded to
`PassMode::ByRef`, `ValueDef.ownership = Guaranteed` + `borrow_source`, and
block-param forwarding of `@guaranteed` values all exist today.

**The single biggest insight: what's missing is analysis the compiler
deliberately does not have.** The OSSA verifier (`verify.rs`) is a single forward
BFS that checks each block in isolation with no fixpoint; it never proves that a
`@guaranteed` value's source is still live in successor blocks. That is exactly
why the return path **unconditionally copies every `@guaranteed` value to
`@owned` before `Return`** (`mod.rs:477`, `expr.rs:257`) — the copy exists
*because* there is no escape analysis to make returning a borrow safe. So the
real cost of references is a net-new **region/outlives + escape checker**, not IR
plumbing.

The difficulty ordering is **non-monotonic** in the staging:

| Stage | Difficulty | Rough effort (1 expert) | Biggest risk |
|---|---|---|---|
| **1** — in-function refs, return refs, inferred lifetimes | Very-high | ~10–14 wk, ~20–35 files | New escape/outlives checker is the only thing preventing **silent dangling returns** (compiles clean, crashes at runtime); sharing `MirTy::Pointer` makes refs indistinguishable from owning pointers to drop/clone |
| **2** — lifetime on every type; refs in closures/tuples/structs/enums | Very-high (cost center) | ~16–26 wk, ~30–40 files | `MonoTypeKey` lifetime-provenance collapse (cross-instantiation double-free); aliased `&mutating` into `RcBox`/escaping closures = UAF the verifier no longer catches; **collision with the unfinished Rc-closure migration** |
| **3** — refs on protocols, default `Static`, `T: not Static` | High (easiest of three) | ~4–7 wk, ~12–16 files (after 1–2) | False-positive structural "is-Static" silently re-opens dangling-storage holes stdlib-wide; witness-lowering lifetime-erasure mismatch → codegen corruption, not a clean error |

**Total: ~8–12 months end-to-end for one strong expert, with Stage 2 the
make-or-break milestone.**

---

## 3. How references map onto today's model

A `&T` is structurally **a nameable, returnable `@guaranteed` borrow**;
`&mutating T` is a `@guaranteed` value from `BeginMutBorrow`. The
instruction-level machinery is complete:

- **Borrow primitives exist:** `BeginBorrow` / `EndBorrow` / `BeginMutBorrow` /
  `EndMutBorrow` / `BeginBorrowAddr` / `BeginMutBorrowAddr`
  (`lib/kestrel-mir/src/inst.rs`). `&T` → `BeginBorrow(Addr)`, `&mutating T` →
  `BeginMutBorrow(Addr)`. **No new MIR instructions needed.**
- **Reference *parameters* are nearly free:** `ParamConvention::Borrow/MutBorrow`
  already thread end-to-end (parser → AST → HIR → MIR → ABI) and already force
  `PassMode::ByRef`. The cheapest win in the whole proposal.
- **Provenance scaffolding exists:** `ValueDef.borrow_source`
  (`lib/kestrel-mir/src/value.rs:14-16`) links a `@guaranteed` value to its
  `@owned` source (single-hop). The verifier *syntactically* accepts
  `@guaranteed` values forwarded as block args (`verify.rs:1012-1033`, Check 4);
  `BlockParam` carries ownership.

**The key gap:** borrows today are scoped to a single call/expression or, at
most, a scope frame. `set_terminator()` force-ends *all* scope-tracked borrows
before any terminator (`mir-lower/src/body/mod.rs:~1820`); `restore_scope()`
strips them across branches; the verifier rejects any open borrow at block exit.
A named `&T` bound to a `let`, returned, or stored must outlive its creating
block — and **there is no lifetime, region, outlives relation, or escape graph
anywhere in type-infer or MIR.** The solver has only
`Equal`/`Coerce`/`Conforms`/`Associated` constraints
(`lib/kestrel-type-infer/src/constraint.rs`). References are an analysis problem
layered on a reusable substrate.

---

## 4. Stage 1 — in-function refs, return refs, inferred lifetimes

**Difficulty: very-high.** Effort: ~10–14 weeks, ~20–35 files (~12 are mechanical
type-pipeline fan-out; the region/escape pass ~5 files is the irreducible core).

### Reusable today (large)

- The full borrow instruction set.
- The `ParamConvention` ABI path → ≈zero work for reference *parameters*.
- `borrow_source`.
- Check 4's acceptance of forwarded `@guaranteed` block args.
- `lower_expr_for_borrow` (already produces `@guaranteed` without consuming the
  source).
- `LiveTracker` / `rebind_scope_values` (exercised today only for `@owned`, but
  the mechanism generalizes).
- `alloc_guaranteed` (`mir-lower/src/body/mod.rs:663`) — call-result `@guaranteed`
  registration already exists.

### Net-new (the bulk)

1. **A distinct `MirTy::Ref{pointee, mutable}` — *not* reusing `MirTy::Pointer`.**
   They are machine-isomorphic (both pointer-width scalars, no drop), but if
   `&T` *is* `Pointer`, drop/clone elaboration cannot distinguish a safe borrow
   from an owning/unsafe pointer. The #1 latent-bug source; pay it down in Stage
   1 or face a painful Stage-2 retrofit. (In the *minimal* v1, you can defer the
   storable `MirTy::Ref` type and carry references as a convention bit — see §6.)
2. **A region/outlives inference pass** after type inference: region variables,
   `borrow ⊆ source-scope` constraints from each `BeginBorrow`, return-region
   constraints satisfiable only from parameters. Zero precedent — a new
   constraint class that interacts with *subtyping* (a partial order), unlike
   membership-style `Conforms`. **You cannot encode `'a: 'b` in the unification
   union-find — it would merge the regions, which is wrong.**
3. **An escape / "must-escape" checker** that walks the `borrow_source` chain and
   verifies a returned `&T` bottoms out in a parameter/static, not a local.
   Safety-critical — a false negative is a silent use-after-free.
4. **Conditional `@guaranteed` return:** replace the two unconditional copy sites
   (`mod.rs:477-489`, `expr.rs:257-265`). This violates the universal invariant
   "every function returns `@owned`" assumed by the verifier, all call sites, and
   `expand_destroy_copy` — a coordinated three-subsystem change.
5. **Cross-block borrow persistence + verifier liveness:** relax
   `set_terminator`/`restore_scope` for reference-locals *only*, thread them
   through block params, and extend the per-block verifier to prove the source is
   live across successors. **The load-bearing invariant relaxation** — doing it
   for reference-locals without breaking the thousands of ordinary ephemeral
   borrows is the core engineering difficulty.
6. **Codegen:** lower a `@guaranteed` `Return` as by-ref ABI in both backends.

### Memory-model hazard — drop timing vs returned borrows

`destroy_scope_except` ends borrows *before* `@owned` drops (correct intra-scope),
but the scope stack frees a local at lexical scope exit with no awareness that a
reference escaped upward. `&x` to a local that's then returned passes Check 4 yet
dangles once `destroy_scopes_to_depth` drops `x`. **The escape checker is the
only thing standing between this feature and silent dangling pointers.**

---

## 5. Stage 1, deep dive: the hardest parts (ranked)

> The headline: **the part that sounds like a PhD (region inference) is free in
> v1; the part that sounds routine (conditional return) is where you'll actually
> ship a UAF if you're sloppy.**

### #1 — Conditional `@guaranteed` return (breaking "every fn returns `@owned`")

- **Why hard:** the seam where the borrow's provenance, the verifier's escape
  proof, and codegen's pointer-vs-value choice all have to line up — *and the
  verifier currently can't help you*. `assert_live`/`try_consume` early-return
  for any non-`Owned` ownership (`verify.rs:362`, `:322`); the `None` branch
  returns `true` for cross-block values (`:355`). A `@guaranteed` value in
  `Return` position is `assert_live`'d as a **no-op** today (`verify.rs:989`).
- **Irreducible core:** (a) a return-convention signal on the function
  (`ret_borrow: bool` on `MonoFunction`/`FunctionDef`) — it can't be a type if
  you defer `MirTy::Ref`, and it can't be inferred from value-ownership because
  `return_mode(repr, is_main)` runs from the *signature*, not the body
  (`abi.rs:33`); (b) the codegen branch — `resolve_scalar` (`func.rs:45-62`)
  **loads through the ByRef pointer** for `Guaranteed`+`Scalar`, so a scalar
  `&T` return must use `get_value` (the raw pointer), or it silently returns the
  *pointee by value*.
- **Failure mode:** silent UAF + silent miscompile. **No ICE.**
- **Difficulty: novelty-low, surgery-high.** Careful surgery on the most
  dangerous invariant in the system.

### #2 — Escape / must-escape checker (+ referent drop-timing)

- **Why hard:** the one *genuinely novel* analysis. No escape concept exists;
  the verifier can't even *see* a live borrow on a local because `@guaranteed`
  projections (StructExtract/TupleExtract/EnumPayload) aren't inserted into
  `self.borrows`, so `try_consume`'s blocking loop (`verify.rs:328-334`,
  filtered on `info.source == v`) never fires for them.
- **Irreducible core:** a per-`ValueDef` `root_provenance` stamp, copied **O(1)
  through projections at creation** (not walked at verify time — the existing
  `borrow_source` is single-hop and `None` for params), plus one branch at the
  Return site that suppresses force-copy *and* asserts the root is
  `Param`/`Static`.
- **Failure mode:** silent UAF.
- **Difficulty: novelty-high (the real research seed), volume-low after the
  param-root restriction** — which makes referent-drop a non-event (caller owns
  it; never in callee `self.owned`, never dropped in `destroy_scopes_to_depth`).

### #3 — Region / outlives inference from zero

- **Why hard in general:** a `RegionVar` type + asymmetric outlives constraint
  graph + post-unification region solver is genuine net-new infra; the solver has
  only `Equal/Coerce/Conforms/Associated` and **nothing inequality-shaped**
  (`constraint.rs`).
- **Why it's #3:** for the Stage-1 slice **you don't build it at all.** The only
  outlives fact is "a parameter outlives the call," trivially true by the ByRef
  convention and checkable by the same `borrow_source` walk as #1/#2.
- **Difficulty: novelty-very-high but entirely deferrable.** Looks hardest, is
  cheapest in v1 — because you skip it. *Trap:* building it prematurely risks
  grafting an outlives encoding onto `unify.rs`, the predictable wrong turn.

### #4 — Lifting intra-block-borrow invariant + cross-block `@guaranteed` liveness

- **Why hard:** real cross-block borrows need `add_guaranteed_block_param`
  (confirmed it exists **only as a panic-string aspiration** — no such function)
  plus co-threading `(source, borrow)` pairs through `LiveTracker`, plus
  consumed-on-one-arm rejection at merges. The verifier is per-block-isolated,
  no fixpoint.
- **Why it's least:** v1 doesn't lift it. Keep `set_terminator` force-ending all
  borrows; the one useful cross-block case — borrow of a *parameter* — is sound
  via **re-borrowing the param fresh per block** (its ValueId is in scope
  everywhere; liveness is a whole-function invariant), needing zero block-param
  infra.
- **Difficulty: novelty-medium, volume-medium — fully deferred in v1.**

### Voluminous-but-mechanical (not "hard", just wide)

The `&T`/`&mutating T` front-end plumbing — lexer/parser → `AstType::Ref` →
`HirTy::Ref` → name-res → survive to MIR. Most *lines touched*, the irreducible
feature surface, but pattern-matches existing `ParamConvention` plumbing. Don't
confuse its volume with the novelty of #2/#3.

### Novel research vs. careful surgery

- **Genuinely novel:** #2 (escape checker) and #3 (region inference). #3 is the
  deeper research but you **defer it entirely**, so #2 is the only novel work
  that ships in v1.
- **Careful surgery on existing invariants:** #1 (return convention, ships) and
  #4 (cross-block liveness, deferred).
- **Riskier of the two that ship: #1, not #2.** The surgery touches a
  *globally-assumed* invariant the verifier, all call sites, codegen, and
  `expand`'s drop elaboration silently rely on — and the verifier gives **no
  backstop** (it no-ops on `@guaranteed`). A bug in #1 is felt project-wide; a
  bug in #2 is felt only at reference returns.

---

## 6. Minimal-viable Stage 1

**Ship:** reference *parameters* (already free) + in-function intra-block borrows
(already work) + **return-borrow-of-a-parameter**, shared `&T` only.

**The 6-site change, all gated on one `ret_borrow: bool`:**

1. `&T`/`&mutating T` surface syntax → `HirTy::Ref` → carried to MIR as the
   convention bit (no `MirTy::Ref` yet).
2. Gate the two copy guards (`mod.rs:476`, `expr.rs:258`) on `!ret_borrow`.
3. Verifier: when `ret_borrow`, require the `Return` value `@guaranteed` **and**
   `borrow_source` traces (to fixpoint) to a `Borrow`/`MutBorrow` **parameter** —
   that *is* the escape proof.
4. Codegen: branch `compile_return` on `ret_borrow` to use `get_value` (pointer),
   not `resolve_scalar` (`func.rs:45`); force `ReturnMode::Direct(ptr)` in
   `return_mode` (`abi.rs:33`).
5. Call result: register `@guaranteed` (reuse `alloc_guaranteed`, `mod.rs:663`)
   and skip `track_owned`, so no `DestroyValue` is emitted for caller-owned
   storage.
6. Carve the escaping return value out of `set_terminator`'s force-EndBorrow
   (carve out *exactly one* value).

**Defer:** `MirTy::Ref`/`HirTy::Ref` as a storable type (no `&T` in struct
fields/arrays/`var`); all region/outlives solver work; borrows-of-locals returns;
cross-block/cross-merge forwarding (`add_guaranteed_block_param`); `&mutating`
returns; reference-returning closures/protocol methods; `&mutating` into
RcBox/heap-shared state.

**One forward-compat insistence:** stamp `root_provenance` as an **enum carrying
the root kind** (`Param(idx)`/`Static`/`Local`) from day one, even at the same
O(1) cost — a bare `bool "escapes"` throws away which-param and forces a re-plumb
when Stage 2 adds local-rooted returns and merge-joins.

**Effect:** drops #3 and #4 out of v1 entirely, collapses #2 to a stamp + one
assertion, leaving **#1 as the dominant remaining risk**.

---

## 7. The traps (read before touching code)

- **Removing the `@guaranteed→@owned` copy guard without gating it on
  `ret_borrow`.** The two guards (`mod.rs:476-483`, `expr.rs:258-266`) are the
  only thing between you and disaster — `lower_expr_for_borrow` can produce a
  `@guaranteed` tail value for *ordinary* functions. Drop the guard globally and
  a normal function returns a pointer-to-stack the caller treats as `@owned` and
  drops → silent UAF / double-free, no diagnostic. **Guard removal must be
  conditional on the per-function bit.**
- **`&mutating`-may-alias × Check 5 — do not allow a `&mutating` *return* in
  v1.** *(Superseded 2026-06-09: ban lifted in favor of a mutable-root rule —
  see [`references-gaps.md`](references-gaps.md) §10.4. The freeze this trap
  protects is already given up by the may-alias decision.)* A returned
  mut-ref keeps the source frozen-for-mutation across the call
  boundary, which `assert_readable` / Check 5 (`verify.rs:373`) cannot enforce
  past `Return` (per-block, no fixpoint). The aliasing decision is sound *within
  a block* (the only intra-block hazard is read-while-mut-borrowed, which Check 5
  already catches) but does **not** extend across the return. Shared `&T` returns
  only.
- **Don't reuse `MirTy::Pointer` for references.** `Pointer` carries no
  ownership/`borrow_source` provenance — the entire soundness argument rests on
  references being `@guaranteed` values *with* `borrow_source`
  (`value.rs:14-16`). Squatting on `Pointer` erases the provenance *and* turns
  the Stage-2 `MirTy::Ref` introduction into a migration instead of an addition.
- **`set_terminator` force-ends *all* scope-tracked borrows** before every
  terminator (`mod.rs:~1820`). The single escaping return value must be carved
  out of that retain-filter — *exactly one* value, nothing more, or you
  reintroduce cross-block borrows you haven't designed for.
- **"Just allow `@guaranteed` block args" as a cross-block shortcut** without
  co-threading is a silent UAF: `assert_live` only consults the per-block
  `self.owned`, and the predecessor's `ValueState` lives in a discarded
  `BlockVerifier`, so a source consumed on one arm is never re-checked
  (`verify.rs:352-369`).
- **Single-source `borrow_source` as an architectural assumption** (vs. a v1
  reject). `return cond ? &a : &b` (two parameter borrows joined at a branch) is
  a natural early ask; keep the model able to *represent* a borrow whose source
  is one-of-several even if v1 *rejects* it.
- **`&mutating` into heap-shared / RcBox state.** A `&mutating` derived from
  dereferencing a shared Rc payload can alias live readers and interacts with the
  COW `getValue`/`setValue` bugs already open in the memory model. "Param outlives
  the call" is about the *referand's storage*, not exclusivity over shared
  interior state — reject heap-shared `&mutating` in v1.

---

## 8. Stage 2 — lifetime on every type; refs in closures/tuples/structs/enums

**Difficulty: very-high. This is the cost center and the make-or-break
milestone.** ~16–26 weeks, ~30–40 files. Redesign-scale.

Why it's pervasive: a reference in an aggregate must carry a provable outlives
relation, so a lifetime/region slot threads through
`TyKind`/`MirTy`/`ResolvedTy`/`FieldDef` and *universally* through any
`Named`/`Tuple`/`Enum` that transitively contains a reference. That changes the
`TyArena` intern key for ref-containing types and forces `MonoTypeKey` (today
`(Entity, Vec<TyId>)`, no lifetime dimension) to encode lifetime provenance — **or
two instantiations with different referent lifetimes collapse to one key and
share a drop shim.** That is exactly the failure class already seen in
`expand_not_copyable_nominal_collapse` / `apply_partial_thunk_mono_collapse`:
silent cross-instantiation double-free/leak.

### Three head-on collisions with existing memory-model work

- **Closures-becoming-Rc-boxed.** The planned `FuncThick` upgrade (copy = retain)
  lets an Rc-boxed closure capturing `&T` spread the reference to extra owners and
  outlive the referent. **Stage 2 ref-capture and the Rc upgrade must be
  co-designed**, or the Rc upgrade must exclude reference-capturing closures.
  Worse, closure env drop is *unstable right now* (gated prototype double-frees
  the clone-alias case; closure-temp-not-dropped leak open). *Bright spot:* the
  by-ref capture path is mechanically ready — the `is_protocol_self` capture in
  `closure.rs` does `BeginBorrowAddr` on the env field with no load/own/drop,
  exactly the lowering a non-escaping `&T` capture needs. What's missing is the
  escape *classification*.
- **Aggregate drop/copy elaboration.** `emit_destroy_recursive` /
  `emit_clone_recursive` recurse into fields. A `&T` field must be drop-no-op +
  clone-pointer-copy. This "accidentally works" today only because `Pointer` has
  no shim — it becomes a **double-free** the moment `T` gains a clone/drop shim
  and the ref is typed as `Pointer`. The distinct `MirTy::Ref` marker (paid in
  Stage 1) is what lets `drop_shim.rs`/`expand.rs` explicitly skip. This path is
  freshly patched and still in flux: the tuple drop/copy symmetric copy/clone-
  alias case is **not fixed**, and `expand.rs` is in the uncommitted working set.
- **COW/RcBox.** Storing a short-lived `&T` in an `RcBox[T]` whose `clone()`
  (non-atomic bump) outlives the referent is a guaranteed dangling read, with **no
  guard today** (`RcBox.setValue` does `dropInPlace(old)+write(new)`). With
  aliasing-allowed `&mutating`, soundness rests *entirely* on the new lifetime
  checker proving the heap container can't outlive the referent — the case that's
  hardest to prove. Non-atomic refcount makes it worse on any future concurrency.

### Conditional-move drop flags

`VarInit`/Bool drop flags answer "does this slot own a value?"; a reference field
introduces an orthogonal "is the referent still alive?" question the flag
machinery does not model.

---

## 9. Stage 3 — refs on protocols; default `Static`; `T: not Static`

**Difficulty: high — genuinely the easiest of the three** (after Stages 1–2 land).
~4–7 weeks, ~12–16 files. The bound system is a near-1:1 clone of working
machinery.

### Reusable (strong)

- `implicit_conformance: true` already exists (`builtin.rs:647`);
  `inject_implicit_copyable_bounds` and `TypeParamCopyRequirement` are confirmed
  in `where_clauses.rs`. Add `Builtin::Static` with the same flag →
  `protocol_allows_negative_conformance` and the whole negative-conformance path
  turn on for free.
- `inject_implicit_static_bounds` / `TypeParamStaticRequirement` are direct clones
  of the Copyable templates.
- **Zero parser/lexer work:** `T: not Static` reuses `WhereConstraint::NegativeBound`,
  which already lowers end-to-end to MIR `WhereConstraint::NotImplements` (the
  `not Copyable` path). `not` is `Token::Not`; `Static` is an ident.
- `WorldResolver.conforms_to` gets a `Static` special-case mirroring
  Copyable/Cloneable — a **structural** check (does the type transitively contain
  a reference?) instead of declared-conformance lookup.
- Per-instantiation `(Entity, Vec<TyId>)` keying in `expand.rs` is the right shape
  for per-instantiation Static reasoning.

### Genuinely new

1. **Ref-returning protocol methods** — `func items() -> &Self.Item` requires
   `HirTy::AssocProjection` and `MemberResolution.return_type` to carry borrow
   annotations, threaded through `build_member_resolution` / `LowerCallableReturnType`
   / witness lowering. The ~3–4 week core.
2. **Lifetime erasure at mono** — the *correct, easy* direction (a ref is just a
   pointer at MIR; erase like Rust). `MonoFunction`/`MirTy` need no Static-specific
   extension. The one place erasure can go wrong is **witness lowering**
   (`witness.rs`): a protocol method returning `&Self.Item` whose concrete witness
   returns owned `T` is a MirTy shape mismatch surfacing as codegen corruption,
   not a clean error — the `witness_instantiation_collapse` class.
3. **`ConditionalStaticParams`** (analog to `ConditionalCopyableParams`) for
   `struct Box[T]: not Static where T: not Static`.

### Why the default `Static` bound matters beyond syntax

The default `T: Static` bound is the **soundness backstop** that contains
Stages 1–2: generic code assuming `T: Static` may freely store/return `T` with no
referent to dangle. The sole risk is a buggy structural "contains-a-reference"
predicate — a false-positive `Static` answer silently re-opens every
dangling-storage hole the bound exists to close, stdlib-wide (same blast radius
the implicit-Copyable injection has historically had).

---

## 10. The `&mutating`-may-alias decision

**What it buys (significant):** it sidesteps the single most expensive part of a
Rust-style borrow checker — exclusivity / two-phase-borrow / non-aliasing
inference. Instead of an aliasing model, you **relax the verifier's existing
read-during-mut-borrow check** (`assert_readable`, Check 5, `verify.rs:373`). The
largest engineering simplification available, and a major reason the feature is
tractable. At Stage 1 (second-class, single-function, non-storable) the soundness
cost is contained.

**What it costs (sharp, escalating):**

- **Soundness surface.** Check 5 is the one rule preventing read-during-mutation.
  Relax it and two live `&mutating` (or reader + mutator) into the same `RcBox`
  payload can interleave a `getValue().clone()+setValue()` (i.e.
  `dropInPlace(old)+write(new)`) with another reader observing a **freed
  buffer** — a UAF the OSSA verifier **will no longer flag**. Peaks at Stage 2
  where aliased `&mutating` becomes reachable from ordinary code.
- **No aliasing-based optimization.** You forfeit LLVM `noalias` wins.
- **Forced ad-hoc rules at Stage 2.** Need an extra restriction (no `&mutating`
  capture into escaping closures / no `&mutating` into RcBox-backed storage) or
  accept documented unsoundness. With exclusivity gone, soundness rests *entirely*
  on the lifetime checker — no linearity backstop underneath.

**Adjudicated view:** take the relaxation — it's what makes the timeline
plausible — but treat it as a **Stage-1-only safe simplification** and write the
Stage 2 storage restrictions into the design *before* implementing
reference-in-aggregate. Do not let Stage 2 ship with aliased `&mutating`
reachable through `RcBox` until the lifetime checker can prove
container-outlives-referent for heap-shared containers.

---

## 11. Recommended sequencing & prerequisites

**Two non-negotiable commitments, made in Stage 1:**

1. **Distinct `MirTy::Ref` from day one** (or at minimum a `root_provenance` enum
   + return-convention bit that does not paint Stage 2 into a corner) — never
   reuse `MirTy::Pointer` for the storable form.
2. **Build the escape/outlives checker as the *first* Stage 1 deliverable.**
   Without it everything compiles clean and dangles at runtime. Land the safety
   gate before the ergonomics.

**Clear these existing WIP/bugs first** (references reuse these exact paths):

- `mir3_noncopyable_move_lowering` + `mir3_noncopyable_var_read_copy` — open OSSA
  "copy of non-Copyable" cases; refs reuse `emit_copy_value`/borrow + consume-half
  address drop-flags.
- `diamond_conditional_move_let_drop_timing` — a `let r = &x` conditionally live
  across an if-merge inherits this broken drop-timing.
- `deferred_end_borrow_design` ("drain points need work") — the exact mechanism
  reference-scope `EndBorrow` placement must extend.
- `mir3_ossa_inline_if_operand_ice` — `&(if c {a} else {b})` lands in the same
  hole.
- **Before Stage 2:** tuple drop/copy elaboration must stabilize (symmetric
  copy/clone-alias case open; `expand.rs` uncommitted), and the **Rc-closure model
  must be settled and co-designed** with reference capture; the gated
  closure-env-drop double-free must be resolved.

**Can Stage 1 ship usefully alone? Yes — and it should.** Reference *parameters*
(nearly free) plus in-function named borrows and parameter-derived returned refs
deliver most of the day-to-day value (pass-by-reference without COW clones, the
`Str.toBytes()`-class amplifiers, accessor methods returning borrows) without the
pervasive cost. Forced order is **Stage 2 → Stage 3** (the `Static` bound is
meaningless without a lifetime representation to make a type non-Static). Ship
Stage 1, then re-evaluate whether the Stage 2 cost (and the COW/Rc-closure
collision) is worth Stage 3's expressiveness before committing to the back half.

---

## Appendix A — Verified code anchors

These were confirmed against the tree during the audit. The MIR/expand code
churns; re-verify before relying on a line number.

| Anchor | What it is | Relevance |
|---|---|---|
| `lib/kestrel-mir/src/inst.rs` | `BeginBorrow`/`EndBorrow`/`BeginMutBorrow`/`EndMutBorrow`/`*Addr` | Borrow primitives all exist; no new MIR insts needed |
| `lib/kestrel-mir/src/value.rs:14-16` | `ValueDef.ownership`, `ValueDef.borrow_source` | Single-hop provenance link; basis of the escape walk |
| `lib/kestrel-mir/src/verify.rs:322` | `try_consume` early-returns for non-`Owned` | Verifier no-ops on `@guaranteed` |
| `lib/kestrel-mir/src/verify.rs:328-334` | `try_consume` blocking loop filtered on `info.source == v` | Projections not in `self.borrows` → loop never fires for them |
| `lib/kestrel-mir/src/verify.rs:352-369` | `assert_live` consults per-block `self.owned` only | Cross-block consume not re-checked (UAF shortcut trap) |
| `lib/kestrel-mir/src/verify.rs:355` | `None` branch returns `true` for cross-block values | No cross-block liveness proof |
| `lib/kestrel-mir/src/verify.rs:362` | `assert_live` early-return for non-`Owned` | — |
| `lib/kestrel-mir/src/verify.rs:373` | `assert_readable` (Check 5) | Read-during-mut-borrow guard; relaxed by `&mutating`-aliasing |
| `lib/kestrel-mir/src/verify.rs:989` | `Return` value `assert_live`'d | No-op for `@guaranteed` returns today |
| `lib/kestrel-mir/src/verify.rs:1012-1033` | Check 4: forwarded `@guaranteed` block args | Syntactically accepted, not liveness-proven |
| `lib/kestrel-mir-lower/src/body/mod.rs:477-489` | Unconditional `@guaranteed→@owned` copy at return | The invariant to conditionally break |
| `lib/kestrel-mir-lower/src/body/expr.rs:257-265` | Second copy-at-return site | Same |
| `lib/kestrel-mir-lower/src/body/mod.rs:663` | `alloc_guaranteed` | Reusable call-result `@guaranteed` registration |
| `lib/kestrel-mir-lower/src/body/mod.rs:~1820` | `set_terminator` force-ends all scope borrows | Must carve out the single escaping return value |
| codegen `func.rs:45-62` | `resolve_scalar` loads through ByRef ptr for `Guaranteed`+`Scalar` | Scalar `&T` return must use `get_value`, not `resolve_scalar` |
| codegen `abi.rs:33` | `return_mode(repr, is_main)` — no convention input | Needs the `ret_borrow` bit to pick `Direct(ptr)` |
| `lib/kestrel-type-infer/src/constraint.rs` | `Constraint` enum: `Equal/Coerce/Conforms/Associated` only | No inequality/outlives constraint exists |
| `lib/kestrel-hir/src/builtin.rs:647` | `implicit_conformance: true` precedent | Stage 3 `Static` bound clones this |
| `lib/kestrel-type-infer/src/where_clauses.rs` | `inject_implicit_copyable_bounds`, `TypeParamCopyRequirement` | Stage 3 templates |
| `closure.rs` (`is_protocol_self` capture) | `BeginBorrowAddr` on env field, no load/own/drop | Exact lowering a non-escaping `&T` capture needs (Stage 2) |
| `add_guaranteed_block_param` | exists **only** as a panic-string aspiration | Real cross-block borrow forwarding is unbuilt |

## Appendix B — Open questions / thin evidence

- **Effort numbers are bracketed estimates, not measured.** The region/outlives
  pass has no precedent in this codebase; its 4–6 week figure is the least
  certain in the analysis.
- **`@guaranteed`-return ABI** was assumed straightforward (pointer-width by-ref)
  but not verified against the LLVM backend specifically (Cranelift path checked
  via `func.rs`/`abi.rs`).
- **Stage 2 `MonoTypeKey` blast radius** depends on how many stdlib aggregates
  would actually carry references — a design decision not yet made.
- **copy_propagation interaction:** the single escaping ref has no `EndBorrow`
  before `Return`; verify this doesn't regress the source-unfreeze-on-EndBorrow
  logic, but no pass rewrite is expected in v1.
- **Field-of-parameter borrows** (`&param.field`): confirm `root_provenance`
  copies correctly through `StructExtract`/`BeginBorrowAddr`; a dedicated
  base-address+provenance instruction may be needed if projection-copy proves
  insufficient.

---

*Provenance: two source-grounded feasibility audits (multi-agent), 2026-06-03.
Stage difficulty, irreducible-core analysis, and the MVP cut were cross-checked by
independent effort and soundness lenses and reconciled.*
