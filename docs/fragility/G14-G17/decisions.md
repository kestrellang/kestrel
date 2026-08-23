# G14 + G17 — open decisions

Collaboration surface for the where-clause-subject work. Ground truth and
evidence live in [`problem.md`](problem.md); this file holds **questions,
options, and answers**.

## How to use this file

- Every decision gets a `D<n>` heading, a **Status** line, and options with
  their real costs. Do not delete an option — mark it rejected and say why.
- **Record who decided and on what evidence.** "Maintainer, 2026-08-20" or
  "measured: suite run under `KESTREL_AUDIT_SUBJECT`". A decision with no
  evidence line is a guess and should be labelled one.
- If you find something that **refutes** a claim in `problem.md`, edit that
  file directly and note it here under *Refutations landed*. Being corrected is
  cheaper before implementation than after — three of the original audit claims
  and three of the first-pass analysis claims have already fallen.
- Claim a decision by putting your name/agent on the **Owner** line before
  working on it, so two agents don't design the same thing twice.

## Coordination

| | |
| --- | --- |
| **Audit IDs** | `G14`, `G17` (= addendum `A15`), `A16`; interacts with `G16`, `F9` |
| **Primary crates** | `kestrel-type-infer` (`conformance.rs`, `where_clauses.rs`, `resolve.rs`, `entailment.rs`), `kestrel-analyze` (`compilation/conformance_completeness.rs`) |
| **Shared branch** | `arch/fixes` — multiple agents. **Never bare `git commit`; scope every commit to explicit paths.** |
| **Tests** | `/triage` skill only. Never `cargo test -p kestrel-test-suite`. Never edit a test to make it pass. |
| **Build** | `cargo build --release --bin kestrel` (the binary is owned by the root crate, not `kestrel-compiler-driver`) |

**Danger zone for concurrent edits:** `conformance.rs:313-373`
(`extension_bounds_hold_impl`) and `conformance_completeness.rs:1550-1660`
(`collect_provided_members_for_conformance` / `extension_clauses_entailed`).
If you are touching either, say so here first.

## Sequencing — read this before starting anything

**G14 and G17 cannot proceed in parallel.** D7 changes the shape of
`WhereClause`, and both danger-zone files above *consume* `WhereClause`.
Whoever lands the type change invalidates the other's working tree mid-edit.

Order:

| # | work | who | may run concurrently with |
| --- | --- | --- | --- |
| 1 | ~~G22~~ — **withdrawn, does not exist on this branch** | — | — |
| 2 | **D7 type change**, behaviour-preserving | orchestrator | nothing else in `kestrel-type-infer` |
| 3a | G17 behavioural rewire | unclaimed | 3b |
| 3b | G14 (arity zip + D1) | unclaimed | 3a |

**G22 was withdrawn the day it was filed.** It was going to be step 1 on the
reasoning that `emit_all` aborting on the first unrenderable diagnostic left
anyone in type-infer debugging blind. That code was replaced by `e0cb32cb` on
2026-06-18; `emit_all` already collects-and-continues and `emit_one` already
degrades to a spanless render, with two passing tests. The finding came from a
probe running in a worktree pinned to `v0.16.0` — audit finding **G24**.

**Before you measure anything for this work, check your base.** `git log -1`
in an agent worktree may show a commit 173 behind `arch/fixes`. Everything in
`problem.md`'s second pass is tagged VERIFIED or MEASURED-ON-v0.16.0 for this
reason; do not promote a MEASURED claim without re-running it here.

**Step 2 must not fix the skip sites — and there are SIX, not four.**
[verified @ `296e3076`] Collapsing `Bound` and `ProjectionBound` into one
variant forces every `ProjectionBound { .. } => {}` site to handle a case it
was skipping. Preserve today's behaviour at **all six** with an explicit
projection skip and a `TODO(G17 stage 3a)`:

| site | today | classification |
| --- | --- | --- |
| `solver.rs:3565` | `=> {}` | **live bug** — call-site obligation for a direct `Def` call |
| `solver.rs:4485` | `=> {}` | **live bug** — same, member/method path |
| `lib.rs:812` | `=> {}` | **live bug** — the only emitter for container-level clauses |
| `generate.rs:1972` | `=> {}` | **live bug** — call-site / type-formation path |
| `lib.rs:958` | `=> {}` | inert feature — protocol assoc-type clauses |
| `solver.rs:5798` | `=> {}` | inert feature — type-alias clauses |

Plus two non-`{}` sites that must keep their current answers: `lib.rs:348`
(the one real consumer) and `entailment.rs:45` (`false`, correct as-is).

**An earlier draft of this note listed only four.** Missing `lib.rs:958` and
`solver.rs:5798` would have made step 2 silently *change* behaviour at two
sites — destroying the "reviews as no behaviour change" property that is the
entire reason step 2 is sequenced first and reviewed separately.

Fixing any of them inside a multi-file refactor buries a semantic change in
mechanical noise; they get individual commits with individual tests in 3a.

---

## D1 — Is the third evaluator in scope?

**Status:** OPEN — blocking. **Owner:** unclaimed.

`conformance_completeness.rs:266-271` + `:369` decides the same-protocol default
case and **never reads where clauses**. It is why §A13's scenario does not
reproduce.

| option | cost | consequence |
| --- | --- | --- |
| **(a) In scope** | largest; touches `E454`'s main path | The only version where "the clauses are evaluated" is true for the common case |
| **(b) Out of scope, filed separately** | smallest | G14 gets fixed on two paths most programs never take. Honest only if the new finding is filed and the audit says so |
| **(c) Out of scope, unfiled** | — | **Rejected.** Leaves the audit claiming a fix that does not cover the reachable path |

**Recommendation:** (a), because (b) means shipping a fix whose own repro is
the case it does not cover. Note this makes G14 materially bigger than
"delete the second evaluator".

---

## D2 — What is `recv` when the analyzer asks whether a protocol-extension member is provided?

**Status:** OPEN — blocks D5. **Owner:** unclaimed.

The analyzer holds a `ResolvedTy` for the conformer. `extension_bounds_hold`
wants an `HirTy`.

Per `problem.md`, `resolved_ty_to_hir` + `self_type_for_compare` is **safe**
(an `Infer` arg excludes a specialized extension rather than selecting one) but
**weak** (a generic conformer degrades to today's permit).

| option | cost | consequence |
| --- | --- | --- |
| **(a) Reify via `resolved_ty_to_hir`, accept the ceiling** | ~0 | Concrete conformers get real answers; generic ones stay permissive. Ceiling must be documented at the call site, not just here |
| **(b) Thread a real `HirTy` for the conformer** | unknown — needs a survey of what the analyzer has at `:1578` | Removes the ceiling |
| **(c) Give the binder a `ResolvedTy` arm** | medium | A third representation in the family the work exists to shrink. Weak option |

**Recommendation:** (a) for the first change, with the ceiling written into the
function's doc comment as a known limit and a follow-up filed — **provided D3
does not depend on beating it.**

---

## D3 — Does the binder handle projections (`I.Item`) in the first change?

**Status:** OPEN. **Owner:** unclaimed.

`WhereClause::ProjectionBound` keeps `{ base, assoc }`, and one real consumer
exists (`type-infer/src/lib.rs:348`, method-level bounds). Evaluating it means
resolving `base` to a concrete type and projecting `assoc` on it.

| option | cost | consequence |
| --- | --- | --- |
| **(a) `None` in the first change** | ~0 | G17 becomes "add one arm to a function that already exists" — a much better position than today. Unsoundness stays open meanwhile |
| **(b) Include it** | larger; needs the projection path *and* `resolve_projection_subject`'s `TypeParameter`-only restriction lifted so `Self.Item` stops collapsing | Closes the only unsound-accept in the audit |

**Recommendation:** (a). G17 is the more serious bug, but bundling the audit's
one soundness fix into a change that also rewires three evaluators makes both
harder to verify. Sequence them.

---

## D4 — Does `bind_clause_subject` also fix A16's parent walk?

**Status:** OPEN — low stakes. **Owner:** unclaimed.

A16's tier 2 is inert because bounds live on the *parent* decl. A binder that
owns the ancestor walk subsumes it. But no live wrong answer has been
constructed from A16 (see *Inferred, not observed* in `problem.md`).

**Recommendation:** let it fall out if the binder needs the parent walk anyway;
do not add scope for it. Re-check whether A16 can be closed *after* D1–D3 land.

---

## D5 — How do we prove the evaluators agree?

**Status:** OPEN. **Owner:** unclaimed.

A direct differential test **cannot be written before D2 is decided** — pairing
the two functions requires manufacturing a `recv`, which is D2. Build it the
obvious way and both sides permit, so the test proves nothing. A two-way
differential would also report "agree" on cases the third evaluator decides
(D1).

| option | cost | value |
| --- | --- | --- |
| **(a) Env-gated audit at `conformance_completeness.rs:1578`** | ~30 lines, no fixtures | `extension_clauses_entailed` already runs there with `extension` and `type_entity` in hand — also call `extension_bounds_hold` and log disagreements. Follow the `KESTREL_AUDIT_DUP` precedent in `kestrel-mir`. Running the existing suite under it enumerates every reachable `(extension, receiver)` pair for free: ~118 files in `declarations/extensions`, 40 containing `extend … where` |
| **(b) `.ks` pairs per shape** | ~0 Rust | Repros `c_assoc_witness.ks` (diagnostics) and `d_solver_permit.ks` (execution) already are this pair. Not exhaustive; pins the specific divergence |
| **(c) Rust differential test** | needs `pub(crate)` on `extension_clauses_entailed` | Only meaningful after D2 |

**Recommendation:** (a) **first** — it is the only option that produces the pair
list the others need, and it is the cheapest way to find out whether the blast
radius is what `problem.md` claims. Then (b) as the permanent regression tests.

---

## D6 — Audit-doc corrections

**Status:** OPEN — should land with the first commit. **Owner:** unclaimed.

Per `docs/fragility-audit.md`'s own "keeping this current" rule:

1. §A13's failure scenario does not reproduce — replace it with the
   cross-protocol witness shape (`c_assoc_witness.ks`).
2. G14's statement is too narrow — it is not bare-assoc-specific; restate as
   the arity mismatch between `LowerExtensionTargetTypeArgs` and
   `hir_args(recv)`.
3. A16's "Correction to the finder's title" is itself wrong —
   `type Item: Equatable` is stored as `Conformances`, not `AstWhereClause`.
4. File the third evaluator as a new finding if D1 lands as (b).

---

---

## D7 — What represents a where-clause subject?

**Status:** **DECIDED — recursive, arbitrary depth.** Maintainer, 2026-08-20.
**Owner:** orchestrator (this session).

The subject has four spellings — `T`, `Self`, `T.Item`, `T.Iter.Item` — and
today three different types try to hold it, each dropping something:

| holder | drops |
| --- | --- |
| `WhereClause::Bound { param: Entity }` | the base, for any projection |
| `WhereClause::ProjectionBound { base: Entity, assoc: Entity }` | depth > 2 |
| `TypeEquality { param, assoc_name: String }` | keys the assoc by **name string** |

### Decision

One recursive type, in `kestrel-type-infer/src/resolve.rs` beside `WhereClause`:

```rust
/// What a where-clause bound is *about*.
///
/// One type for every spelling, so no code path can hold a subject whose
/// receiver it has silently dropped. Nests to arbitrary depth.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum WhereSubject {
    /// `T`
    Param(Entity),
    /// The `Self` position. NOT collapsed to the enclosing entity: in a
    /// protocol extension `Self` is the *conformer*, resolved per-conformance.
    /// Pinning it to the protocol entity is the same receiver-loss bug one
    /// level up.
    SelfType,
    /// `<base>.<assoc>`, to any depth.
    Projection { base: Box<WhereSubject>, assoc: Entity },
}
```

and the clause enum collapses 4 variants → 2:

```rust
pub enum WhereClause {
    Bound    { subject: WhereSubject, protocol: Entity, protocol_type_args: Vec<HirTy> },
    Equality { subject: WhereSubject, rhs: HirTy },
}
```

### Why this shape

**It deletes the bug class rather than fixing an instance.** `ProjectionBound`
exists *only* because `Bound`'s subject couldn't express a projection. Every
`WhereClause::ProjectionBound { .. } => {}` arm is a place someone handled
`Bound` and skipped the sibling. With one variant **there is nothing to skip**:
the match arm is the same arm.

[verified @ `296e3076`] **8** match sites, **6** no-ops, of which **4** are live
bugs — see the Sequencing table for the per-site breakdown. (An earlier draft
of this paragraph said "seven … four of those seven"; that was a survivor of
the refuted first-pass count.)

**But the explicit arms are a lower bound on the review surface.** A further
**13** sites match `Bound` *only* and let `ProjectionBound` fall through
implicitly — `if let`, `let … else { continue }`, `filter_map`, or a `match`
catch-all. Merging the variants silently **widens every one of them**. The
sharpest is `collect_context_where_clauses`
(`conformance_completeness.rs:1811-1817`), which **mutates the subject in
place**:

```rust
for clause in &mut clauses {
    if let ResolvedWhereClause::Bound { param, .. } = clause
        && let Some(&mapped) = decl_to_struct.get(param) { *param = mapped; }
}
```

Today a projection is a different variant and is untouched. After the merge
this starts rewriting projection *bases* unless it is written to look only at
the subject root. **Total review surface: 6 explicit + 2 non-`{}` + 13 implicit
= 21 decision points**, not 6.

Depth falls out for free. `T.Iter.Item` is
`Projection { base: Projection { base: Param(T), assoc: Iter }, assoc: Item }`.

**Justification corrected 2026-08-20** [verified @ `296e3076`]. This originally
rested on the fabrication chain "generating a depth-3 projection
(`I.Item.Item`)". **That premise is refuted** — `lib.rs:991` emits
`ctx.associated(self_tv, …)`, a *depth-2* projection rooted at `Self`; no
depth-3 shape is produced on that path.

The real justification is simpler and lives in source, not in a bug:
`resolve_projection_subject` bails at `where_clauses.rs:286`
(`segments.len() != 2`), so **`where T.Iter.Item: P` written by a user today
silently collapses** to the bare-assoc path. Shipped at
`declarations/associated_types/where_clause_on_nested_associated_type.ks:12`.
A flat pair cements that bail; recursion removes it. Better premise — it
does not depend on any disputed measurement.

`Equality` absorbs both `TypeEquality` and `DirectEquality`, killing the
name-string key: `T.Item = X` is `Projection { Param(T), Item }`, `V = X` is
`Param(V)`.

`SelfType` stays explicit rather than resolving to the enclosing entity, which
makes `Self.Item: P` representable — shipped at
`protocol_extension_mixed_self_constraints.ks:15`, and today it silently
collapses to bare-assoc (G14's shape).

### One bridge replaces three ad-hoc keys

```rust
fn lower_subject(ctx, &WhereSubject, subs) -> TyVar
```
`Param` → `subs.find`; `SelfType` → `self_tv`; `Projection` →
`ctx.assoc_projection(lower_subject(base), assoc)`.

That is `lib.rs:457-459` generalized. It replaces `get_or_create_subject_tv`'s
`Self` re-base (`lib.rs:991`) and `solver.rs:2836`'s cross-protocol name
fallback with one recursion.

**Scope limit — D7 alone does NOT fix the base-free substitution key**
[verified @ `296e3076`]. `where_clause_assoc_subs` is a `Vec<(Entity, TyVar)>`
keyed on the **assoc entity alone** (`ctx.rs:178`; 7 pushes, 6 reads). Two
subjects that share an assoc entity still collide on `find()`-returns-first
**no matter how the subject is represented** — `lower_subject` builds the right
TyVar and then the lookup throws the distinction away. Either re-key that
vector by `WhereSubject` as part of D7, or accept that the miscompile is not
closed until stage 3a. **Do not let step 2 imply the bug is fixed.**

An earlier draft of this section listed `param_tyvars` aliasing as one of the
mechanisms `lower_subject` displaces. That claim is **refuted** — no
`emit_where_clauses` exists at HEAD (or at `v0.16.0`), and `lib.rs:314` is a
comment. The `T.Assoc` path was fixed by #184; the aliasing did not disappear,
it moved into `where_clause_assoc_subs`, which is the paragraph above.

### Costs and risks, stated up front

- ~~**Path resolution needs an order.**~~ **RETRACTED 2026-08-20** [verified @
  `296e3076`, read directly]. I flagged this as the main implementation risk —
  that resolving `T.Iter.Item` needs `T: HasIter` to find `Iter`, then `Iter`'s
  bound to find `Item`, requiring breadth-first-by-depth or a fixed point.
  **It is already solved.** `ResolveTypePath::execute` walks every segment
  (`resolve_type.rs:106-111`), and `resolve_segment` (`:154+`) has a dedicated
  `TypeAlias` arm — *"look for nested associated types via its bounds (e.g.
  `T.Iter.Item`)"* — with the ordering constraint documented above it
  (*"type-param associated types are checked before nested alias bounds"*).
  Arbitrary depth resolves today.

  **The only thing refusing it is `resolve_projection_subject`'s own guard**
  (`where_clauses.rs:286`), and that guard exists to protect its return type:
  `Option<(Entity, Entity)>` cannot *express* a chain, so the function refuses
  the input rather than returning something it has no way to say. The in-source
  comment concedes it: *"Only the depth-1 `T.Assoc` shape for now … deeper
  chains fall through to the collapsing path."*

  This is D7's thesis found in the wild, and it **lowers** the estimate: what is
  needed is not new resolution logic but a walk that returns the *chain* of
  entities rather than only the last, so the recursive subject can be built
  from it. Small addition on existing, already-ordered code.
- **`SelfType` explicit touches the one branch that works** —
  `conformance.rs:363`'s `Some(*param) == target_entity`, pinned by
  `h_selfq_neg.ks`. Keep that test green or explain the change.
- `NegativeBound` (`where T: not Copyable`) stays unmodeled, as today
  (`where_clauses.rs:122-124`). Not this change's job, but note that three
  stdlib projection bounds are `I.Item: not Copyable` / `: Copyable`.
- Interning `WhereSubject` to a `SubjectId` is available later if `Box` +
  derived `Hash` ever shows up in a profile. Not now.

### Rejected

| option | why |
| --- | --- |
| Flat `{ base: Entity, assoc: Entity }` | cements the `segments.len() != 2` bail at `where_clauses.rs:286`, which silently collapses `where T.Iter.Item: P` — a shape users can write today and one shipped test already uses |
| Keep `Bound` + `ProjectionBound` separate | preserves the skip-the-sibling failure mode that produced four of the live bugs |
| `Root(Entity)` with `Self` resolved to the enclosing entity | re-introduces receiver loss one level up; `Self` in a protocol extension is the conformer, not the protocol |

---

---

## D8 — `SelfType` is defined but not constructed in the representation commit

**Status:** **DECIDED — option (a), payload-free, inert until stage 3a.**
Maintainer, 2026-08-20. **Owner:** orchestrator.

D7's `SelfType` and the behaviour-preserving constraint are in tension.
Today `Self` resolves to the **enclosing entity**, and ~8 readers depend on
that — notably `conformance.rs:363`'s `Some(*param) == target_entity`, pinned
by `h_selfq_neg.ks`. Constructing a distinct `SelfType` removes the entity
those readers compare, which is a behaviour change in the one commit that must
not have any.

| option | shape | verdict |
| --- | --- | --- |
| **(a) define, don't construct** | `SelfType` (no payload); resolver keeps emitting `Param(enclosing)` | **CHOSEN** |
| (b) compatibility payload | `SelfType { enclosing: Entity }`, readers use a `root_entity()` helper | rejected — see below |

### Why (a)

**Correctness.** `Self` has no entity — it is a *position*, resolved
per-conformance; in a protocol extension it denotes the conformer, unknown
where the clause is read. That is the whole reason D7 keeps it separate. `(b)`
writes the rejected answer *into the type* and relies on discipline to stop
anyone reading it. A field nobody may read is not modeling the domain; it is a
known-false value stored where people look for the truth.

**Honesty of the interim state.** Under (a) the resolver emits `Param(Foo)` —
today's wrong behaviour, but the type is not yet claiming to model `Self`. It
reads as *not done*. Under (b) the type claims `Self` **is** modeled and hands
back the wrong entity. It reads as *done*. Wrong-and-unfinished beats
wrong-and-finished; the recurring failure mode in this audit is not missing
information but information that looks settled and is not.

**The compiler does the counting.** When stage 3a switches the producer, every
reader matching on `WhereSubject` without a `SelfType` arm **stops compiling** —
the work list is generated, not remembered. The number of affected sites in
this area has been wrong four times (seven → four → six → twenty-one); this is
not a place to trust hand-enumeration. Under (b) a reader calling
`root_entity()` keeps the wrong answer **silently and forever**: no compile
error, no failing test, nothing that forces its removal.

**(b) buys sequencing only** — incremental reader migration — at the price of a
permanent defect in the type. Compatibility fields that outlive their reason
are exactly how `param_tyvars`, the flat `ProjectionBound`, and the base-free
`where_clause_assoc_subs` key all arrived. (a)'s atomic flip is ~8
same-shaped comparison sites in one crate, smaller than the merge in commit 1.

### Consequence, stated plainly

`Self.Item: P` **keeps collapsing** until stage 3a. That is the status quo, not
a regression — but do not let the commit message imply otherwise.

## Decided

- **D7** — `WhereSubject` is recursive, arbitrary depth; `Bound` and
  `ProjectionBound` collapse into one variant. Maintainer, 2026-08-20.
- **D8** — `SelfType` is payload-free and **not constructed** in the
  representation commit; the resolver keeps emitting `Param(enclosing)` until
  stage 3a flips producer and readers together, compiler-enforced. Maintainer,
  2026-08-20.

## Refutations landed

- **2026-08-20** — §A13's scenario does not reproduce; a third evaluator masks
  it. Source: diagnosis agent, repro `temp/g14/a_assoc_subject.ks`.
- **2026-08-20** — G14 is not bare-assoc-specific; a plain `TypeParameter`
  subject fails identically. Source: repro `temp/g14/g_param_subject_control.ks`.
- **2026-08-20** — A16's counter-example is wrong (`Conformances`, not
  `AstWhereClause`). Source: `type_alias.rs:80-83`, repro `temp/g14/j_assoc_bounded.ks`.
- **2026-08-20** — **D3's premise.** D3 recommends deferring projections to a
  later change, reasoning that G17 is "the more serious bug" but bundling it
  hurts verification. Sound reasoning, understated severity: G17 **emits wrong
  code** (`leak5.ks`, verified by running), and the damage is not in the
  `conforms_to` arm D3 is about — it is `param_tyvars` aliasing every `_.Item`
  to one `TyVar`. Resequence: the subject fix lands **first and alone**, before
  either evaluator is touched. It is the smallest change, the only one that
  stops emitting bad code, and independent of the whole G14 evaluator question.
  Source: `problem.md` second pass.
- **2026-08-23** — **C3's headline claim.** "The commit that stops `leak5`
  emitting wrong code is C3" is refuted: C3 landed as specified and `leak5`
  still miscompiles. Cause of the bad claim — the C1 measurement isolated each
  miscompile to "exactly one read", which read as *independent* sites; R3 and
  R4 are actually the two arms of one `else if` at `solver.rs:2818-2850`, so
  fixing R3 just routes the query into the still-base-blind R4, which re-admits
  the same entry by name. Fix is a same-entity exclusion on R4 (not C6's
  base-awareness, and provably a no-op pre-C3); spike-verified, not landed.
  Source: `plan-3a.md` C3 ⚠ banner. **Fix landed as C3b (`76ddd5f7`)**;
  `leak5` is now a post-mono error, and C4 becomes the commit that makes it
  reject in the frontend.
- **2026-08-23** — **C4's expected delta.** "`assoc_projection_bound_call_site.ks`
  flips too — its accept comes from the same mechanism" is refuted: C4 landed and
  `call_site` still fails. Its callee is *correct* (the body projects off `A`, the
  clause is about `A`), so C4's base check finds a genuine match and permits, as
  it must; what is missing is the call-site obligation, which is C7. The file's
  own header comment already said this — the plan's C4 section contradicted it.
  Cause of the bad claim: C4's expected-delta list was extended from `leak5` to
  the other two post-mono-failing repros by analogy, and the analogy holds for
  `on_container` only. Real delta −2, not −3. Source: `plan-3a.md` C4 ⚠ banner.
- **2026-08-20** — "only one projection bound ships in the stdlib" is wrong;
  there are three (`adapters.ks:397`, `:666`, `:866`). Conclusion unaffected —
  `Copyable` is answered structurally before the arm — but the count is not.
  Source: re-grep of `lang/std`.
- **2026-08-20** — "`extension_bounds_hold` is the canonical evaluator, just
  delete the other one" is wrong: it permits unconditionally for protocol
  extensions, so deleting the analyzer's evaluator converts a false-reject into
  a blanket permit. Source: `conformance.rs:332`/`:366` traced against
  `LowerExtensionTargetTypeArgs` returning `Some(vec![])`.
