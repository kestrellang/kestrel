# Stage 3a — closing the G17 miscompile

> **Scope.** This is the plan for the *behavioural* half of G17, after commit 1
> (`6ece3ae3`, `WhereSubject`) and commit 2 (arbitrary-depth chains, in flight).
> It does not cover G14 (stage 3b) except where the two share a call site.
>
> **Status:** designed, not implemented. Every empirical claim below carries a
> provenance stamp per `docs/contributing/verifying-claims.md`.

## Preflight

```
pwd              /Users/dino/Documents/Projects/kestrel
git log -1       6ece3ae3 refactor(type-infer): D7 — one WhereSubject for every bound subject
git rev-parse    6ece3ae3e34e0ae3092fd0f34e692662dc728025
branch           arch/fixes   (parent checkout; no worktree)
build            cargo build --release --bin kestrel   → clean
```

Working tree at measurement time also carried another agent's **uncommitted**
commit-2 work in `where_clauses.rs`, `kestrel-name-res/src/{lib,resolve_type}.rs`.
`resolve_projection_subject` already folds an arbitrary-length chain
(`where_clauses.rs:322`) and already returns `None` for `Self`-rooted chains
(`:309`, D8). Nothing measured below depends on those files.

## What was measured, fresh, at `6ece3ae3`

### The corpus still behaves as `problem.md` describes

[verified @ `6ece3ae3`, 2026-08-22 — built and run, not read]

| repro | result at HEAD |
| --- | --- |
| `leak5.ks` | **clean build, exit 0, `result=int:4332925600`** — heap pointer, miscompile intact |
| `control2.ks` | correct `E100 … B.Item !: Show` at `:24:5` |
| `distinct_samename.ks` | **clean build, exit 0, prints `result=i`** — `Int64.show()` ran on a `String`. Also a miscompile, not merely a wrong accept |
| `distinct.ks` | correct `E100 … B.ItemB !: Show` at `:27:5` |
| `structcase.ks` | frontend accepts; post-mono failure |
| `structcase_nobound.ks` | correct `E100 … B.Item !: Show` at `:28:34` |
| `callsite.ks` | frontend accepts; post-mono failure. *(New datapoint — the call-site obligation `A.Item: Show` is never emitted.)* |
| `selfproj.ks` | frontend accepts; post-mono failure. *(`extend Producer where Self.Item: Show`.)* |

Correction to the `problem.md` table: `distinct_samename` is filed as a "wrong
accept". It is a **miscompile**, same class as `leak5`. Its output happens to be
a constant (`"i"`) rather than a pointer, which is why nobody noticed.

### Which read site leaks — instrumentation spike

**Disclosure.** I added `ktrace!("g17", …)` at the five `where_clause_assoc_subs`
read sites in `solver.rs`, built, ran, and **reverted**. `solver.rs` is
byte-identical to its pre-spike state (`shasum 8fa39bc26f9d79d310b4c13ff3329d06131b0511`
before and after; `git status` shows it unmodified). No production code was left
behind. The trace also computed, for each hit, whether the recorded TyVar is an
`AssocProjection` whose base resolves to the same TyVar as the query's base —
`MATCH` / `MISMATCH` / `NOBASE` (recorded value is not a projection, so the proxy
cannot tell).

Per-compilation totals, `leak5` vs. its correctly-rejecting control `control2`:

```
                                            control2   leak5   distinct_samename
2836 NOBASE    Iterator.Item                     24      24        24
2836 MISMATCH  Iterator.Item                      3       3         3
2836 MISMATCH  Producer.Item        (test file)   —       —         1   ←
2822 NOBASE    (Iterator/Iterable.Item, Output)  25      25        25
6039 NOBASE    (Iterator.Item, *.Output)          4       4         4
6039 MATCH     (Iterator.Item, TargetIterator)    2       2         2
6039 MISMATCH  Iterator.Item                      1       1         1
6039 MISMATCH  Producer.Item        (test file)   —       1   ←     —
3914 (member)  Iterable.TargetIterator            1       1         1
5996                                               0       0         0
```

[verified @ `6ece3ae3`, spike, reverted]

Two facts fall straight out:

1. **`leak5` leaks at exactly one site: `solver.rs:6039`** — `lower_hir_ty_sub`'s
   `HirTy::AssocProjection` arm, the one with `let _ = base_tv;`. It is the only
   line that differs from the passing control.
2. **`distinct_samename` leaks at exactly one site: `solver.rs:2836`** — the
   cross-protocol `Name`-equality fallback. Also the only differing line.

Everything else in the table is stdlib background that is *identical in the
program that correctly rejects*. That is the blast-radius map, and it says the
danger is concentrated in the `Iterator`/`Iterable` bridge, not in the test files.

`solver.rs:5996` — the site I expected to be the culprit — **never fires** on any
of these programs.

### There is a second, independent base-free mechanism, and it is not in the vector

[inferred from source, corroborated by the `structcase` A/B pair @ `6ece3ae3`]

`structcase.ks` puts `A.Item: Show` on a *struct*. `lib.rs:705` skips
container-level projection clauses entirely, so **no push into
`where_clause_assoc_subs` ever happens** — yet `structcase` still accepts and
`structcase_nobound` (same file, clause deleted) rejects. The accept therefore
cannot come from the vector. It comes from:

```
solve_conforms (solver.rs:2115)
  → WorldResolver::conforms_to, TyKind::AssocProjection arm (resolve.rs:627)
     — discards `base`
  → collect_assoc_type_protocol_bounds(assoc)                (resolve.rs:2022)
  → gather_bounds_from_where_clause(alias_entity, owner)     (resolve.rs:2269)
     — walks the RAW AstWhereClause and matches
       `resolve_type_entity(subject) == alias_entity`
```

`resolve_type_entity` on the AST path `A.Item` runs `ResolveTypePath(["A","Item"])`
and returns `Found(Producer.Item)` — the **last** segment. So the clause
`where A.Item: Show` grants `Show` to the *entity* `Producer.Item`, and therefore
to `B.Item`, `C.Item`, and every other `_.Item` in scope. This is aliasing
mechanism (2) from `problem.md`, and it is live independently of the vector.

**Consequence for sequencing, stated plainly:** re-keying the vector makes
`b.produce()` type as `AssocProjection { base: B, assoc: Producer.Item }` instead
of `A.Item` — the type confusion is gone, the emitted code is correct — and then
mechanism (2) still grants `Show` to it. So **the re-key alone downgrades `leak5`
from a silent miscompile to a post-mono error; it does not make it reject.**
Conversely the base-aware conformance check alone does nothing, because
`b.produce()` still *is* `A.Item`, whose base genuinely matches the clause. Both
are required, in that order.

`distinct_samename` is different: its two `Item`s are distinct entities, so
mechanism (2) already refuses (this is why `distinct.ks` rejects). It needs only
the `2836` narrowing.

---

## 1. The key

### The question

`where_clause_assoc_subs: Vec<(Entity, TyVar)>` (`ctx.rs:186`). What replaces
`Entity`?

### The answer: `(base TyVar, assoc Entity)`, compared after union-find resolution

```rust
/// Which *projection* a cached TyVar stands for.
///
/// The base is a TyVar, not a `WhereSubject`, because this table is a memo over
/// the inference world: it exists so a body's own `T.Assoc` uses reuse the TyVar
/// the where clause already built. Six of its seven readers hold a TyVar and no
/// subject, and `lower_subject` is not invertible — by the time a reader asks,
/// the base may have unified with a concrete type that no `WhereSubject` names.
#[derive(Clone, Copy)]
pub(crate) struct AssocSubKey {
    /// `None` only for a genuinely baseless binding — `DirectEquality` on a
    /// TypeAlias entity (`where Item = Int64`), which names no receiver.
    pub base: Option<TyVar>,
    pub assoc: Entity,
}
```

with the field made private behind exactly two methods on `InferCtx`, so the
comparison rule lives in **one** place:

```rust
fn push_assoc_sub(&mut self, base: Option<TyVar>, assoc: Entity, tv: TyVar)
fn assoc_sub(&self, base: Option<TyVar>, assoc: Entity) -> Option<TyVar>
```

### Why not `WhereSubject`

`WhereSubject` is the right key one level up, at the *clause*. Using it again
here would mean two keys for one fact — the duplication this audit exists to
remove. `lower_subject` is the bridge, and it is applied **once, at push**. Three
concrete reasons the read side cannot use it:

- Six of the seven reads (`generate.rs:2365`, `solver.rs:2822`, `:2836`, `:3914`,
  `:5996`, `:6039`) hold only a TyVar. Reconstructing a declaration-world subject
  from it is the inverse of `lower_subject`, which does not exist and cannot: a
  base that has unified with `Struct{StrSrc}` is not any `WhereSubject`.
- The equivalence we actually want *is* TyVar unification. If two bases unify,
  their projections genuinely are the same type and sharing the TyVar is correct.
  `WhereSubject` equality would under-approximate that and force a spurious miss.
- `WhereSubject` says nothing about which of several *instantiations* of the same
  generic body a TyVar belongs to. The vector is per-`InferCtx`, so this does not
  bite today, but keying on it would encode the assumption.

### What happens when the base unifies after the push — the crux

**Store the raw TyVar at push. Resolve both sides at lookup. Never compare raw
indices. Never cache the resolved value.**

- *Why not resolve at push?* `DirectEquality` writes `TySlot::Redirect` straight
  into a param slot (`lib.rs:519`, `lib.rs:822`), and ordinary unification
  redirects constantly. A canonical snapshot taken at push goes stale the moment
  the base is unified, and a stale canonical produces a **false miss** — the
  over-rejection failure mode.
- *Why resolve at lookup is safe:* union-find is monotone. Two TyVars equal at
  time *t* stay equal for every *t' > t*. So resolve-at-lookup can only become
  *more* permissive as inference proceeds, never less. That is the safe direction
  for a memo: it never invents a distinction that unification has already erased.
  The mirror case — a base unified *after* the entry was pushed — is precisely the
  case where sharing the TyVar is *correct*, and resolve-at-lookup gets it right.
- *Why raw-index comparison is wrong:* the same subject legitimately has two
  indices after any redirect. `witness_protocol_args` already learned this and
  carries a resolved-canonical rescan at `solver.rs:2766-2781` and again at
  `:2185-2197`. Follow that precedent; do not invent a second convention.

### The `None` rules

- A **baseless entry** (`None` in the table) matches only a baseless query.
  Widening it to "matches anything" reintroduces the bug.
- A **baseless query** — genuinely only `generate.rs:2344`, see §2 — resolves by
  *unambiguity*: if exactly one entry has that `assoc`, use it; if two or more,
  bail to the general path. This is not base-blindness; it is the same
  "ambiguous — bail rather than pick arbitrarily" rule `witness_protocol_args`
  already uses two screens up. It preserves today's answer in every
  single-subject program while turning the two-subject collision — the entire bug
  — into a fall-through instead of a `find()`-returns-first.

### Rejected alternative: intern `assoc_projection` on `(resolved base, assoc)`

This would delete the memo entirely — `ctx.assoc_projection(base, assoc)` would
*be* the lookup. Rejected for now because interning keys on a base that may
resolve later, so `X.Item` created while `X` is unresolved and `Y.Item` created
after `X ≡ Y` get two TyVars for one projection; `generate.rs:2088` already warns
that caching here "would merge projection TyVars that are distinct today". Revisit
once the audit (commit 1) shows how often the two disagree — if the answer is
"never", this is the real simplification and the vector goes away.

---

## 2. Every push and every read

### 7 pushes — all in `lib.rs`, all have a base available

| # | symbol (`lib.rs`) | line | base to record |
| --- | --- | --- | --- |
| P1 | `emit_method_projection_bound_constraint` | `:469` | the base `lower_subject` walked to build `proj_tv`. **Requires `lower_subject` to return the base as well as the leaf** — see below |
| P2 | `emit_method_type_equality_constraint` | `:501` | `subject_tv` (`ctx.param(param)`) |
| P3 | `emit_method_direct_equality_constraint` | `:525` | `None` — `where Item = X` on a TypeAlias entity names no receiver. The one legitimate baseless entry |
| P4 | `emit_container_where_clauses`, `TypeEquality` arm | `:803` | `subject_tv` from `get_or_create_subject_tv` |
| P5 | `emit_protocol_assoc_type_where_clauses`, pre-registration loop | `:883` | `subject_tv` (the tv the sibling `associated` constraint was emitted on) |
| P6 | same fn, per-alias `alias_tv` creation | `:917` | `subject_tv` |
| P7 | same fn, inner `TypeEquality` on an alias's own clause | `:972` | `alias_tv` |

P1 needs a small signature change: `lower_subject` currently returns only the
leaf TyVar. Return `(base_tv, leaf_tv)` — or, better, have it return the whole
chain and let the caller take the last two — so the emitter records the right
base for a depth-*n* subject without re-walking.

### 6 reads (+1 name fallback) — what each has, and what a miss should do

Every one of these is a **memo shortcut**, never an obligation. So the correct
behaviour on a miss is always *fall through to the general path*, which every
site already has and which is strictly **more** informative than the shortcut —
a based projection or an `Associated` constraint instead of a bare alias TyVar.
**A miss cannot itself cause over-rejection.**

| # | site | query base available? | fall-through on a miss |
| --- | --- | --- | --- |
| R1 | `generate.rs:2344` — `lower_hir_ty_with_subs`, `AliasUse{args:[]}` | **no** — the only genuinely baseless read | `param_tyvars`, then `ctx.type_alias(entity, [])`. Apply the *unambiguity* rule |
| R2 | `generate.rs:2365` — same fn, `AssocProjection` (`let _ = base_tv;`) | **yes**, `base_tv` | `ctx.project_associated(base_tv, assoc, span)` |
| R3 | `solver.rs:2822` — `solve_associated`, `AliasUse` | **yes**, `container` | the existing `param_tyvars` → `assoc_projection` → `lower_hir_ty_sub` chain |
| R4 | `solver.rs:2836` — the **`Name`-equality fallback** | **yes**, `container` | §3 |
| R5 | `solver.rs:3914` — `solve_member` receiver substitution | **partly** — `TyKind::AssocProjection{base,..}` has one; `TyKind::TypeAlias{entity}` does not | don't substitute the receiver; let `resolve_member` do its protocol-bound search. This is already the documented behaviour when `should_substitute` is false |
| R6 | `solver.rs:5996` — `lower_hir_ty_sub`, `AliasUse{args:[]}` | **yes, but ignored today** — `recv_tv` *is* the base of a bare `Item` in a member signature | the `parent_is_protocol` `Associated`-constraint path, then `ctx.type_alias` |
| R7 | `solver.rs:6039` — `lower_hir_ty_sub`, `AssocProjection` (`let _ = base_tv;`) | **yes**, `base_tv` | `ctx.project_associated(base_tv, assoc, span)` |

R6 deserves the emphasis: it reads as baseless but is not. A bare `Item` in
`Producer.produce()`'s signature, lowered for receiver `B`, *means* `B.Item`, and
`recv_tv` is sitting in the same function. Passing it as the query base is the
whole fix at that site. Only R1 is truly baseless.

**Where over-rejection actually comes from**, then, is not the miss but the
downstream: an `Associated` constraint that used to be short-circuited now gets
emitted, deferred, and — if nothing resolves it — never solved. Watch the suite
for *"could not infer type"* and unresolved-TyVar diagnostics, **not** for `E100`
storms. That is the signal the audit in commit 1 must be built to detect.

---

> ## ⚠ §3 IS REFUTED BY MEASUREMENT — read this before acting on it
>
> [measured @ `a8fa672c`, C1 audit sweep over all 3655 testdata files]
>
> §3 assumes the legitimate `Iterable`/`Iterator` bridge is a **same-receiver**
> cross-protocol name match, and concludes that requiring equal bases "leaves
> the bridge's same-receiver hits alone". **There are no same-receiver hits.**
>
> The 27×/compilation figure is confirmed exactly. But every one of the 27 is
> `assoc=Iterator.Item` answered from an entry filed under **`Iterable.Item`,
> across five distinct receivers — zero same-receiver.** So **C6 as specified
> is not a narrowing of the fallback, it is a deletion of all 27 uses.**
>
> The bridge is justified by the `where TargetIterator.Item = Item` **equality
> clause**, not by base identity. C6 therefore depends on the equality path
> working first — the very fallback §3 flags as unimplemented. Re-plan C6
> against the equality clause before touching `solver.rs:2836`.

## 3. The `solver.rs:2836` name fallback

### What it is serving

The in-source comment says "different protocols can define the same associated
type (e.g. `Iterator.Item` vs `Iterable.Item`)". That is exactly right, and the
measurement says it is **heavily** load-bearing:

> **27 fires per compilation**, on every one of the eight repros including the
> ones that compile correctly, all on `Iterator.Item` (24 `NOBASE`, 3 `MISMATCH`).
> [verified @ `6ece3ae3`]

The shape it bridges is `Iterable`'s
`type TargetIterator: Iterator where TargetIterator.Item = Item`. `lib.rs:972`
pushes that clause under `find_assoc_type_in_bounds(TargetIterator, "Item")` =
**`Iterator.Item`**, while `lib.rs:883` pre-registers `Iterable.Item`. A later
`solve_associated` asks for `Iterator.Item` on a container whose entry was filed
under `Iterable.Item`, misses the entity match, and the name fallback rescues it.

### Is the case still served after re-keying?

Partly, and only if the equality is allowed to do its own job. The constraint
that expresses the bridge **already exists** — `lib.rs:954-967` emits
`Associated(alias_tv, "Item", fresh)` + `Equal(fresh, rhs_tv)`. The fallback is a
short-circuit around a constraint the solver could discharge itself. So the
principled narrowing is:

**Keep the name fallback, but require the resolved bases to match.** A name match
across two protocols on the *same* receiver is the legitimate `Iterable`/`Iterator`
bridge. A name match across two *different* receivers is `distinct_samename`.
That single added condition rejects `distinct_samename` and leaves the bridge's
same-receiver hits alone.

### What breaks if it simply goes

At minimum the 3 `MISMATCH` fires per compilation lose their shortcut and fall to
the deferred-`Associated` path; the 24 `NOBASE` fires are unclassified by the
proxy and could be anything. Deleting it outright is the single riskiest edit in
this plan and **must not be attempted before the commit-1 audit reports real
numbers.** Narrowing is cheap; deletion is a separate, later, measured decision.

---

## 4. Ordering vs. the `conforms_to` move

D7's other half: move the `TyKind::AssocProjection` arm out of
`WorldResolver::conforms_to` (`resolve.rs:627`) into `solve_conforms`
(`solver.rs:2115`), where an `InferCtx` exists and `ctx.resolve(base)` works.

**Are they independent?** In code, yes — different files, no shared symbol.
In *effect*, no, and the direction matters:

| order | result for `leak5` |
| --- | --- |
| conforms move first | **no change.** `b.produce()` still types as `A.Item`; the base check finds base `A`, the clause is about `A.Item`, it matches, permit. Still miscompiles |
| re-key first | `b.produce()` types as `B.Item` — the emitted code is now correct — but mechanism (2) still grants `Show` base-free, so it builds. **Downgrades the miscompile to a post-mono error** |
| re-key, then conforms move | **frontend `E100`** |

So: **re-key → conforms move.** The re-key is the prerequisite; the move is the
commit that produces the diagnostic. Doing the move first is not harmful, just
inert, and it would waste the review — its test could not distinguish it from a
no-op.

Blast radius of the move itself is small and countable: `conforms_to` has
**6 callers**, all inside `kestrel-type-infer` — `solver.rs:1460`, `:1948`,
`:2061`, `:2115`, `unify.rs:527`, `resolve.rs:762`. Four already hold a
`&mut InferCtx`. [verified @ `6ece3ae3`]

Leave `WorldResolver::conforms_to`'s `AssocProjection` arm answering the
*declares-only* question (bounds on the alias, base ignored) and make
`solve_conforms` intersect it with a base check, rather than deleting the arm —
the two remaining non-solver callers keep working and the change reads as a
narrowing, not a relocation.

---

## 5. The skip sites, dependency-ordered

Current lines at `6ece3ae3` (commit 1 moved them and left `TODO(G17 stage 3a)`):

| site | symbol | classification | depends on |
| --- | --- | --- | --- |
| `lib.rs:705` | `emit_container_where_clauses`, `Bound` arm | **live bug** — only emitter for container-level clauses (`structcase.ks`) | **re-key** (it adds an 8th push; landing it first widens the leak from post-mono error to `leak5`-class miscompile) |
| `generate.rs:1941` | `emit_where_clause_constraints_with_subs` | **live bug** — call-site / type-formation | projection policy (below) |
| `solver.rs:3534` | direct-`Def` call-site obligation (`callsite.ks`) | **live bug** | projection policy |
| `solver.rs:4435` | member/method call-site obligation | **live bug** | projection policy; also needs `SubjectRoot` to model its two-stage `resolution.type_params` → `subs` lookup, which the in-source comment already calls out as "a fix, not a refactor" |
| `lib.rs:930` | protocol assoc-type clauses | inert feature | re-key |
| `solver.rs:5786` | `emit_type_alias_where_clauses` | inert feature | re-key |

Three more `TODO(G17 stage 3a)` markers are in scope for 3a but are not in the
"six": `solver.rs:5911` (`emit_static_wellformedness`, projection subjects skipped
for the `Static` bound), `resolve.rs:1750` (`find_protocol_type_args` — an
`as_param()` filter that drops parameterized projection bounds like
`where T.Item: P[Int16]`), and `entailment.rs:40` (`None => false`, now trivially
fixable since `WhereSubject` derives `Eq`). All three are independent and low
risk. `conformance.rs:353` and the three in
`conformance_completeness.rs` (`:1642`, `:1676`, `:1824`) belong to stage 3b.

### The projection policy — a design point the skip sites force

`lower_subject`'s `Projection` arm uses `ctx.assoc_projection`, which allocates a
TyVar **without** emitting an `Associated` constraint. That is right for a
declaration-scope bound, which must survive as an opaque projection. It is wrong
for a call-site obligation: at `good(StrSrc(…))` the subject lowers to
`assoc_projection(StrSrc_tv, Producer.Item)`, which never reduces to `String`, so
`solve_conforms` judges an unreduced projection instead of the concrete type it
denotes. The three call-site skip sites therefore need `ctx.project_associated`
(constraint-emitting) rather than `ctx.assoc_projection`.

Add that as a second policy parameter alongside `SubjectRoot` — `Opaque` vs
`Reduce` — rather than duplicating `lower_subject`. Note `ctx.rs:530-533`
explicitly warns against re-adding a raw `assoc_projection` variant for exactly
this misuse; a named policy is the compliant way to have both.

---

## 6. Tests

Promote the `scratchpad/g17/` A/B pairs into
`lib/kestrel-test-suite/testdata/types/generics/`, beside the existing
`assoc_projection_bound_witness.ks` / `_extension.ks`. All of them go in as
`// test: diagnostics`.

**Never assert `leak5`'s printed value** — it is a heap address under ASLR
(`4332925600` observed here; three other values recorded previously for the same
binary). A `diagnostics` test never runs the binary, which removes the
temptation structurally.

| new file | from | mode | annotation | today |
| --- | --- | --- | --- | --- |
| `assoc_projection_bound_cross_receiver.ks` | `leak5` | diagnostics | `// ERROR: !: Show` on the `needsShow(b.produce())` line | **fails** (no diagnostic) — the record of the bug |
| `assoc_projection_bound_cross_receiver_control.ks` | `control2` | diagnostics | same annotation | passes; must stay green |
| `assoc_projection_bound_same_name_distinct_protocols.ks` | `distinct_samename` | diagnostics | `// ERROR: !: Show` | **fails** |
| `assoc_projection_bound_distinct_names_control.ks` | `distinct` | diagnostics | `// ERROR: !: Show` | passes; must stay green |
| `assoc_projection_bound_on_container.ks` | `structcase` | diagnostics | `// ERROR: !: Show` on `go()`'s body | **fails** (post-mono error, invisible to a diagnostics test) |
| `assoc_projection_bound_on_container_control.ks` | `structcase_nobound` | diagnostics | same | passes; must stay green |
| `assoc_projection_bound_call_site.ks` | `callsite` | diagnostics | `// ERROR: !: Show` at the `good(StrSrc(…))` call | **fails** |
| `assoc_projection_bound_self_subject.ks` | `selfproj` | diagnostics | `// ERROR: !: Show` at `.render()` | **fails** — the D8 `SelfType` flip's test |

Each file keeps its `// G17 …` comment naming the audit ID and the mechanism, per
`verifying-claims.md`'s "name the file so the reason is obvious".

Notes on what the harness can and cannot see:

- **`// ERROR:` matching is a case-insensitive substring of the diagnostic
  message and its labels** (`diagnostic_matcher.rs:255-269`); `!: Copyable` is
  already used this way across `memory_model/generic_copyability/`. So `!: Show`
  is a valid annotation.
- **G23 does not block any of these.** All eight anchor their diagnostic in the
  test file (`control2.ks:24:5`, `distinct.ks:27:5`, `structcase_nobound.ks:28:34`
  — all verified), so the `d.file_id == test_file_id` filter at
  `diagnostic_matcher.rs:191` keeps them. G23 *would* block G14's
  `f_stdlib_iter.ks`, which anchors at `lang/std/iter/iterator.ks:857:37`. Do not
  promote that one until G23 is addressed.
- **A `diagnostics` test does not monomorphize.** `all_diagnostics`
  (`compiler.rs:140-155`) runs generic MIR lowering only when the front end is
  clean, and collects only coded diagnostics. Post-mono verification never runs.
  So `structcase` / `callsite` / `selfproj` cannot record their *current* symptom
  — which is fine and correct: the annotation records the behaviour the compiler
  should have, and it flips to passing on the day it does.
- There is no execution-mode variant worth writing. The only observable of the
  miscompile is a value we are forbidden from asserting.

---

## 7. Blast radius, and how to measure it before committing

### The static picture

[verified @ `6ece3ae3`] Of 3655 `.ks` files in `testdata/`:

| shape | files |
| --- | --- |
| projection **bound** in a where clause (`X.Y: P`) | 13 |
| projection **equality** in a where clause (`X.Y = T`) | 17 |
| any `where` clause | 367 |
| declares an associated type | 223 |

That undercounts badly, and the spike shows why: the stdlib's
`Iterable`/`Iterator` bridge fires **~50 reads and ~27 name-fallback hits per
compilation of a nine-line program**. Every file that touches an iterator — 226
use `for` alone — walks the same code. The honest statement is that the re-key's
*potential* radius is the whole suite, and its *actual* radius is unknown until
the `NOBASE` fires are classified.

### Detection-only first — commit 1

Follow `KESTREL_AUDIT_DUP` (`kestrel-mir/src/mono/audit.rs`; env-gated,
`KESTREL_AUDIT_FILTER` substring narrowing, inert when unset). Add
`KESTREL_AUDIT_SUBJECT` to `kestrel-type-infer`:

- widen the vector to `(AssocSubKey, TyVar)` and record the base at all 7 pushes;
- keep **every read exactly as base-blind as it is today** — so the commit is
  provably behaviour-preserving, and the compiler enforces it (no reader may
  touch `key.base`);
- under the env var, each read logs the verdict a strict key *would* have given:
  `MATCH` / `MISMATCH` / `MISS` (no entry at all) / `AMBIGUOUS` (baseless query,
  >1 candidate), tagged with the read site and the assoc's `Protocol.Name`.

Then run the suite under it and get the real distribution. That number replaces
every "expected delta" guess below. It is much cheaper than discovering the
answer in a red suite, and it is the same measurement D5 recommends for G14.

If `triage` cannot set an env var for a run, take the measurement as a one-off
sweep with the release CLI over `testdata/` — that is a *measurement*, not a
second harness, and `verifying-claims.md`'s prohibition is on the latter.

---

## 8. Commit-by-commit

Every commit is scoped to explicit paths (shared branch; never a bare
`git commit`). Suite runs go through `/triage` only.

### C1 — record the base; env-gated audit; no behaviour change

**Changes** `ctx.rs` (`AssocSubKey`, `push_assoc_sub`, `assoc_sub`, field made
private), the 7 pushes in `lib.rs`, `lower_subject` in `generate.rs` to return the
base, the 7 reads rewritten to call `assoc_sub(None-ish, assoc)` with today's
base-blind semantics plus the audit log.
**Test** the 8 promoted testdata files, landed here so their pre-fix failures are
on record from the start (4 pass, 4 fail).
**Expected delta** zero, other than the 4 new expected-to-fail files.
**Risk** low. Mechanical. The one hazard is accidentally tightening a read —
guard against it by keeping `key.base` unreadable outside `ctx.rs` in this commit.
**Output** the number that sizes C3 and C6.

### C2 — `solver.rs:5996` takes `recv_tv` as its base

**Changes** one read (R6). Split out because it is the only read whose base has to
be *identified* rather than passed through, and it never fires on the repro
corpus — so it is a pure-measurement commit against the suite.
**Test** none new; it is covered by the audit and the suite.
**Expected delta** 0.
**Risk** low-medium. If it moves anything, it moves iterator-heavy files.

### C3 — the re-key: reads compare resolved bases

**Changes** all reads switch to the strict rule (`None` matches `None`; baseless
query = unambiguity rule); `solver.rs:6039` and `generate.rs:2365` lose their
`let _ = base_tv;`. `solver.rs:2836` is **not** touched here.
**Test** `assoc_projection_bound_cross_receiver.ks` — still failing, but for a new
reason: `leak5` stops miscompiling and becomes a post-mono error. Add an
execution-mode sibling only if a non-ASLR observable can be found; otherwise
record the change in the commit message and the audit doc.
**Expected delta** the C1 audit's `MISMATCH` + `MISS` count. From the spike, at
least 1 fire/compilation at `6039` on `Iterator.Item` changes answer.
**Risk** **HIGH** — this is the commit that can produce deferred, never-solved
`Associated` constraints. Failure signature is *"could not infer type"*, not
`E100`.
~~**This is the commit that stops `leak5` emitting wrong code.**~~

> ### ⚠ C3'S HEADLINE CLAIM IS REFUTED — R3 falls through to R4
>
> [measured @ `6fb52dcb` + C3, built and run]
>
> C3 landed exactly as specified above and **`leak5` still miscompiles** —
> clean build, `result=int:4315148960`. The re-key is not sufficient, because
> the two "isolations confirmed at exactly one read each" finding
> (`leak5` → R7, `distinct_samename` → R4) treats the read sites as
> independent. **R3 and R4 are not independent: they are the two arms of one
> `else if` chain at `solver.rs:2818-2850`.**
>
> The audit at C3 shows the cascade precisely — `leak5 \ control2` is now:
>
> ```
> MISS site=solver:lower_hir_ty_sub:AssocProjection assoc=Producer.Item base=2 blind=3 strict=-1  ← R7 fixed
> MISS site=solver:solve_associated                 assoc=Producer.Item base=2 blind=3 strict=-1  ← R3 fixed
> MISS site=solver:solve_associated:name-fallback   assoc=Producer.Item base=2 blind=3 strict=-1  ← R4 STILL ANSWERS 3
> ```
>
> R7 and R3 both correctly refuse the entry filed under base `A` (`@1`) for a
> query on base `B` (`@2`). R3's refusal then **enters the `else if`**, and R4 —
> deliberately left base-blind, per the §3 refutation — matches the *same
> entity* `Producer.Item` by name and hands back the very TyVar R3 just
> rejected. Net answer unchanged; the miscompile survives.
>
> **This is not C6.** The fix is not to make R4 base-aware (which would delete
> the `Iterable`/`Iterator` bridge — §3 stands). It is that R4 should never
> consider a candidate whose assoc entity **equals** the query's:
>
> ```rust
> &|e| e != assoc && self.query_ctx.get::<Name>(e) == want,
> ```
>
> That is R4's own documented contract — *"**different protocols** can define
> the same associated type (e.g. `Iterator.Item` vs `Iterable.Item`)"*. Same
> entity is by definition not a different protocol, and it is R3's job.
>
> **It is a provable no-op at pre-C3 semantics.** R4 has exactly one caller,
> the `else if` after R3. Pre-C3 R3 was base-blind, so it returned `None` only
> when *no* entry had `assoc == entity` — meaning R4's candidate set never
> contained a same-entity entry in the first place. The exclusion restores that
> invariant rather than changing it, and the bridge (`Iterator.Item` answered
> from `Iterable.Item` — **different** entities) is untouched.
>
> **Landed as C3b** (`76ddd5f7`), maintainer-approved after independent review.
> [verified @ `76ddd5f7`, built and run] `leak5` → post-mono error;
> `distinct_samename` still builds and prints `result=i` (C6's, correctly
> unaffected); `control2`, `distinct`, `structcase{,_nobound}`, `callsite`,
> `selfproj` all byte-identical to baseline. Suite `3818 passed, 5 failed`,
> unchanged.

### C3b — the name fallback skips same-entity candidates

**Changes** one predicate in `assoc_sub_by_name` (`ctx.rs`): `e != assoc &&`.
**Test** none new; `assoc_projection_bound_cross_receiver.ks` stays failing —
`leak5` stops *miscompiling* but the frontend still accepts, which is C4's job.
**Expected delta** 0, and it is a *provable* 0: R4's single caller is the
`else if` after R3, and pre-C3 R3 was base-blind, so reaching R4 already meant
no same-entity entry existed. C3b restores that invariant rather than changing
it.
**Risk** low. Explicitly **not** C6: C6 makes the fallback base-*aware* and
deletes the cross-receiver `Iterable`/`Iterator` bridge; C3b filters on the
*entity*, and the bridge is cross-entity, so the two do not overlap.
**This is the commit that stops `leak5` emitting wrong code.**

### C4 — the conformance answer for a projection respects its base

**LANDED.** [verified @ this commit, built + full suite]

**Changes** `conforms_to`'s `TyKind::AssocProjection` arm keeps the declares-only
answer, exactly as §4 recommends. Alongside it, a new `TypeResolver` method
`assoc_projection_base_admits(assoc, protocol, base_spine)` — **default-permit**,
so the solver's test stubs answer as before — and `solve_conforms` intersects the
two. `WorldResolver`'s impl asks three questions in order, any one of which
permits:

1. Is the grant *receiver-free*? `collect_assoc_type_direct_bounds_inner` split
   into `collect_assoc_type_receiver_free_bounds_inner` (the alias's own
   `Conformances`, plus the owning protocol's where clause — both statements
   about the alias itself) and the owner-hierarchy walk. Only the second can
   name a receiver, so only the second is base-sensitive.
2. Otherwise: which owner-hierarchy clauses could have produced the grant? Read
   through **`WhereClausesOf`**, not `gather_bounds_from_where_clause` — the
   structured subject keeps the receiver that the latter's last-segment
   `resolve_type_entity` collapse throws away. A bare-`Item` subject names no
   receiver and permits.
3. Does any of them name *this* receiver?

Only "at least one clause could have granted it, all of them name a receiver,
and none of those receivers is ours" rejects.

**Comparison is by entity spine, not by TyVar.** `WhereSubject::spine()` flattens
a subject to `(root type parameter, then each projected alias)` — `[A]`, `[T,
Iter]`; `base_spine` in `solver.rs` flattens a `TyKind::AssocProjection` chain the
same way. That is the one bridge between the declaration world and the inference
world here, and it exists because `WorldResolver` still cannot lower a subject to
a TyVar. `None` on either side ("no nameable root": a `Self`-rooted chain, a
concrete base, an unresolved base) means *cannot compare*, which permits.

**Measured** suite **3818 passed / 5 failed → 3820 passed / 3 failed.** Two
flipped, zero collateral:

| test | before | after |
| --- | --- | --- |
| `assoc_projection_bound_cross_receiver` | fail | **pass** — frontend `E100 … B.Item !: Show` at the `needsShow(b.produce())` line |
| `assoc_projection_bound_on_container` | fail | **pass** — same diagnostic at `go()`'s body |
| `assoc_projection_bound_call_site` | fail | fail — **the plan was wrong here, see below** |
| `assoc_projection_bound_same_name_distinct_protocols` | fail | fail — C6's, still blocked |
| `assoc_projection_bound_self_subject` | fail | fail — C9's, D8 keeps `Self` collapsing |

> **⚠ "So does `assoc_projection_bound_call_site.ks` (their accept comes from the
> same mechanism)" is REFUTED.** [measured @ this commit]
>
> `call_site`'s accept does **not** come from this mechanism, and the file's own
> header already said so. Its callee `good[A] where A: Producer, A.Item: Show` is
> correct: the body projects off `A`, and the clause is about `A`, so C4's base
> check finds a genuine **match** and permits — as it must. The missing check is
> the *call-site obligation* `A.Item: Show` at `good(StrSrc(…))`, which is never
> emitted at all. That is C7 (`solver.rs:3534` + the `Reduce` projection policy),
> unchanged.
>
> Cause of the bad claim: C4's section was written from the §4 ordering table,
> which only ever modelled `leak5`. `on_container` and `call_site` were added to
> the expected-delta list by analogy, and the analogy holds for exactly one of
> them. Expected delta was −3; the true delta is **−2**.

**Risk taken** the one the brief flagged — this is the direction that produces
false rejections. Contained by construction: the check narrows *only*
`TyKind::AssocProjection`, and every unknown permits. Zero new `E100`s across the
suite bears that out.

**This is the commit that makes `leak5.ks` reject.** Post-C3b it produced a
post-mono error and no binary; it now produces a frontend `E100` and no binary.
`control2`, `distinct`, `structcase_nobound`, `distinct_samename`, `callsite`,
`selfproj` are all byte-identical to their C3b behaviour.

### C5 — `lib.rs:705`: emit container-level projection clauses

**Changes** one skip site. Depends on C3 (adds a push).
**Test** a new positive file — a struct whose method *uses* the bound it declares
(`where A.Item: Show` and a body that calls `.show()` on `a.produce()`) — which
does not compile today. `assoc_projection_bound_on_container.ks` already covers
the negative direction and passed at C4.
**Expected delta** +1 pass; possible new accepts in
`declarations/associated_types/`.
**Risk** low-medium.

### C6 — narrow `solver.rs:2836` to same-resolved-base name matches

**Changes** one condition.
**Test** `assoc_projection_bound_same_name_distinct_protocols.ks` flips to passing.
**Expected delta** unknown until C1 reports; the fallback fires 27×/compilation.
**Risk** **HIGHEST** in the plan. Hold this commit until C1's numbers are in and
the `Iterable`/`Iterator` bridge is shown to survive on the deferred-`Associated`
path. If it does not, the fallback stays and the equality bridge gets fixed first,
in its own commit.

### C7 — the call-site skip sites

**LANDED (first of the three).** [verified @ this commit, built + full suite]

> **⚠ THE SITE ATTRIBUTION WAS WRONG.** [measured @ `b096cdf9` + the C7 spike]
>
> The plan, and the Sequencing table in `decisions.md`, assign
> `assoc_projection_bound_call_site.ks` to **`solver.rs:3534`** (`:3565`
> pre-commit-1, `:3598` today) — "the call-site obligation for a direct `Def`
> call". **That site is never reached by this test, and deleting its guard is a
> measured no-op.**
>
> `emit_resolved_call` has exactly two callers, `solver.rs:3409` and `:3439`,
> both inside `solve_overloaded_call`. It is the **overload-resolution** path.
> `good(StrSrc(…))` is an unambiguous call, so it never goes there: the callee
> is typed by `lower_entity_ref` (`generate.rs:1810`), whose where-clause
> emitter is **`emit_where_clause_constraints_with_subs` — the
> `generate.rs:1941` site**, filed in the Sequencing table as the "call-site /
> type-formation path".
>
> Instrumented at `emit_resolved_call`, `solve_member`, and
> `emit_where_clause_constraints_with_subs` on `callsite.ks`: 20 / 11876 / many
> hits respectively, **one** of which carried a `Projection` subject, and it
> was the `generate.rs` one. Deleting the `solver.rs` guard alone changed
> nothing on any of the eight repros.

**Changes** the guard at `generate.rs:1941` goes, plus the `Opaque`/`Reduce`
projection policy on `lower_subject` / `lower_subject_with_base`. Both remaining
call-site sites keep their guards and get `Opaque` (a no-op for the `Param`-only
subjects that reach them), so this commit is exactly one behavioural site.

**The `Reduce` policy is load-bearing, and the plan is right about why.**
Deleting the guard with `Opaque` still accepts `callsite.ks`: the subject lowers
to `assoc_projection(StrSrc_tv, Producer.Item)`, which never reduces, and C4's
base check on an `AssocProjection` with a *concrete* base has no spine to compare
(`None` = cannot compare = permit). `ProjectionPolicy::Reduce` calls
`project_associated`, which emits the `Associated` constraint, the projection
resolves to `String`, and `String: Show` fails for real. Measured: guard-deletion
alone = no change on all eight repros; guard-deletion + `Reduce` = frontend
`E100`.

**Measured** suite **3820 passed / 3 failed → 3821 passed / 2 failed.** One
flipped, zero collateral:

| test | before | after |
| --- | --- | --- |
| `assoc_projection_bound_call_site` | fail | **pass** — frontend `E100 … String !: Show` at `callsite.ks:51:21`, under the `good` callee token |
| `assoc_projection_bound_same_name_distinct_protocols` | fail | fail — C6's, still blocked |
| `assoc_projection_bound_self_subject` | fail | fail — C9's |

`leak5`, `control2`, `distinct`, `distinct_samename`, `structcase`,
`structcase_nobound`, `selfproj` are byte-identical to their C4 behaviour.

**Blast radius, bounded by measurement.** 19 files in `testdata/` carry a
where-clause projection bound, plus `lang/std/iter/adapters.ks` (`:397`, `:666`,
`:866`). Those three stdlib bounds are **container-level clauses on structs**,
and `WhereClausesOf` only reads an entity's own `AstWhereClause` — it does not
walk up to a parent — so an `Iterator`-adapter construction never sees them
here. That is C5's territory, still guarded. Confirmed by the green suite and by
`peekable()` running correctly.

**The two remaining call-site sites see no projection subject anywhere in the
corpus.** [measured, spike reverted, `solver.rs` byte-identical —
`shasum 830b97a3b83f24f0d827d638767b644484d6dfed` before and after] Instrumented
both skip arms and swept all 19 projection-bound files plus 12
`stdlib/iterator/` files: **zero hits at each.** So:

- **`solver.rs:3598` (`emit_resolved_call`, overloaded direct-`Def` call)** — the
  *same shape* as the site fixed here, and the same edit applies verbatim: drop
  the `matches!(subject, Param(_))` guard, pass
  `ProjectionPolicy::Reduce(&span)`. **No corpus test would flip.** It needs a
  new test first: a generic function with a projection bound that is
  *overloaded*, so `solve_overloaded_call` is the path taken.
- **`solver.rs:4498` (`solve_member`, member/method call)** — **not** the same
  shape. It uses `subject.as_param()` and a two-stage lookup
  (`resolution.type_params` → `fresh_params[idx]`, else `subs`) that `SubjectRoot`
  does not model, exactly as its in-source comment says. The clean adoption is to
  build one merged `Vec<(Entity, TyVar)>` = `zip(type_params, fresh_params)`
  followed by `subs` — `find` gives first-wins, which *is* the two-stage
  semantics — and then use `SubjectRoot::Subs(&merged)`. Note this path is hot
  (11876 hits compiling a 50-line program), so the merge wants to be conditional
  on a projection subject actually being present. **No corpus test would flip**,
  and more than that: `resolution.where_clauses` never contains a projection
  subject on the whole projection-bound corpus, including
  `assoc_projection_bound_extension.ks`, whose
  `extend Box[T]: Show where T.Item: Show` is served by the body-setup emitter
  (`lib.rs`'s `emit_method_projection_bound_constraint`) rather than by the
  `Box(…).show()` call site. A test for this site has to be written from
  scratch — the natural one is `Box(inner: StrSrc(s: "z")).show()`, the negative
  twin of the existing positive.

**Risk taken** the over-rejection direction, as flagged. Contained: an
unmappable subject root still yields `None` from `lower_subject` and is skipped,
which is the same conservative permit as before.

### C8 — the two inert skip sites, `Static` wf, `find_protocol_type_args`, entailment

`lib.rs:930`, `solver.rs:5786`, `solver.rs:5911`, `resolve.rs:1750`,
`entailment.rs:40`. One commit each, each with a small positive test for the
feature it un-inerts. **Risk** low. Independent of everything above except the
re-key.

### C9 — the D8 `SelfType` flip

**Changes** `where_clauses.rs:309` stops returning `None` for `Self`-rooted
chains; `resolve_bound_subject` emits `WhereSubject::SelfType`;
`lower_subject`'s `SelfType => None` becomes `=> Some(self_tv)`. Per D8 the
compiler generates the reader work list — every non-exhaustive match stops
compiling.
**Test** `assoc_projection_bound_self_subject.ks` flips to passing.
`h_selfq_neg.ks`'s shipped negative and the five latent `where Self: Q` files
listed in `problem.md` must stay green.
**Expected delta** +1 pass, and the ~8 `Some(*param) == target_entity`
comparison sites flip together.
**Risk** medium. Last, because it is the only commit whose work list is not known
until it is attempted.

## Risks, ranked

1. **C6 — narrowing the name fallback.** 27 stdlib fires per compilation on the
   `Iterable`/`Iterator` bridge. [verified] Mitigation: C1's audit gates it; narrow
   rather than delete; be prepared to fix the equality bridge first instead.
2. **C3 — the re-key's `NOBASE` population.** ~50 fires/compilation the spike
   could not classify because the recorded value is not a projection. Mitigation:
   C1 exists precisely to classify them. Watch for deferred-constraint failures,
   not conformance failures.
3. **C7 — call-site obligations.** Turns a blanket permit into a real check on
   every generic call carrying a projection bound. Mitigation: land the three
   sites as three commits; the `Reduce` policy must be right or concrete call
   sites judge an unreduced projection.
4. **C4 — the conforms move.** Small and countable (6 callers, one crate) but it
   is the commit that changes an *answer*, so any latent reliance surfaces here.
5. **C9 — the `SelfType` flip.** Bounded by the compiler, but the count of
   affected sites in this area has been wrong four times.
6. **C5, C8.** Low.

## One-line answers

- **Correct key:** `(base TyVar, assoc Entity)`, base stored raw, both sides
  `ctx.resolve()`d at lookup, `None` matching `None` and a baseless *query*
  resolved by unambiguity. Not `WhereSubject` — that is the clause-level key, and
  `lower_subject` is the one bridge between the two worlds.
- **On a miss:** always fall through to the general path. The memo is never an
  obligation, and every site's fallback is strictly more informative than the
  shortcut. Over-rejection, if it comes, comes from unsolved deferred
  constraints, not from the miss.
- **The commit that makes `leak5.ks` reject: C4** — landed; frontend `E100 …
  B.Item !: Show`. The commit that stops it
  emitting wrong code is ~~**C3**~~ **C3 + C3b** — C3 alone leaves the leak
  live via the R3→R4 fallthrough (see the ⚠ banner on C3). C4 does nothing
  without both, and per the ordering table it now needs **C3b** specifically:
  until `b.produce()` types as `B.Item`, C4's base check finds a genuine match
  against the `A.Item` clause and permits.

---

# C1 measurement — the numbers that replace every estimate above

[measured @ `a8fa672c`, `KESTREL_DEBUG=audit-subject`, all 3655 testdata files.
`kestrel dump diagnostics` verified equivalent to a full build — 1456 reads
either way — so the sweep used it.]

**4,939,238 reads, ~1352 per compilation. 94.23% NONE · 2.07% MATCH · 3.70%
MISS · zero MISMATCH · zero AMBIGUOUS.**

| site | reads | MATCH | MISS | div% |
|---|---|---|---|---|
| `solver:lower_hir_ty_sub:AssocProjection` (R7) | 1733616 | 10984 | 14622 | 0.84% |
| `solver:solve_associated` (R3) | 1173329 | 80431 | 10969 | 0.93% |
| `solver:solve_associated:name-fallback` (R4) | 1081929 | 0 | 98689 | 9.12% |
| `generate:AssocProjection` (R2) | 482511 | 7312 | 58485 | 12.12% |
| `solver:lower_hir_ty_sub:AliasUse` (R6) | 380129 | 0 | 0 | 0% |
| `solver:solve_member` (R5) | 87724 | 3656 | 0 | 0% |
| `generate:AliasUse` (R1) | **0** | – | – | – |

## Blast radius

3655/3655 files diverge — but that number is useless on its own. **3650 of
them diverge by exactly the same 50-MISS stdlib floor**, a byte-identical
7-row signature present in every compilation. Only **five** files contribute
anything of their own:

```
56  declarations/wacky_inference/transitive_equality_constraints_in_extension_method.ks
54  declarations/extensions/init_in_generic_extension_no_double_type_args.ks
53  declarations/associated_types/iterable_iter_bound_propagates.ks
51  validation/type_checking/tuple_index_with_associated_type_equality.ks
51  declarations/associated_types/where_clause_two_associated_types_equal.ks
```

The plan's static estimate (13 + 17 files) understated it; "the whole suite"
overstates it. The truth is **the stdlib prelude, invariantly, plus five files.**

## Consequences for the remaining commits

- **Zero MISMATCH corpus-wide.** C3 can only ever *lose a shortcut*, never swap
  one answer for another. The failure mode to watch is an unsolved deferred
  `Associated` constraint — "could not infer type" — **never an E100 storm.**
  Predicted in the *`None` rules*; now measured.
- **C2 is a no-op on this corpus.** R6 never diverges across 380129 reads, R1
  never executes at all, and R5 has zero divergence. Re-scope or drop it.
- **C6 is blocked** — see the §3 refutation above.
- Both isolations confirmed at exactly one read each:
  `leak5 \ control2` → R7, `assoc=Producer.Item via=Producer.Item@1`, base `A`
  vs query base `B`. `distinct_samename \ distinct` → R4,
  `assoc=ProducerB.Item via=ProducerA.Item@1`.

## What C1 actually shipped

`where_clause_assoc_subs` is `Vec<(AssocSubKey, TyVar)>` carrying the base,
**stored raw and resolved only at lookup**. The field is now **private to
`ctx.rs`** — stronger than planned: no reader can reach the table at all, only
`push_assoc_sub` / `assoc_sub` / `assoc_sub_by_name`, every one of them
byte-for-byte base-blind today. Verified: all references outside `ctx.rs` are
comments.

Suite `3815 passed, 0 failed`, run twice. Both miscompiles unchanged.
