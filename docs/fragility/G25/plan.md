# G25 — associated-type equality clauses are declared but never evaluated

> **Status:** filed 2026-08-25, designed, not implemented.

> ⚠ **Re-verified @ `60d3406a`, 2026-09-22 (step 0 + commit 1). Three of this
> plan's claims are refuted; §"What already exists" and Design §1/§2 rest on
> them. Read this before the rest.**
>
> 1. **"The two emitters skip equality clauses" — REFUTED.** Both
>    `emit_protocol_assoc_type_where_clauses` (`lib.rs`) and
>    `emit_type_alias_where_clauses` (`solver.rs`) have a live `TypeEquality`
>    arm that emits `Associated(alias_tv, "Item", fresh)` + `Equal(fresh, rhs)`.
>    What they skip is `DirectEquality` and projection-subject `Bound`s — those
>    are D7's "inert features". Only `entailment.rs` returns `false` for
>    equality. So Design §2 ("emit the unification") already exists for this
>    clause; it is not the missing piece.
> 2. **"A bare `TargetIterator` is `Projection { Projection { SelfType,
>    TargetIterator }, Item }`" — REFUTED.** An equality clause carries no
>    `WhereSubject` at all. `WhereClausesOf` on the alias produces
>    `TypeEquality { param: Iterable.TargetIterator (the TypeAlias entity),
>    assoc_name: "Item", rhs: Iterable.Item }` (measured by the commit-1 probe).
>    Even on the `Bound` path a bare `TargetIterator.Item` would not root at
>    `SelfType`: `resolve_type_path_chain` sets `self_rooted` only for a literal
>    `Self` segment, and `resolve_projection_subject` returns `None` for a
>    non-`TypeParameter` root. The "blocker dissolved" claim is false; building
>    that representation is still work.
> 3. **What R4 actually stands in for (new, measured).** The `TypeEquality` arm
>    in `emit_protocol_assoc_type_where_clauses` registers the answer in the
>    memo via `push_assoc_sub(Some(alias_tv), find_assoc_type_in_bounds(child,
>    "Item"), rhs_tv)`. For the bridge clause that lookup returns **`None`**:
>    `find_assoc_type_in_bounds` asks off `TyKind::Param { alias }`, which does
>    not see an alias's declared `: Iterator` bound. Asked off
>    `TyKind::TypeAlias { alias }` it returns `Iterator.Item`. So the clause is
>    evaluated but its memo registration is dead, and R4 fills that hole by
>    name. Every one of the 99,391 R4 hits in the corpus relies on a clause
>    whose production registration is dead this way (`!prod` in the probe).
> 4. **The 27/compilation count — CONFIRMED** at `60d3406a` (after C3b and
>    G26), and **every hit outside the witness is `JUSTIFIED`** with
>    `base_rel=proj-of-entry` (asked off `S.TargetIterator`, answered from an
>    entry filed off `S`, exactly the clause's shape). The witness's one hit is
>    `UNJUSTIFIED` and involves no equality clause at all: it is a `Bound`
>    (`A.Item: Show`) entry for `ProducerA.Item` handed to `ProducerB.Item`.
>    Numbers: `docs/fragility-audit.md` G25 entry.
>
> Consequence for the sequence: before folding variants, the cheaper candidate
> is to make the existing `TypeEquality` registration resolve its lhs (and to
> register in `emit_type_alias_where_clauses`, which does not push at all), then
> measure whether R4 still fires. Not predicted — measure it.
>
> **Step 1 landed (2026-09-23).** Both emitters now resolve the clause's lhs
> through `alias_bound_assoc_entity` (asks off `TyKind::TypeAlias`) and register
> `(base = alias_tv raw, assoc) → rhs_tv`. **Fixing only the registration broke
> the stdlib** (`expected T got Self.Item` in `Array.append`, `flatten`, `Set`,
> `Deque`, `Dictionary`, …): the RHS `Item` arrives as
> `AssocProjection { SelfType(Iterable), Item }`, and both emitters lowered that
> `Self` to the literal protocol `Self`, not the conformer. So the equation was
> always about the wrong receiver; R4 masked this because it answered from the
> correctly-based `Iterable.Item` entry. The fix binds the protocol's `Self` to
> the conformer (`subject_tv` in `lib.rs`, `container` in `solver.rs`) through a
> `(protocol → tv)` subs entry that `lower_hir_ty_with_subs`'s `SelfType` arm
> now honours. **Measured @ step 1:** R4 hits over 3681 testdata files: **1**
> (the witness, `UNJUSTIFIED`); floor **0**; all 17 `lang/` packages **0**.
> Suite 3836/5 unchanged, per-file `dump diagnostics` exit codes identical, the
> witness still prints `result=i`. R4 is now reached only by the miscompile.
>
> **Step 2 landed (2026-09-23): R4 is deleted.** `assoc_sub_by_name`, its arm in
> `solve_associated`, `Keying::BaseBlind` and the `audit-assoc-eq` probe are
> gone. The witness is rejected in the frontend (`E100 … B.Item !: Show`, no
> binary produced) and passes unedited. Suite **3837 passed, 4 failed**; that
> test is the only verdict change. G25 is closed in the audit. Steps 3 (fold
> the variants) and 4 (entailment) continue below as cleanup.
>
> **Step 3 (fold) — STOPPED at its drop gate, not landed (2026-09-23, base
> `165dd802`).** I prototyped `Equality { subject: WhereSubject, rhs }`. The
> alias root is expressed with the existing `WhereSubject::Param(alias)` (the
> same convention bare-assoc bounds already use), so no new variant was needed.
> The drop census (a temporary `g25-fold` ktrace in `resolve_where_clauses`,
> over 3681 testdata files plus 17 `lang/` packages) found **2 of 24
> equality clauses per compilation dropped**. There is 1 distinct source,
> **`Item.Output = Item`** on `extend Iterator where Item: Addable` /
> `Multipliable` (`lang/std/iter/iterator.ks:1032`, `:1049`).
> `resolve_type_path_chain(["Item","Output"])` returns `NotFound("Output")`:
> name-res follows only an alias's *declared* bounds, not the `Item: Addable`
> bound that sits in the same extension where clause. The string-keyed
> `TypeEquality` never needed the entity. Dropping the clause breaks `sum()`
> (`expected Item got Item.Output`). There are no other drops: per file, the
> extras were 19 KEEP + 3 DIRECT. Resolving that assoc needs where-clause-aware
> path resolution (the D7 "path resolution needs an order" concern, for
> clause-granted bounds), so it is a design question, not a focused fold.
>
> **Step 3 landed (2026-09-23), with two-pass resolution.** `WhereClause` now
> has `Bound` and `Equality { subject: WhereSubject, rhs }`. `T.Item = X` is
> `Projection { Param(T), Item-entity }` and `V = X` is `Param(V)`; an alias
> root reuses `Param(alias)`, so no new variant was needed.
> `resolve_where_clauses` resolves every `Bound` of the holder first (pass 1).
> It then resolves equalities in source order (pass 2): by the name-res
> segment walk, and if that fails, by `assoc_name` among the associated types
> of the protocols this holder's own bounds place on the same subject
> (`ProtocolAssociatedTypes`, which includes refinement parents; only the
> requirement declared on a protocol counts). **Ambiguity (2+ requirements)
> keeps the clause out and traces `KESTREL_DEBUG=where-eq` AMBIGUOUS**, pinned
> by `types/generics/assoc_equality_ambiguous_across_holder_bounds.ks`: the
> body is rejected, not at an ideal location. Zero matches traces UNRESOLVED.
>
> **Clause census** (3681 testdata files + 17 `lang/` packages):
>
> | | prototype | two-pass |
> |---|---|---|
> | kept via path walk | 62,596 | 62,596 |
> | kept via holder bounds | — | 7,362 |
> | kept direct (`V = X`) | 18,408 | 18,408 |
> | **dropped** | **7,362** | **0** |
> | AMBIGUOUS / UNRESOLVED | — | 0 / 0 |
>
> Per compilation: 17 + 2 + 5 = 24 clauses, 0 dropped. Suite 3838 passed / 4
> failed (the +1 is the new ambiguity test); no existing verdict changed.
> `sum()`/`product()` are covered by `stdlib/iterator/min_max_sorted.ks`
> (execution), which passes.
>
> **Caveat: the string key is gone from the clause layer only.**
> `Constraint::Associated { container, name: String, result, span }` is still
> keyed by name one layer down: consumers spell the entity back out with
> `resolve::assoc_name`. Getting to "no name matching" means making that
> constraint entity-keyed too. That is a separate change, noted and not done.
>
> **Step 4 landed (2026-09-23).** `HirTy::same_type` (in `kestrel-hir`) is a
> span-insensitive structural comparison, written by hand rather than derived.
> `Infer` and `Error` are never equal to anything. `constraint_entailed_by`
> now entails an `Equality` exactly when the context holds the same subject
> with a `same_type` RHS, with no substitution or transitivity.
> **Probe first** (temporary `g25-entail` ktrace, 3681 testdata files + 17
> `lang/` packages): the `Equality` arm was **never evaluated**, so **0
> answers flip** from false to true. Entailment itself is live, about 809
> calls per compilation, but no conformance check in the corpus reaches an
> extension equality clause. This is a correctness fix with no visible effect
> today; two unit tests pin it, span-insensitive entailment and its
> different-RHS/subject control. **Correction 2026-09-23:** as committed in
> `cfe147ad` those tests did not compile. The helper passed a `u32` to
> `Span::synthetic(usize)`, which never changed, so the step-4 report of them
> passing cannot be true. Fixed at `entailment.rs`; they pass now, along with
> all 21 of the crate's lib tests. `triage` does not run crate unit tests, so
> verifying a change also means running `cargo test -p <crate>`. Suite 3838 passed / 4 failed, no verdict
> changes. Remaining gap: `conformance_completeness::substitute_clause`
> passes equalities through unsubstituted, so a protocol-param subject can
> only match a context clause on the very same entity. That is conservative,
> and noted, not done.
> Found while trying to close G17's C6. Every claim below carries a provenance
> stamp per [`../../contributing/verifying-claims.md`](../../contributing/verifying-claims.md).

## The finding

Kestrel lets you write an associated-type equality clause, and the stdlib does:

```kestrel
// lang/std/iter/iterator.ks:114   [verified @ 58e2c550]
type TargetIterator: Iterator where TargetIterator.Item = Item
```

That clause states a type identity: this `Iterable`'s `TargetIterator.Item`
**is** its own `Item`. Nothing evaluates it.

```rust
// entailment.rs   [verified @ 58e2c550]
// TypeEquality / DirectEquality carry HirTy on the RHS, which has
// no structural equality. Reject conservatively until a real
// caller demands proper handling …
WhereClause::TypeEquality { .. } | WhereClause::DirectEquality { .. } => false,
```

The two emitters that would carry such a clause into the solver —
`emit_protocol_assoc_type_where_clauses` (`lib.rs`) and
`emit_type_alias_where_clauses` (`solver.rs`) — are the pair D7 classified as
**"inert features"** and deliberately preserved as skips. They are not missing;
they are switched off.

## Why the stdlib works anyway — and why that is the bug

`Iterable`/`Iterator` interoperate because `solver.rs`'s associated-type
**name-equality fallback** notices that `Iterable.Item` and `Iterator.Item` are
both *spelled* `Item` and treats them as interchangeable.

> **The name fallback is not a shortcut for the equality clause. It is the only
> implementation of it.**

Measured [@ `a8fa672c`, `KESTREL_DEBUG=audit-subject`, 3655 files]: the fallback
fires **27× per compilation**, on every file including ones that pass, and
**every single hit is cross-entity — zero same-receiver**. All 27 are doing the
job the equality clause was supposed to do.

This is the same defect class as G17 itself — **matching by name instead of by
identity** — surviving because the correct mechanism was never built. A
workaround became infrastructure.

### The witness

`assoc_projection_bound_same_name_distinct_protocols.ks` — a **silent
miscompile**, `Int64.show()` executed on a `String`, printing `result=i`.
Deterministic output, unlike G17's `leak5` (an ASLR heap address), so it is the
better regression witness of the two.

Two unrelated protocols each declare an associated type named `Item`; the
fallback matches them across protocols and the wrong witness is selected. Rename
the aliases apart (`_control.ks`) and the compiler rejects correctly.

**This is why G17's C6 is blocked.** Narrowing the fallback to require matching
bases does not narrow it — with zero same-receiver hits, it **deletes** all 27
uses and takes the shipped `Iterable`/`Iterator` bridge with it. You cannot
remove the crutch until the leg works.

## What already exists

Three of the four pieces are in tree [all verified @ `58e2c550`]:

| piece | state |
| --- | --- |
| `Constraint::Equal { a: TyVar, b: TyVar, span }` | **exists** — nothing routes equality clauses into it |
| `emit_protocol_assoc_type_where_clauses`, `emit_type_alias_where_clauses` | **exist**, currently skip projection subjects |
| Representation of `TargetIterator.Item` | **exists since G17 C9** — see below |
| Evaluation | missing |

### The representation blocker dissolved without anyone noticing

Earlier analysis called `TargetIterator.Item` unrepresentable because its base
is a `TypeAlias` and `resolve_projection_subject` accepts only `TypeParameter`
roots. **That was an artifact of not desugaring bare associated-type names.**

Inside a protocol, a bare `TargetIterator` *means* `Self.TargetIterator`. So the
subject is:

```
Projection { base: Projection { base: SelfType, assoc: TargetIterator }, assoc: Item }
```

The root is `SelfType`, not a `TypeAlias`. That became constructible when G17 C9
started emitting `SelfType`, and arbitrary depth arrived in D7 commit 2. Nobody
noticed the blocker had gone.

## Design

### 1. Fold the two variants into one

```rust
Equality { subject: WhereSubject, rhs: HirTy }
```

replacing `TypeEquality { param: Entity, assoc_name: String, rhs: HirTy }` and
`DirectEquality { param: Entity, rhs: HirTy }`.

This is D7's deferred 4 → 2 and it kills the **last base-free key** — the
`String`-keyed assoc name. `T.Item = X` becomes
`Projection { Param(T), Item }`; `V = X` becomes `Param(V)`.

D7 deferred this for a stated reason that still applies and must be respected:
it is a **resolution** change that can *silently drop* clauses, because
`WhereClausesOf` drops what it cannot resolve. Behaviour-preserving commit,
measured, no fixes bundled.

### 2. Emit the unification

Where the clause is in force, emit
`Equal(lower_subject(subject), lower_hir_ty(rhs))`.

That is exactly what the name fallback achieves by accident. *"These share a
spelling, so reuse the TyVar"* becomes *"the clause says they are equal, so
unify them"* — same outcome, correct reason.

### 3. Give `HirTy` a span-insensitive comparison

The blocking comment says `HirTy` "has no structural equality." Literally true:
it derives `Clone, Debug, Hash` and **not `PartialEq`** [verified @ `58e2c550`].

It derives `Hash`, so the fields are comparable and a derive would compile —
**but `HirTy` carries `span` fields**, so a derived `PartialEq` would call two
syntactically identical clauses unequal. This needs a normalized or
span-skipping comparison, written deliberately. Do not reach for
`#[derive(PartialEq)]`.

### 4. Retire the name fallback

Once identity flows from the clause, `solver.rs`'s name fallback can go and
`same_name_distinct_protocols` flips green.

## Sequence

Same shape that worked for G17 — **detection before estimate.**

| # | commit | gate |
| --- | --- | --- |
| **1** | **Detection only.** Instrument the name fallback: for each of its 27 hits, log whether an equality clause in scope would have covered it. Follow the `KESTREL_DEBUG=audit-subject` precedent from G17 C1 and `KESTREL_AUDIT_DUP` in `kestrel-mir`. | suite unchanged, bit-identical |
| **2** | Fold `TypeEquality`/`DirectEquality` into `Equality`. Behaviour-preserving. | suite unchanged |
| **3** | Emit the unification. **The real change.** | measured, not predicted |
| **4** | Delete the name fallback. | `same_name_distinct_protocols` flips green |

Commit 1 is the important one. If the answer is 27/27, deletion is provably
safe. If it is 20/27, seven cases exist that nobody understands **before**
anything changes.

**Predict no deltas in this document.** Four expected-outcome claims in
`plan-3a.md` were extended by analogy and refuted by measurement (C4's third
test, C7's site table, C9's annotation, C11's witness — the last one *inverted*,
declaring a live site dead). Do not add a fifth.

## Risks, ranked

1. **Over-constraint.** Emitting `Equal` where nothing existed forces
   unifications that previously stayed open, turning inferred programs into type
   errors. Unlike every G17 commit, this is neither reject-direction nor
   permit-direction — it is **both**, which makes it harder to reason about and
   makes commit 1 more valuable, not less.
2. **Ordering.** Equality clauses must be emitted *before* the uses they inform,
   or the unification arrives after the wrong type is already committed. Same
   class as the R3/R4 cascade that made G17 C3 a no-op.
3. **Cycles.** `where A.Item = B.Item, B.Item = A.Item` must terminate.
   `conformance.rs`'s depth guard falls back to `return true`, so a cycle here
   costs **non-termination, not a wrong answer**.
4. **Silent clause drops** (see §1) — a folded variant that fails to resolve
   disappears rather than erroring.

## Not in scope

- G17's remaining scope half — [`../G14-G17/plan-scope.md`](../G14-G17/plan-scope.md), commits S1–S4.
- G14 entirely.
- The 16 raw `AstWhereClause` walkers outside `kestrel-type-infer`. Two of them
  (`kestrel-name-res`) match `Self` by raw string compare and are genuinely
  buggy, but cannot use `WhereClausesOf` — name-res has zero references to
  `kestrel-type-infer`, and `WhereClausesOf` depends on name-res. Dependency
  cycle; separate finding.
