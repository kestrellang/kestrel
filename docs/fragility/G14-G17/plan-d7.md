# D7 — implementation plan: `WhereSubject`

> **Scope.** Implements decision **D7** in [`decisions.md`](decisions.md) —
> sequencing step 2, "the type change, behaviour-preserving". Does **not**
> implement G14 (step 3b) or G17's behavioural rewire (step 3a).
>
> **Provenance.** Every line number and claim below was read on
> `arch/fixes` @ `00144212` ("docs(fragility): retract D7's main implementation
> risk"), 2026-08-20, in the parent checkout `/Users/dino/Documents/Projects/kestrel`.
> No worktree. Anchors are given symbol-first per
> [`docs/contributing/verifying-claims.md`](../../contributing/verifying-claims.md).
> No production code was written; no throwaway spike was needed (the only
> borrow question — recursing with `&mut InferCtx` while holding `&WhereSubject`
> — is answered by inspection: the subject borrows from the clause `Vec`, never
> from `ctx`).

---

## 0. Headline: the maintainer's list of six is a lower bound

The brief names six `ProjectionBound { .. } => {}` arms that must keep
skipping. Verified — all six exist at HEAD at exactly the cited lines. But
those are only the sites where someone *wrote down* that they were skipping.

**Thirteen further sites match `Bound { .. }` and let `ProjectionBound` fall
through implicitly** — an `if let`, a `let … else { continue }`, a
`filter_map`, or a `match` catch-all. Collapsing the two variants silently
*widens* every one of them: a subject the site never saw now enters its body.
Three of the thirteen are outside `kestrel-type-infer`, and one of those
(`collect_context_where_clauses`, `conformance_completeness.rs:1811-1817`)
**mutates the subject in place**, so widening it would start rewriting
projection bases that are untouched today.

Full inventory in §4. The count that matters for review is **21 non-test
decision points**, not six.

Two further corrections for `decisions.md` (not applied — this plan is the
only file I am permitted to write):

- D7 *Why this shape* still says "seven `ProjectionBound { .. } => {}` sites …
  four of those seven". The Sequencing table's "SIX, not four" is right; the
  prose paragraph is a survivor of the refuted draft. Verified count at HEAD:
  **8** `ProjectionBound` match sites total, **6** of them `=> {}` — matching
  `problem.md`'s adjudication row.
- Q3's brief (and D7's retraction note) attributes `TypeResolution::SelfType`
  to `resolve_first_segment`. It is not there: bare `Self` returns from
  `ResolveTypePath::execute` itself (`resolve_type.rs:92-94`);
  `resolve_first_segment` (`:121-151`) never produces `SelfType`. This matters
  because `execute` short-circuits `Self` *before* the segment walk, which is
  exactly why the chain walk in §3 needs its own `Self` handling.

---

## 1. Where `WhereSubject` lives, and the visibility question

### Answer: beside `WhereClause` in `kestrel-type-infer/src/resolve.rs`. No new module.

```rust
// lib/kestrel-type-infer/src/resolve.rs, immediately above `enum WhereClause` (:117)

/// What a where-clause bound is *about*.
///
/// One type for every spelling, so no code path can hold a subject whose
/// receiver it has silently dropped. Nests to arbitrary depth.
///
/// Invariant: a `Projection` chain always bottoms out at `Param` or
/// `SelfType`; there is no baseless projection.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum WhereSubject {
    /// `T`
    Param(Entity),
    /// The `Self` position. NOT collapsed to the enclosing entity: in a
    /// protocol extension `Self` is the *conformer*, resolved per-conformance.
    ///
    /// **Not constructed yet** — see `TODO(G17 stage 3a)` in
    /// `where_clauses::resolve_bound_subject`. Producing it requires every
    /// reader to know the clause's owning entity, which most readers do not
    /// have; that threading is stage 3a's job. See plan-d7.md §2.2.
    SelfType,
    /// `<base>.<assoc>`, to any depth.
    Projection { base: Box<WhereSubject>, assoc: Entity },
}

impl WhereSubject {
    /// The subject entity iff this is a bare `Param` — the exact set of
    /// subjects the pre-D7 `WhereClause::Bound { param }` could hold.
    /// Every site that used to destructure `param` uses this, so
    /// "projections are skipped here" is one grep, not twenty match arms.
    pub fn as_param(&self) -> Option<Entity> {
        match self {
            WhereSubject::Param(e) => Some(*e),
            _ => None,
        }
    }

    /// Rewrite the root `Param` through `f`; leaves `SelfType` alone.
    /// Used by `conformance_completeness::collect_context_where_clauses`.
    pub fn map_root(&self, f: impl Fn(Entity) -> Entity) -> WhereSubject { … }
}
```

**Why not its own module.** `WhereClause` and `WhereSubject` are one type in
two halves; splitting them puts the doc comment explaining the relationship in
a third place. `resolve.rs` is 2292 lines but the enum sits at the top with
`MemberResolution`/`MemberKind`, which is where the crate's other
resolution-result vocabulary already lives. Single source of truth (root
`CLAUDE.md`) argues for adjacency.

**Who imports it.** Only two crates name `kestrel_type_infer::resolve::WhereClause`
today, so only two can need `WhereSubject`:

| crate | file | how |
| --- | --- | --- |
| `kestrel-type-infer` | `where_clauses.rs`, `conformance.rs`, `entailment.rs` | `use crate::resolve::{WhereClause, WhereSubject};` |
| `kestrel-type-infer` | `lib.rs`, `solver.rs`, `generate.rs`, `resolve.rs` | already path-qualified (`crate::resolve::WhereClause::…`) — extend the same style |
| `kestrel-analyze` | `compilation/conformance_completeness.rs:56` | `use kestrel_type_infer::resolve::{WhereClause as ResolvedWhereClause, WhereSubject};` |

Nothing else in the workspace references the type — verified by
`grep -rn "WhereClause" --include="*.rs" lib/` and filtering out the unrelated
`kestrel_ast_builder::WhereClause` (the AST component) and
`kestrel_mir::item::function::WhereClause` (the MIR signature type). Three
distinct types share the name; only the middle one changes.

### The `TyVar` trap does **not** apply

`TyVar` is `pub struct TyVar(pub(crate) u32)` (`ty.rs:10`) — the type is
public, the payload is not, so an out-of-crate holder can pass one around but
cannot build or read one. `WhereSubject` has no such asymmetry: `Entity`
(`kestrel-hecs`) is fully public and `Box` is std, so every variant is
constructible from `kestrel-analyze`. That matters concretely —
`substitute_clause` (`conformance_completeness.rs:1662-1688`) *constructs* a
`Bound`, and `collect_context_where_clauses` (`:1811`) mutates one.

The `TyVar` trap does bite **`lower_subject`** (§5), whose return type is
`TyVar`. That function must therefore live inside `kestrel-type-infer` and
must not be part of the signature any analyze-side caller needs. It isn't —
`kestrel-analyze` never lowers a subject to a TyVar; it works in `HirTy` /
`ResolvedTy`.

### Derives

`WhereSubject` gets `Clone, PartialEq, Eq, Hash, Debug` exactly as decided.
Note `WhereClause` itself derives only `Clone, Debug, Hash` — it cannot derive
`Eq` because `HirTy` (`kestrel-hir/src/ty.rs:16`) is `Clone, Debug, Hash`
only. So `WhereSubject`'s `Eq` is usable *within* subject comparisons (the
`already_bound` dedupes, the eventual `where_clause_assoc_subs` re-key) but
does not promote `WhereClause` to `Eq`. Do not add `Eq` to `WhereClause` in
this change; it would require touching `HirTy`, which is out of scope.

---

## 2. Scope decisions

### 2.1 `TypeEquality` + `DirectEquality` → `Equality` — **recommend: defer to a later commit**

D7 specifies 4 variants → 2. I recommend **4 → 3 now** (`Bound` +
`ProjectionBound` merge only), and the `Equality` collapse as a separate
follow-up, for three reasons that are about correctness, not just review size.

**(a) It is a resolution change, not a representation change, and it can drop
clauses.** `TypeEquality` is built by `extract_associated_type_path`
(`where_clauses.rs:384-406`), which resolves **only the base segment** and
keeps `assoc_name: String` verbatim. It never asks name-res whether the assoc
name resolves to anything. To key the assoc by `Entity`, the builder must
resolve the full path — and that resolution can fail, at which point the
clause is silently dropped (`resolve_where_clauses`'s policy for unresolvable
subjects, `:71-73`). The live cases this would put at risk ship in the stdlib:

```
lang/std/iter/iterator.ks:1032   extend Iterator where Item: Addable, Item.Output = Item
lang/std/iter/iterator.ks:1049   extend Iterator where Item: Multipliable, Item.Output = Item
lang/std/iter/iterator.ks:114    type TargetIterator: Iterator where TargetIterator.Item = Item
lang/std/collections/set.ks:1396        … where T: Addable, T.Output = T, …
lang/std/collections/dictionary.ks:1976 … where V: Addable, V.Output = V, …
```

`Item.Output` is a projection whose **base is a `TypeAlias`, not a
`TypeParameter`** — a shape `resolve_projection_subject` explicitly refuses
today (`:297-301`). Resolving `Output` on it means going through
`resolve_assoc_type_nested`'s alias-bound path. If it comes back `NotFound`
for any of these, `sum`/`product`/`Set +`/`Dictionary +` lose their equality
constraint. That is precisely the class of silent regression this commit is
sequenced first to avoid.

**(b) The consumers need the name back.** Every reader of `TypeEquality` feeds
`assoc_name` straight into `InferCtx::associated(container, name, result, span)`
(`ctx.rs:796`), which is **name-keyed** by construction — `Constraint::Associated`
carries a `String`. Sites: `lib.rs:486`, `:769`, `:937`; `solver.rs:3553`,
`:4458`, `:5788`; `generate.rs:1958`. Entity-keying the clause just means each
of those does `ctx.get::<Name>(assoc)` to convert back, adding a lookup that
can return `None`. That is a lossy round-trip, not a simplification. The
constraint language has to go entity-keyed *first* — a much larger change.

**(c) Two different entity-resolution paths would disagree.** `TypeEquality`
readers also call `find_assoc_type_in_bounds(ctx, param, assoc_name)`
(`lib.rs:1097-1117`) to get an entity for `where_clause_assoc_subs`. That
resolves through `TypeResolver::resolve_associated_type` — the *solver's*
view. `ResolveTypePath` resolves through name-res's view. Making the clause
carry one of them changes which entity ends up in `where_clause_assoc_subs`,
and that vector is the exact structure G17's miscompile lives in. Changing it
under the banner "no semantics changed" is not defensible.

**Recommendation.** Ship `Bound`/`ProjectionBound` → `Bound { subject }` now.
Record `Equality` as **D7b**, sequenced after stage 3a has re-keyed
`where_clause_assoc_subs` by `WhereSubject` — at which point the entity is
needed anyway and the change pays for itself. If the maintainer wants 4→2 in
one go regardless, the plan is unchanged except that commit 2 (§7) grows an
`Equality` variant and the risk table gains R1 at **high**.

### 2.2 `SelfType` is defined but **not constructed** in this change

This is the one place where "behaviour-preserving" and D7's decided shape pull
apart, and it needs to be stated before implementation rather than discovered
during it.

Today a bare `Self` subject resolves through `resolve_type_entity`
(`where_clauses.rs:314-333`) → `TypeResolution::SelfType` →
`resolve_self_entity` (`:336-351`) → the enclosing struct/enum/protocol, or an
extension's `ExtensionTargetEntity`. It is stored as `Bound { param: <that
entity> }` and **every downstream reader treats it as an ordinary param
entity**. The load-bearing one is `extension_bounds_hold_impl`
(`conformance.rs:361-367`):

```rust
} else if Some(*param) == target_entity {
    recv.clone()
```

— pinned by the shipped negative test `h_selfq_neg.ks` and called out in D7's
own risk list. Two more depend on the same collapse:
`WorldResolver::collect_extension_where_clause_protocols` (`resolve.rs:2074-2081`,
comment: *"`Self: Protocol` — param is the target protocol entity"*), and
`emit_container_where_clauses`'s `get_or_create_subject_tv` path
(`lib.rs:707`, `:969-1001`).

Switching to `WhereSubject::SelfType` means each reader must map it back to
"the enclosing entity". **Most readers do not have the enclosing entity.**
`constraint_entailed_by` (`entailment.rs:31`) receives `&WhereClause` and
`&[WhereClause]` with no owner. `find_protocol_type_args_from_bounds`
(`resolve.rs:1699`) has the *param's* owner, not the clause's. Deriving it
would mean a parent walk per read — new resolution work inside a commit whose
entire property is that it does none.

**Therefore:** define `SelfType`, document it, do not construct it. The
variant is `pub` in a `pub mod` of a library crate, so `dead_code = "warn"`
(root `Cargo.toml:85`) does not fire on an unconstructed public variant. The
`TODO(G17 stage 3a)` in `resolve_bound_subject` names the missing input
(clause owner) so the next person does not have to rediscover it.

> **Question for the maintainer.** A middle path exists and was rejected here
> only because D7 is decided: `SelfType { enclosing: Entity }`. It is
> distinguishable *and* every current reader keeps today's answer by reading
> `.enclosing`, so it could be constructed immediately and stage 3a would flip
> readers one at a time instead of all at once. It costs the "no payload"
> purity of the decided shape. If you want the flip inside this change rather
> than stage 3a, this is how it becomes behaviour-preserving. Say so and I'll
> re-plan; otherwise I proceed with the unconstructed variant.

### 2.3 What this change explicitly does **not** fix

Copied forward so no reviewer infers otherwise, per D7's *Scope limit*:

- `where_clause_assoc_subs` stays a `Vec<(Entity, TyVar)>` keyed on the assoc
  entity alone (`ctx.rs:175-178`, pushed at `lib.rs:461`, `:493`, `:517`,
  `:788`, `:871`, `:905`, `:954`). **`leak5.ks` still miscompiles after this
  change.** Add `TODO(G17 stage 3a)` at `ctx.rs:178` pointing at D7's scope
  limit; do not re-key.
- `solver.rs:2836`'s cross-protocol `Name`-equality fallback is untouched.
- All four live-bug skip sites stay skipping.
- `NegativeBound` stays unmodeled (`where_clauses.rs:122-124`).

---

## 3. Building the subject — the chain walk

### The bail, and why it exists

`resolve_projection_subject` (`where_clauses.rs:275-312`) returns
`Option<(Entity, Entity)>` and refuses anything that is not exactly
`TypeParam.Assoc`:

- `:286` — `if segments.len() != 2 { return None; }`
- `:297-301` — base must be `NodeKind::TypeParameter`
- base is resolved by `ResolveTypePath { segments: vec![segments[0]] }`, so a
  bare `Self` base returns `TypeResolution::SelfType`, *not* `Found`, and
  falls out at `:295` — which is why `Self.Item: P`
  (`protocol_extension_mixed_self_constraints.ks:15`) collapses.

Everything it refuses falls to `resolve_type_entity` (`:314`), which resolves
the **whole dotted path in one shot** and keeps only the last entity. So
`C.Iter.Item: Equatable` is stored today as `Bound { param: Item }` — base
gone, exactly as D7 says.

### Recommendation: a **free function in `kestrel-name-res`**, with `ResolveTypePath` delegating to it

Not a new query, not a local walk in `where_clauses.rs`.

```rust
// lib/kestrel-name-res/src/resolve_type.rs

/// One resolved step of a dotted type path.
pub struct TypePathChain {
    /// Same answer `ResolveTypePath` gives.
    pub resolution: TypeResolution,
    /// Entity per resolved segment, in source order. `steps[i]` is
    /// `segments[i]`. Empty unless `resolution` is `Found`.
    pub steps: Vec<Entity>,
    /// The path began at the `Self` position (`segments[0] == "Self"`), so
    /// `steps[0]` is whatever `Self` was resolved *through* (a synthetic
    /// `Self` type param, or an extension target), not a user-written name.
    pub self_rooted: bool,
}

pub fn resolve_type_path_chain(
    ctx: &QueryContext<'_>,
    segments: &[String],
    context: Entity,
    root: Entity,
) -> TypePathChain { … }
```

and `ResolveTypePath::execute` becomes

```rust
fn execute(&self, ctx: &QueryContext<'_>) -> TypeResolution {
    resolve_type_path_chain(ctx, &self.segments, self.context, self.root).resolution
}
```

**Why a free function rather than a second query.** There is a house pattern
for exactly this, documented at `where_clauses.rs:47-50`: *"Free-function
implementation … separated from the query impl so it can be called directly by
other queries without going through the memoization layer when that wouldn't
help."* A second memoized query on the same key would double the cache entry
for every type path in the program to serve one caller. `WhereClausesOf` is
itself memoized, so the chain walk runs once per where-clause subject per
revision — negligible.

**Why not a local walk in `where_clauses.rs`.** `execute`'s ordering is
load-bearing and documented: `resolve_segment` (`:157-181`) checks
type-param associated types *before* nested alias bounds, with the constraint
written above it. `try_resolve_self_as_type_param` (`:224-261`) and
`try_resolve_self_via_extension_target` (`:268+`) each run their own segment
loop. A second copy would be a second source of truth for path resolution —
the exact defect this audit exists to find. Refactor `execute` to record its
steps; do not re-derive them.

**Implementation shape.** `execute`'s body already threads `current` through
`resolve_first_segment` then a loop over `segments[1..]`; recording `current`
into a `Vec` after each step is a two-line change. The two `Self.` helpers
need the same treatment — each already walks `segments[1..]` with a `current`
cursor (`:248-258`, `:292-299`). No new resolution logic anywhere, which is
what D7's retraction predicted.

### How `where_clauses.rs` uses it

`resolve_projection_subject` is replaced by one function that returns the
whole subject, projection or not:

```rust
/// Resolve a where-clause bound subject to a `WhereSubject`, preserving the
/// receiver at every depth. `None` = unresolvable (clause is dropped, as
/// today).
fn resolve_bound_subject(
    ctx: &QueryContext<'_>,
    ast_ty: &AstType,
    entity: Entity,
    root: Entity,
) -> Option<WhereSubject>
```

- `AstType::Named { segments }` with `len() == 1`: unchanged —
  `Found(e) => Param(e)`, `SelfType => Param(resolve_self_entity(..)?)`
  with the `TODO(G17 stage 3a)` marking where `WhereSubject::SelfType` goes.
- `len() >= 2`: `resolve_type_path_chain`. If `chain.self_rooted`, fall back
  to today's collapse (`resolve_type_entity` → `Param(last)`) with a
  `TODO(G17 stage 3a)` — see §2.2; the `Self` root has nothing to name yet.
  Otherwise require `steps[0]` to be `NodeKind::TypeParameter` (preserving
  `:297-301`) and fold:
  `steps[1..].iter().fold(Param(steps[0]), |base, &assoc| Projection { base: Box::new(base), assoc })`.
- Anything unresolvable → `None`, and the caller's existing
  `if projection.is_none() && param.is_none() { continue; }` (`:71-73`)
  becomes a single `let Some(subject) = … else { continue };`.

**Depth-1 output is bit-identical to today**, which is what makes commit 1
mechanical. Depth ≥ 2 is a real (small, enumerated) delta — see §7.

---

## 4. Every consumer, and what each becomes

Found by `grep -rn "WhereClause::" --include="*.rs" lib/` plus
`grep -rn "WhereClause\b"` for type-position references, at `00144212`. The
`kestrel-mir` and `kestrel-ast-builder` hits are different types with the same
name and are excluded.

### 4.A — Construction (4 sites, all in `where_clauses.rs`)

| line | today | becomes |
| --- | --- | --- |
| `:82` | `ProjectionBound { base, assoc, .. }` | **deleted**; folded into `:88` |
| `:88` | `Bound { param: param.unwrap(), .. }` | `Bound { subject, .. }` from `resolve_bound_subject` |
| `:208` | implicit `T: Copyable` → `Bound { param }` | `Bound { subject: WhereSubject::Param(param) }` |
| `:262` | implicit `T: Static` → `Bound { param }` | same |

### 4.B — The six documented skips: keep skipping

Every one becomes `let Some(param) = subject.as_param() else { continue; };`
(or `=> {}` → the same guard inside the merged arm), each carrying
`// TODO(G17 stage 3a): projections are skipped here — see docs/fragility/G14-G17/decisions.md`.

| site | function | classification (from `decisions.md`) |
| --- | --- | --- |
| `solver.rs:3565` | `Def`-call where-clause emission (`:3524` query) | live bug |
| `solver.rs:4485` | member/method path, over `resolution.where_clauses` | live bug |
| `lib.rs:812` | `emit_container_where_clauses` | live bug |
| `generate.rs:1972` | `emit_where_clause_constraints_with_subs` | live bug |
| `lib.rs:958` | `emit_protocol_assoc_type_where_clauses` (TypeAlias clauses) | inert feature |
| `solver.rs:5798` | `emit_type_alias_where_clauses` | inert feature |

### 4.C — The two documented non-`{}` sites: keep their answers

| site | today | becomes |
| --- | --- | --- |
| `lib.rs:348` | destructures `ProjectionBound { base, assoc, … }` → `emit_method_projection_bound_constraint` (`:447-471`) | merged arm dispatches on `subject`: `as_param()` → `emit_method_bound_constraint`, else → the projection emitter, now taking `&WhereSubject` and using `lower_subject` (§5). Identical for the only depth constructed in commit 1. |
| `entailment.rs:45-47` | `ProjectionBound \| TypeEquality \| DirectEquality => false` | `Bound` arm guards on `as_param()`; `None => false`. Same answer. |

### 4.D — Thirteen implicit-widening hazards (**not on the brief's list**)

Each currently matches `Bound` only, so `ProjectionBound` falls past it. After
the merge each must add `as_param()` or an explicit projection branch, or its
behaviour changes.

| # | site | shape today | required guard |
| --- | --- | --- | --- |
| 1 | `where_clauses.rs:202-206` `inject_implicit_copyable_bounds` | `matches!(wc, Bound { param: p, .. } if *p == param && …)` | compare `*subject == WhereSubject::Param(param)` — a `Projection` can never equal it, so behaviour is preserved *by the `Eq` derive*, no branch needed |
| 2 | `where_clauses.rs:256-260` `inject_implicit_static_bounds` | same | same |
| 3 | `conformance.rs:345-352` `extension_bounds_hold_impl` | `let Bound { param, protocol: pb, .. } = clause else { continue }` | `let Some(param) = subject.as_param() else { continue }` — **also preserves the `Self` branch at `:363`**, since `Self` is still `Param(target_entity)` (§2.2) |
| 4 | `entailment.rs:70-80` `bound_entailed` context scan | `filter_map(Bound { param: cp, .. } if *cp == param)` | `.filter(\|c\| c.subject.as_param() == Some(param))` |
| 5 | `entailment.rs:94-104` param-bounds scan | same | same |
| 6 | `generate.rs:2030-2035` `emit_copyable_wellformedness` | `let Bound { param, protocol, .. } = clause else { continue }` | `as_param()` guard |
| 7 | `resolve.rs:1713-1723` `find_protocol_type_args_from_bounds` direct match | `if let Bound { param, .. } = clause && *param == param_entity` | `clause.subject.as_param() == Some(param_entity)` |
| 8 | `resolve.rs:1730-1738` same fn, inherited match | same | same |
| 9 | `resolve.rs:2074-2082` `collect_extension_where_clause_protocols` | `if let Bound { param, protocol, .. }` then `param == target_protocol` | `as_param()` guard; keeps the `Self`-collapse contract documented at `:2078` |
| 10 | `solver.rs:5886-5891` static-wf emission | `let Bound { param, protocol, .. } = clause else { continue }` | `as_param()` guard |
| 11 | `conformance_completeness.rs:1642-1649` `extension_clauses_entailed` | `if let Bound { param, protocol, .. } = c && proto_subs.get(param)` | `as_param()` guard; projections fall to `substitute_clause` exactly as today |
| 12 | `conformance_completeness.rs:1662-1688` `substitute_clause` | `Bound` arm rewrites `param`; `other => Some(other.clone())` catches `ProjectionBound` | `Bound` arm must `match subject.as_param()`: `Some(p)` → today's rewrite; `None` → `Some(clause.clone())`, reproducing the old catch-all |
| 13 | `conformance_completeness.rs:1811-1817` `collect_context_where_clauses` | `if let Bound { param, .. } = clause && let Some(mapped) = … { *param = mapped }` — **in-place mutation** | rewrite only when `as_param()` is `Some`; leave projections untouched (today they are). `WhereSubject::map_root` is the obvious tool but must be gated so a `Projection` root is **not** remapped in this commit |

Hazard 13 is the one to watch: `map_root` is the semantically *desirable*
behaviour and the wrong thing to ship here. Gate it, `TODO(G17 stage 3a)`.

### 4.E — Tests that must be updated mechanically

`entailment.rs` `#[cfg(test)]` (`:114-283`) builds `WhereClause::Bound { param, protocol, protocol_type_args }`
**8 times** across 4 tests (`:183`, `:188`, `:219`, `:224`, `:246`, `:251`,
`:270`, `:275`). All become `subject: WhereSubject::Param(t)`. No assertion
changes. This is not "weakening a test" — it is the same test against the same
API with a renamed field.

### 4.F — Not affected, confirmed

`kestrel-mir-lower/src/items/witness_lower.rs:156` and
`function_sig.rs:316` construct `kestrel_mir::item::function::WhereClause` —
a different type in a different crate. No change.

---

## 5. `lower_subject`

### Where it lives

`lib/kestrel-type-infer/src/generate.rs`, next to `lower_hir_ty_with_subs`
(`:2053`) — the other "HIR-ish thing → `TyVar`" bridge, and the function the
existing subject sites already call for the *args* half of the same clause
(`lib.rs:433`, `:466`; `generate.rs:1944`). Putting it in `ctx.rs` would be
wrong: `InferCtx` must not know about `WhereClause`.

### Signature

```rust
/// Root policy for a subject's `Param` leaf.
pub(crate) enum SubjectRoot<'a> {
    /// Look the entity up in a caller-supplied substitution; `None` on a miss.
    /// This is the "skip the clause" policy (`solver.rs`, `generate.rs`).
    Subs(&'a [(Entity, TyVar)]),
    /// Mint-or-reuse via `InferCtx::param` — the "always succeeds" policy
    /// (`lib.rs`'s method path).
    Mint,
}

/// `lib.rs:457-459` generalized to arbitrary depth.
///
/// `Param`      → per `root`
/// `SelfType`   → `None` (TODO(G17 stage 3a): needs the clause owner)
/// `Projection` → `ctx.assoc_projection(lower_subject(base)?, assoc)`
pub(crate) fn lower_subject(
    ctx: &mut InferCtx<'_>,
    subject: &WhereSubject,
    root: SubjectRoot<'_>,
) -> Option<TyVar>
```

An enum rather than a closure: a `FnMut(&mut InferCtx, Entity)` cannot be held
across the recursive call without re-borrowing `ctx`. The enum sidesteps it
entirely and there are exactly two policies, both already in the tree.

`assoc_projection` (`ctx.rs:541-546`) allocates a fresh `TyVar` per call and is
**not** memoized, so one call per clause per emission is what happens today and
what continues to happen. Do not add caching here — that would merge
projection TyVars that are currently distinct.

### Which call sites adopt it in this commit, without behaviour change

| site | adopt? | why |
| --- | --- | --- |
| `lib.rs:457-459` (`emit_method_projection_bound_constraint`) | **yes** | it *is* the function. Becomes `lower_subject(ctx, subject, SubjectRoot::Mint)?`; identical output for a depth-1 `Projection { Param(base), assoc }` |
| `solver.rs:3532` (Bound arm) | **yes** | today `subs.iter().find(\|(e,_)\| *e == param)` and skip on miss — exactly `SubjectRoot::Subs`. `Some(tv)` → today's body, `None` → today's skip |
| `generate.rs:1938` (Bound arm) | **yes** | same `subs.find`-and-skip shape |
| `lib.rs:426` (`emit_method_bound_constraint`) | **no** | uses `ctx.param(param)` unconditionally, including for `TypeAlias` subjects (bare `Item`). `SubjectRoot::Mint` reproduces it, but only if the caller stops consulting `type_params`/`parent_type_params` — which it doesn't. Adopt in stage 3a. |
| `lib.rs:707`, `:759` (`get_or_create_subject_tv`) | **no** | its `TypeAlias` arm re-bases onto `self_tv` (`:993`) — the fabrication described in `problem.md`. Replacing it is a *fix*, not a refactor. `TODO(G17 stage 3a)`. |
| `solver.rs:4426-4434` | **no** | two-stage lookup (`resolution.type_params` first, then `subs`). Expressible as a pre-merged subs list, but the merge allocates per clause and buys nothing here. `TODO(G17 stage 3a)`. |
| `solver.rs:5765`, `:5886`; `generate.rs:2030`; `conformance.rs`; `entailment.rs`; `resolve.rs`; `conformance_completeness.rs` | **no** | none of these produce a `TyVar` at all |

Three adoptions. That is the honest answer to "which can adopt it without
changing behaviour" — the rest are fixes wearing a refactor's clothes.

---

## 6. The 16 raw `AstWhereClause` walkers — **none are forced to change**

`WhereConstraint` / `kestrel_ast_builder::WhereClause` is the **AST** type.
D7 changes `kestrel_type_infer::resolve::WhereClause`, a *different type in a
different crate*. Nothing in this plan touches the AST vocabulary, so every
raw walker compiles unchanged. Verified by
`grep -rn "WhereConstraint" --include="*.rs" lib/`:

| crate | sites |
| --- | --- |
| `kestrel-semantics` | `lib.rs:309/323/347` (copy semantics), `lib.rs:452`, `staticness.rs:163` |
| `kestrel-mir-lower` | `items/function_sig.rs:346/366/380` |
| `kestrel-analyze` | `body/move_tracking.rs:1970`, `decl/generics.rs:500-506`, `compilation/conformance_completeness.rs:648`, `compilation/constraint_cycles.rs:111` |
| `kestrel-name-res` | `resolve_type.rs:439/478/589/649`, `resolve_value.rs:571` |
| `kestrel-doc` | `signature.rs:340/380/531` |
| `kestrel-ast-builder` | `builders/function.rs:277/286` (constructs), `builders/helpers.rs:317/367/387` |
| `kestrel-type-infer` | `resolve.rs:2235` (`WorldResolver::gather_bounds_from_where_clause`) — inside the crate, but on the *AST* type, so also unchanged |

**One caveat, not a forced change.** `resolve_type.rs:439` and `:478`
(`search_bounds_for_assoc`, `search_inherited_assoc_bounds`) are the walkers
that make `T.Iter.Item` resolve at all — `search_bounds_for_assoc` requires
`segments.len() == 1` for the subject, `search_inherited_assoc_bounds`
requires `>= 2`. The chain walk in §3 depends on both. Do not refactor them in
this change; they are the thing being relied on.

So `WhereClausesOf` remains bypassed by all 16. That is a real finding
(`problem.md`'s "Also bypassing `WhereClausesOf`" list) and stays open — D7
neither fixes nor worsens it. **Size impact: zero.**

---

## 7. Migration sequence

Three commits. Each builds; each is reviewable in isolation; each is scoped to
explicit paths (never a bare `git commit` — shared branch, `decisions.md`
*Coordination*).

### Commit 1 — `refactor(type-infer): D7 — one WhereSubject for every bound subject`

Paths: `lib/kestrel-type-infer/src/{resolve,where_clauses,conformance,entailment,generate,lib,solver}.rs`,
`lib/kestrel-analyze/src/compilation/conformance_completeness.rs`.

1. Add `WhereSubject` + `as_param` + `map_root` to `resolve.rs`.
2. `WhereClause::Bound { subject, protocol, protocol_type_args }`; delete
   `ProjectionBound`. Leave `TypeEquality` / `DirectEquality` alone.
3. `where_clauses.rs`: `resolve_projection_subject` → `resolve_bound_subject`,
   **keeping both guards** (`segments.len() != 2`, base is `TypeParameter`) so
   output is value-identical to today. Update the two `already_bound`
   `matches!` and the two implicit-bound pushes.
4. Apply §4.B (6 sites), §4.C (2), §4.D (13), §4.E (8 test constructions).
5. `TODO(G17 stage 3a)` at `ctx.rs:178`.

**Property: no `WhereClause` value differs from today.** Provable by
inspection — the only construction path is `resolve_bound_subject`, whose
guards are unchanged.

**Suite: must be bit-identical.** This is the commit the "reviews as no
behaviour change" claim attaches to.

### Commit 2 — `feat(type-infer): D7 — where-clause subjects nest to arbitrary depth`

Paths: `lib/kestrel-name-res/src/resolve_type.rs`,
`lib/kestrel-type-infer/src/where_clauses.rs`, plus the new test file.

1. `resolve_type_path_chain` + `TypePathChain` in name-res;
   `ResolveTypePath::execute` delegates.
2. `resolve_bound_subject` drops the `segments.len() != 2` guard and folds the
   chain. Keeps the `TypeParameter`-root guard and keeps `self_rooted` paths
   collapsing (§2.2, §3).
3. `lower_subject` gains nothing — it was already recursive.

**Behaviour delta — fully enumerated.** Exactly **two** files in the tree
spell a depth ≥ 3 where-clause subject, both `// test: diagnostics` with **no
`// ERROR:` annotations** (i.e. expecting zero diagnostics):

```
declarations/associated_types/where_clause_on_nested_associated_type.ks:12
    func findIn[C](c: C) where C: Container, C.Iter.Item: Equatable { }
declarations/associated_types/nested_associated_type_equality.ks:11
    func intContainer[C](c: C) where C: Container, C.Iter.Item = lang.i64 { }
```

The second is an `Equality` and is **not affected** by commit 2 (§2.1 defers
`Equality`; `extract_associated_type_path` keeps its own `len() != 2` bail at
`:393`). So the delta is **one file**: `C.Iter.Item: Equatable` stops being
emitted as a bare `Bound { param: Item }` and becomes a skipped depth-3
projection. Since the emission can only *add* diagnostics and the file expects
none, the test stays green. Verified by grep:

```
grep -rn "where[^;{]*[A-Za-z_][A-Za-z0-9_]*\.[A-Za-z0-9_]*\.[A-Za-z0-9_]*[[:space:]]*[:=]" \
  --include="*.ks" lib/kestrel-test-suite/testdata lang/
```
→ 2 hits, both above. **Zero in `lang/std`.** The three shipped stdlib
projection bounds (`adapters.ks:397`, `:666`, `:866`) are all depth-2 and
already take the projection path.

`Self.Item: Equatable`
(`declarations/extensions/protocol_extension_mixed_self_constraints.ks:15`)
is **unchanged** by commit 2 because `self_rooted` chains still collapse.

**Suite: expected identical.** If anything else moves, stop — the enumeration
was wrong and that is a finding.

### Commit 3 — `docs(fragility): record D7 as landed; correct the site counts`

`decisions.md` only: mark D7 implemented-for-step-2, fix "seven" → eight/six
(§0), record the `resolve_first_segment` anchor correction, and state
explicitly that `leak5.ks` still miscompiles.

**Optional commit 2b** (ask first): file the G17 regression pair as
expected-to-fail testdata per `verifying-claims.md` Tier 1
(`assoc_projection_bound_cross_receiver.ks` + its minus-one-clause control,
asserting *rejection*, never the miscompiled output). This **adds** failing
tests, so it changes the suite tally. It belongs with stage 3a unless the
maintainer wants the record filed now.

---

## 8. Test plan

### 8.1 The differential — the primary evidence

Baseline full run is already in flight. After commit 1 and again after commit
2, run the full suite via the `/triage` skill (never `cargo test -p
kestrel-test-suite`) and diff pass/fail **per test name**, not just totals.
Totals hide a swap. Read the results before reporting — a started run with
ignored output is not a test run (root `CLAUDE.md`).

Expected: identical after commit 1; identical after commit 2, with the one
enumerated file in §7 as the only candidate for movement.

### 8.2 Rust unit tests — put them in `tests/`, not `#[cfg(test)]`

`where_clauses.rs` has no `#[cfg(test)]` today, but a hand-built `World` is
the wrong harness for it: resolving `C.Iter.Item` needs `ResolveName`,
`VisibleChildrenByName`, `resolve_type_param_assoc`, and a real
`AstWhereClause` on the function — reproducing that by `world.spawn()` (the
`entailment.rs:146-172` style) is ~150 lines of scaffolding that will drift.

**`kestrel-type-infer` already has the right harness**:
`lib/kestrel-type-infer/tests/assoc_where_repro.rs:11-27` and
`tests/integration.rs:33` both do lex → parse → `build_declarations` →
`seed_lang_module` from a source string, and the crate's `[dev-dependencies]`
already carry `kestrel-lexer` / `kestrel-parser` / `kestrel-syntax-tree`.

New file **`lib/kestrel-type-infer/tests/where_subject.rs`**, reusing that
`build_from_source` helper, querying `WhereClausesOf` directly and asserting
on subject shape:

| test | source | asserts | lands in |
| --- | --- | --- | --- |
| `bare_param_subject_is_param` | `func f[T](x: T) where T: P` | subject == `Param(T)` | commit 1 |
| `depth_two_projection_keeps_its_base` | `func f[C](c: C) where C: Container, C.Iter: Iterator` | `Projection { base: Param(C), assoc: Iter }` | commit 1 |
| `implicit_copyable_bound_uses_param_subject` | `func f[T](x: T)` | injected bound's subject is `Param(T)` — guards §4.A rows 3-4 | commit 1 |
| **`nested_projection_nests_to_depth_three`** | `where C: Container, C.Iter.Item: Equatable` | `Projection { base: Projection { base: Param(C), assoc: Iter }, assoc: Item }` — **the Q8 test**: it parses into depth 3 even though every consumer skips it | commit 2 |
| `self_subject_still_collapses` | `extend Iterator where Self: Comparable` | subject is `Param(<Iterator>)`, **not** `SelfType` — pins §2.2 so the deferral is deliberate and its removal is visible | commit 1 |

Note the harness caveat recorded at `assoc_where_repro.rs:75-84`: name
resolution from a *method* context is under-wired in this harness but works
from a *free-function* context. All four positive tests above use free
functions. The `extend` case reads the extension's own `WhereClausesOf`
without inferring a body, so it does not hit that limit.

`entailment.rs`'s 4 existing tests are updated mechanically (§4.E) and keep
their assertions.

### 8.3 What **not** to write

- **No testdata `.ks` diagnostics test anchored in stdlib.** The matcher
  filters `d.file_id == test_file_id` (`diagnostic_matcher.rs:191`) and
  discards `lang/std`-anchored diagnostics — finding **G23**. G14's own repro
  (`f_stdlib_iter.ks`) lands its error at `lang/std/iter/iterator.ks:857:37`
  and is therefore invisible to the suite. Do not file it as one.
- **No second verification harness.** `verifying-claims.md` §"The one thing
  not to build".
- **No test asserting `leak5.ks`'s printed value** — it is a heap address
  under ASLR. Any G17 test asserts *rejection*.
- **No weakened tests.** If a test moves, that is the finding.

---

## 9. Risks, ranked

| # | risk | likelihood | blast radius | mitigation |
| --- | --- | --- | --- | --- |
| **R1** | An implicit-widening site (§4.D) is missed, and a projection subject silently enters a body that never saw one. `conformance_completeness.rs:1811`'s in-place mutation is the worst case — it would start rewriting projection bases. | **high** if worked from the brief's list of six; low if worked from §4.D | silent semantic change inside a "no behaviour change" commit — destroys the property the whole sequencing exists to buy | Mechanical sweep: after the merge, `grep -rn "as_param()" ` must return **21** non-test sites and `grep -rn "\.subject"` must return no unguarded destructure. Reviewer checklist = §4.B/C/D tables. |
| **R2** | `Equality` collapse is pulled into this change (against §2.1) and `Item.Output` fails to resolve to an entity, dropping the stdlib `sum`/`product`/`Set +`/`Dictionary +` equalities. | medium *if attempted*, zero if deferred | stdlib type-checks differently; failure mode is a **dropped** clause, i.e. a wrong *accept*, which the suite may not catch | Defer (§2.1). If not deferred: before changing the builder, instrument `extract_associated_type_path` to log every `(base, assoc_name)` that fails full-path resolution, and run the stdlib build. Treat any hit as a blocker. |
| **R3** | `SelfType` gets constructed "because D7 says so", breaking `conformance.rs:363`, `resolve.rs:2079`, and `lib.rs:707`. | medium — the decision text invites it | `h_selfq_neg.ks` flips; `where Self: Q` stops gating anywhere | §2.2 + the `self_subject_still_collapses` unit test, which fails loudly the moment someone flips it. Escalate to the maintainer (§2.2 question) rather than improvising. |
| **R4** | Refactoring `ResolveTypePath::execute` into a chain-returning walk changes an answer — the `Self` helpers each have their own loop and their own early-return semantics. | medium | name resolution is upstream of everything; a regression here is broad and hard to attribute | `execute` must remain a one-line delegate returning `.resolution` — no logic in the wrapper. Land commit 2 separately from commit 1 so a suite diff attributes cleanly. Existing `lib/kestrel-name-res/tests/integration.rs` covers the query. |
| **R5** | Another agent lands in `conformance.rs:313-373` or `conformance_completeness.rs:1550-1660` concurrently — both are named danger zones in `decisions.md` *Coordination*, and this change touches both. | medium — shared branch, multiple agents | merge conflicts in the two files where a botched merge is a silent semantic change | Claim both in `decisions.md` *Danger zone* before the first edit. Scope commits to explicit paths. Land commit 1 in one sitting; it is mechanical and should not straddle a suite run. |
| **R6** | `where_clause_assoc_subs` gets re-keyed by `WhereSubject` "while we're in here". | medium — it is the obvious next move and `lower_subject` makes it easy | **fixes** the G17 miscompile inside the behaviour-preserving commit, i.e. changes behaviour in the change that promised not to | `TODO(G17 stage 3a)` at `ctx.rs:178`. Explicitly out of scope (§2.3), and D7's own *Scope limit* says so. |
| **R7** | Commit 2's depth delta is bigger than the two-file enumeration — e.g. a package outside `lang/std` and `testdata` (samples, `lang/` non-std, external packages) spells a depth-3 subject. | low | a build that used to pass stops | The grep in §7 covered `lang/` *and* `testdata`. Before landing commit 2, re-run it over `samples/`, `external/`, and any `.ks` under `bin/`. Cheap; do it. |
| **R8** | `Box<WhereSubject>` shows up in a profile (`WhereClausesOf` is queried on hot paths — `solver.rs:3524` runs per `Def` reference). | low | compile-time regression | Depth is ≤ 3 in the entire corpus, so at most one `Box` per projection clause and zero for the common `Param` case. D7 already names interning to a `SubjectId` as the escape hatch. Measure before optimizing. |

---

## 10. Could not determine — what I'd need

1. **Whether the maintainer accepts `SelfType` being defined-but-unconstructed
   in this change** (§2.2), or wants `SelfType { enclosing: Entity }` so the
   flip can happen now. This is the single decision that changes the shape of
   commit 1. I did not re-open D7; I am reporting that its decided shape and
   the hard constraint are in tension, and naming the two ways out.
2. **Whether `Equality` must collapse in this change** (§2.1). I recommend
   deferring and gave three reasons; if overruled, R2 becomes the top risk and
   the instrumentation described there is a prerequisite, not an option.
3. **Whether `Self.Item` should resolve through the synthetic `Self` type
   param.** `try_resolve_self_as_type_param` (`resolve_type.rs:224-261`) shows
   the AST builder mints a `NodeKind::TypeParameter` for `Self` in protocol
   extensions. If that entity is the right root, `Self.Item` could become
   `Projection { base: Param(<synthetic Self>), assoc: Item }` with no
   `SelfType` variant involved at all — which would be behaviour-preserving in
   a way bare `Self` is not. I did **not** verify that the synthetic param
   exists for every protocol extension, only that name-res looks for one.
   Verifying it needs a `kestrel dump`-level trace on
   `protocol_extension_mixed_self_constraints.ks`, which I did not run
   (a full-suite baseline is in flight and I stayed off the build).
4. **The current pass/fail status of the two depth-3 testdata files.** I read
   their annotations (both expect zero diagnostics) and reasoned that commit
   2's delta can only remove an emission, not add one — but I did not run
   `triage` on them, deliberately. Confirm with a targeted run before landing
   commit 2, not with the full suite.
