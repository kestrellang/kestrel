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
**This is the commit that stops `leak5` emitting wrong code.**

### C4 — move the `AssocProjection` conformance arm into `solve_conforms`

**Changes** `resolve.rs:627` keeps the declares-only answer; `solver.rs:2115`
intersects it with a resolved-base check against the clause's `WhereSubject`.
Requires reaching the owning clause from the solver — via `WhereClausesOf` on the
body owner, the same walk `collect_assoc_type_direct_bounds_inner` already does at
`resolve.rs:2072-2083`.
**Test** `assoc_projection_bound_cross_receiver.ks` **flips to passing**. So do
`assoc_projection_bound_on_container.ks` and `assoc_projection_bound_call_site.ks`
(their accept comes from the same mechanism).
**Expected delta** −3 failures. Any *new* failure is a program that was relying on
a bound leaking across receivers.
**Risk** medium. Contained: 6 `conforms_to` callers, all in one crate.
**This is the commit that makes `leak5.ks` reject.**

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

### C7 — the three call-site skip sites

**Changes** `generate.rs:1941`, `solver.rs:3534`, `solver.rs:4435`, plus the
`Opaque`/`Reduce` projection policy on `lower_subject`. One commit each; the
policy lands with the first.
**Test** `assoc_projection_bound_call_site.ks` (already green from C4 — these
commits make it reject at the *call site* with a user-code span rather than
wherever C4 catches it, which is the better diagnostic; add a span assertion).
**Expected delta** new rejections wherever a generic call has an unsatisfied
projection bound. The three stdlib projection bounds (`adapters.ks:397`, `:666`,
`:866`) are all `Copyable` / `not Copyable`, answered structurally before the arm,
so the stdlib should be unaffected — **verify, do not assume.**
**Risk** medium-high, and it is the one that most plausibly produces false
rejects.

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
- **The commit that makes `leak5.ks` reject: C4.** The commit that stops it
  emitting wrong code is **C3**, and C4 does nothing without it.
