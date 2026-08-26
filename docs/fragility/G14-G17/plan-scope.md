# Stage 3c — the scope half of G17

> **Commit IDs renumbered 2026-08-25.** This document originally used
> `C10a/C10b/C10c/C11`. Those collided with commits already landed under
> different meanings — `f60d8247` is "G17 C10" (the overloaded call site) and
> `58e2c550` is "G17 C11" (the member obligation). The scope commits are now
> **S1–S4**; bare `C10` in the original prose meant the scope fix and is now
> `S2`. Two agents working from separate briefs picked the same next number,
> which is what happens when IDs are allocated per-document instead of
> per-audit.

> ## ⚠ Q5's F12 finding is REFUTED — measured on a clean tree
>
> [verified @ `b3f79f4f`, clean tree, current binary, built and run]
>
> Q5 reports the shadowing repro as **ACCEPTED** — a wrong-accept where an
> unbounded method type parameter named `Item` gains `Show`. **It is rejected.**
> The original measurement was taken with another agent's uncommitted
> `solver.rs` in the binary and was disclosed as such; on a clean tree it does
> not hold.
>
> What *is* live is the same leak in the opposite direction — a **wrong-reject
> with a degraded diagnostic**:
>
> | repro | result |
> | --- | --- |
> | `extend Producer where Item: Show { func leak[Item](x: Item) … }` | `E100: does not conform to protocol; does not satisfy constraint` — **spanless**, "(no source location available — diagnostic attached to a synthesized node)" |
> | same, parameter renamed `U` | `E100: no member 'show' on type 'U'` at `:13:43`, expression underlined |
>
> **Renaming a method's type parameter turns a precise located error into a
> spanless vague one.** The subject still resolves to the wrong `Item` — the
> scope leak is real — but it over-rejects rather than under-rejects.
>
> Two consequences for this plan:
>
> 1. **S2's risk profile improves.** S2 is reject-direction, and this case
>    already over-rejects, so the scope fix should *repair* it rather than
>    endanger it. Add this A/B pair to S2's test set; the control must stay
>    green and the shadow case should gain a real span.
> 2. **The `E439` hole in Q5 stands** — it compares only against ancestors'
>    `TypeParams` and extension LHS params, so a `TypeAlias` is invisible to it.
>    That is a source-read, unaffected by the binary, and it is why the repro
>    had to be assoc-shaped at all.
>
> This is the fifth prediction in this work refuted by running rather than
> reading. See `plan-3a.md`'s banners for the other four.

> **Scope.** G17 as filed has two halves. Every commit so far (C1, C3, C3b, C4,
> C7, C9, C10, C11) fixed the **key**: `T.Assoc: P` collapsing onto the bare
> associated type. Nobody has touched the **scope**: `TypeResolver` resolves
> where-clause subject *names* in the ambient `body_owner` scope rather than in
> the scope of the entity the clause is written on. This is that design.
>
> **S4 re-verified live @ `b3f79f4f`** — `struct Pair[A,B] where …, A.Item: Show`
> with a *direct* `self.b.produce().show()` still accepts at the frontend and
> dies post-mono. The committed C11 (`solve_member`, `58e2c550`) did **not**
> close it; the two are distinct arms.
>
> **Status:** designed, not implemented. No production code was written for this
> document. No spike was run; no file outside this one was touched.

## Preflight

```
pwd              /Users/dino/Documents/Projects/kestrel
git log -1       29de8da8 fix(type-infer): G17 C9 — construct WhereSubject::SelfType, resolve it per-receiver
git rev-parse    29de8da8af250051d0a6b2f152b1dce9dfbc5473
branch           arch/fixes   (parent checkout; no worktree created or entered)
build            cargo build --release --bin kestrel → up to date (0.11s, no recompile)
```

**Measurement-base disclosure.** The working tree is shared with another agent.
At measurement time it carried their **uncommitted** edits to
`lib/kestrel-type-infer/src/solver.rs` (13 insertions / 13 deletions; `shasum
3f23a3f6ae6f2e57789d933e3a994c91bf2b9e80`) plus two new untracked testdata files
and one modified one. `cargo build` reported *up to date*, so
`target/release/kestrel` (built 2026-08-25 13:08:54) **includes** those edits.
Every result below is therefore "`29de8da8` + one agent's in-flight solver.rs",
not a clean `29de8da8`. The A/B pairs below are internally consistent (both
halves of every pair ran against the same binary), which is what makes them
decisive; the absolute diagnostics could shift if that work lands differently.

---

## 1. The two leaks, re-verified at HEAD

### Leak B — the resolution context. **LIVE, and not gated by E439.**

`gather_bounds_from_where_clause(param, entity, …)` reads the `AstWhereClause`
off `entity` but matches subjects with `self.resolve_type_entity(subject)`,
which runs `ResolveTypePath { context: self.body_owner }`. The clause's names
are looked up in the *body being inferred*, not where the clause is written.

**The classic F12 shape is now blocked by a different check.** `struct Outer[T]
{ func f[T]() }` fires `E439 type parameter 'T' shadows outer type parameter`
(`kestrel-analyze/src/decl/generics.rs:415`). The leak is still visible
underneath it — with the shadow, `x.show()` on the *unbounded* inner `T` is
accepted, and `self.v.show()` on the *bounded* outer `T` is rejected with
`no member 'show' on type 'T'` — but a program in that shape does not build,
so it is not a shippable defect.

**E439 compares names only against ancestors' `TypeParams` and extension LHS
params.** An associated type is a `TypeAlias`, so it is invisible to that check,
and the leak is fully live through it:

`temp/g17scope/assoc_shadow.ks` — [verified @ `29de8da8`+wt, built and run]

```kestrel
protocol Producer { type Item; func produce() -> Item }

extend Producer where Item: Show {
    public func leak[Item](x: Item) -> String { x.show() }   // ← accepted
}
```

| file | only difference | result |
| --- | --- | --- |
| `assoc_shadow.ks` | method type param named `Item` | **accepted** — one spanless `E100`, no error on `x.show()` |
| `assoc_shadow_control.ks` | method type param named `U` | correct `E100 no member 'show' on type 'U'` at `:15:43` |

Renaming a method's type parameter from `U` to `Item` makes an **unbounded**
type parameter satisfy `Show`. That is the F12 shape, alive at HEAD, with no
E439 and no other diagnostic covering it. It leaks in both directions in the
same file: the method's `Item` wrongly *gains* the bound, and the protocol's
`Item` wrongly *loses* it — which is what the spanless `E100` is (the control
does not emit it).

### Leak A — the ancestor chain. **Not an independent leak.**

Both `_inner` functions walk `parent_of` from `self.body_owner`. The finding is
that nothing checks the clause's owner against the *subject's* owner. Re-reading
it against the code: the walk is **the right walk**. It answers "which clauses
are in force at this use site", and that is exactly `body_owner` and its
ancestors — a clause on `extend Array[T] where T: Comparable` binds `Array`'s
own `T` from an extension that is *not* an ancestor of `Array`, so the
subject-owner chain would be the wrong one to walk.

What is missing is not a chain filter but a **scope check**, and per-holder
resolution *is* that check: a name that does not resolve in holder `E`'s own
scope cannot resolve to subject `S`, so `WhereClausesOf { entity: E }` rejects
the mismatch structurally. Fixing Leak B therefore subsumes Leak A for
`TypeParameter` subjects, with no separate commit.

It does **not** subsume it for `TypeAlias` subjects, because an alias arrives
from a *receiver type* rather than from the scope. That residue is the receiver
question, not the scope question, and it is §6's S4.

### Residual, found while verifying — cross-receiver **member lookup** is still base-blind

C4 made the *conformance* answer base-aware. It did not touch member lookup.

`temp/g17scope/xrecv_member.ks` — [verified @ `29de8da8`+wt, built and run]

```kestrel
struct Pair[A, B] where A: Producer, B: Producer, A.Item: Show {
    public func go() -> String { self.b.produce().show() }   // B.Item, unbounded
}
```

| file | result |
| --- | --- |
| with `A.Item: Show` | frontend **accepts**; fails post-mono: `type 'std.text.String' does not implement 'show'` |
| clause deleted | correct `E100 no member 'show' on type 'B.Item'` at `:20:34` |

Same class as `leak5`, one layer over: the clause on `A.Item` makes a member
appear on `B.Item`. It is `resolve_member` → `TyKind::AssocProjection` arm →
`resolve_assoc_type_member` → `collect_assoc_type_protocol_bounds`, which
discards `base` exactly as `conforms_to` used to. **This is a new G17 datapoint
and it blocks checking G17 off** — see §8.

---

## 2. Can all three functions be rebuilt on `WhereClausesOf`?

Yes — with **one blocking difference** that needs a query split, and one
deliberate widening.

### What `WhereClausesOf` gives that the raw walk does not

- Names resolved in **the entity's own scope** (`context: entity`). This is the
  whole point.
- A structured `WhereSubject`, so `T.Assoc: P` stays a `Projection` instead of
  collapsing to `Param(Assoc)`, and `Self` stays `SelfType` (C9).
- `protocol_type_args` (unused by these three; `find_protocol_type_args_from_bounds`
  already reads them from this query).
- Memoization per `(entity, root)`. The three targets are **not** memoized and
  sit on the member-resolution hot path, so this is a net win — but the query
  body calls `lower_ast_type` and two semantics queries on first touch, so the
  first-call cost moves rather than disappearing. Worth a build-time check on
  the S2 commit, not a design blocker.

### The blocking difference: implicit bound injection

`resolve_where_clauses` runs `inject_implicit_copyable_bounds` and
`inject_implicit_static_bounds` after the declared clauses. A naive migration
therefore adds `Copyable`/`Cloneable`/`Static` to **every** type parameter's
bound set. Consequences, all real:

- `resolve_param_member`'s `if direct_bounds.is_empty() && …protocol_bounds.is_empty()`
  early-out (`resolve.rs:1851`) becomes **unreachable** — every param would carry
  at least one bound. The `MemberError::NotFound` fast path is silently deleted.
- `Cloneable` carries `clone()` (`lang/std/core/copy.ks:33`). Injecting it makes
  `t.clone()` resolve on any param whose requirement is `RequiresCloneable`,
  without an explicit `: Cloneable` bound. `Copyable` and `Static` are empty
  protocols, so those two add no members — but they *do* enter the "direct
  bounds" set that `resolve_param_member` uses to **out-rank** inherited
  candidates, so a name collision could newly resolve to a different member.
- `conforms_to`'s `TyKind::Param` arm would start answering `true` for
  `T: Copyable` on every non-`not Copyable` param. That is arguably the right
  answer, but it is a behaviour change smuggled in under a refactor.

**Fix, single-source-preserving:** split `resolve_where_clauses` in
`where_clauses.rs` into

```rust
pub fn declared_where_clauses(ctx, entity, root) -> Vec<WhereClause>   // the match loop only
pub fn resolve_where_clauses(ctx, entity, root) -> Vec<WhereClause>    // = declared + the two injections
```

and expose a second memoized query `DeclaredWhereClausesOf { entity, root }`.
The injections stay in exactly one place; `WhereClausesOf` stays the
"everything in force" answer; the three targets ask the narrower question they
have always been asking. This is not a second source of truth — it is one
pipeline with a named intermediate.

### Does the silent drop lose anything?

**No.** `resolve_where_clauses` drops a clause whose subject or protocol does
not resolve (`continue`). `gather_bounds_from_where_clause` drops the same
clauses for the same reason (`resolve_type_entity` returns `None` → the `&&`
chain short-circuits). Neither reports. The sets of *droppable* clauses differ
only where the two resolution contexts differ — which is precisely the delta
this plan is measuring, not a loss.

Two smaller asymmetries, both benign:
- `NegativeBound` is dropped by the query and ignored by the raw walk. Same.
- `Equality` becomes `TypeEquality`/`DirectEquality` in the query and is ignored
  by the raw walk. The three targets only consume `Bound`, so unchanged.

### The deliberate widening, per function

| function | subject predicate today | on `DeclaredWhereClausesOf` |
| --- | --- | --- |
| `collect_param_direct_bounds_inner` | `resolve_type_entity(subj) == param` | `subject.as_param() == Some(param)` |
| `collect_assoc_type_direct_bounds_inner` | same, collapsing `A.Item` → `Item` | `subject_receiver_of(&subj, alias) != NotThisAlias` |
| `collect_assoc_type_receiver_free_bounds_inner` (parent-protocol half) | same | same as the alias row |

The alias rows must accept `Unnamed` **and** every `Spine`, not just a matching
one: these functions hold no base to compare against. Restricting them here
would double-fix what C4 already owns and would reject in a place that has no
receiver. The receiver check stays where it is (`assoc_projection_base_admits`)
and gains a second caller in S4.

---

## 3. The correct ancestor chain

**Unchanged: `body_owner` and its ancestors.** Concretely:

> A clause on entity `E` binds subject `S` iff `E ∈ {body_owner} ∪ ancestors(body_owner)`
> **and** the clause's subject, resolved **in `E`'s own scope**, is `S`.

The first conjunct is "which clauses are in force here" and is already right.
The second is the fix. Neither `ancestors(owner_of(S))` nor "the clause-holder's
own chain" is correct: `extend Array[T] where T: Comparable` puts a clause on an
entity that is not an ancestor of `T`'s owner, and it is a legitimate bound.

**What changes for a body that mentions a param from an outer scope:**
`struct Outer[T] where T: Show { func f() { self.v.show() } }`. Today: walk
`f → Outer`, read `Outer`'s raw clause, resolve `"T"` in `f`'s scope. It works
only because `f` declares no `T` — it is correct by luck. After: resolve `"T"`
in `Outer`'s scope, get `Outer`'s `T`, match. Same answer, now for a reason.
When `f` *does* introduce a colliding name (the `Item` case above), today's
answer flips to wrong in both directions and the new one does not.

**One owner for the walk.** All of `collect_param_direct_bounds_inner`,
`collect_assoc_type_direct_bounds_inner`, `assoc_projection_base_admits`
(`resolve.rs:807-842`), `collect_extension_where_clause_protocols` and
`find_protocol_type_args_from_bounds` re-implement a slice of it — the last
walks only `parent_of(param)` and therefore misses extension-level clauses on a
method body, a *partial-chain* gap distinct from the scope leak. Extract:

```rust
/// Every where clause in force at `scope`, holder-first, each resolved in its
/// own holder's scope. The single source of truth for "which clauses apply here".
pub fn clauses_in_force(
    ctx: &QueryContext<'_>, root: Entity, scope: Entity,
) -> impl Iterator<Item = (Entity /* holder */, WhereClause)>
```

with the visited-set dedup that all four copies already carry. Five call sites
collapse onto it and the walk stops being re-derivable.

---

## 4. Does this close A16/G15?

> G15: `constraint_entailed_by`'s param tier queries `WhereClausesOf` on the
> `TypeParameter` entity, which never carries a where clause, so the tier is inert.

**Yes — but only if the extraction in §3 happens, and it needs its own commit.**

The defect (`entailment.rs:97`) is `WhereClausesOf { entity: param }`. A
`TypeParameter` has no `AstWhereClause`, and the injections read `TypeParams` off
the same entity, which a param also lacks — so the query returns `vec![]` and
the tier is dead, exactly as filed. `resolve.rs` gets it right by hopping
`parent_of(param)` first; so does `find_protocol_type_args_from_bounds`
(`resolve.rs:1895`), which is the closest working model.

`clauses_in_force(ctx, root, scope)` is that hop plus the chain, parameterised
on scope. Entailment has no `body_owner` — it is a static analysis with no body
— so it passes `scope = parent_of(param)` and the resolver passes
`scope = body_owner`. The missing capability is supplied by the same function,
which is what "does A16 fall out" is asking.

**It must be a separate commit** because the direction is opposite: making a
dead *permit* tier live can only turn rejects into accepts. That is the same
direction as G14's wrong-accepts, and it must not be measured jointly with S2's
reject-direction change or the two deltas cancel in the aggregate.

---

## 5. Detection first — the measurement design

Following the C1 precedent. **Correction to the brief:** the gate is not an
`KESTREL_AUDIT_SUBJECT` env var — `ctx.rs:689/705/783` uses
`kestrel_debug::is_enabled("audit-subject")`, i.e. `KESTREL_DEBUG=audit-subject`.
Keep that. The new category is `audit-scope`.

### Where it goes

Inside `gather_bounds_from_where_clause`, at the top, before the existing loop —
one site, because it is the *only* raw `AstWhereClause` walker left in
`kestrel-type-infer` (`resolve.rs:2474`; `where_clauses.rs:57` is the query
itself). Both `_inner` functions and the receiver-free half funnel through it,
so one probe covers all three targets.

```rust
if kestrel_debug::is_enabled("audit-scope") {
    self.audit_scope(subject_entity, entity, /* old */ &old_protocols);
}
```

`audit_scope` recomputes the answer via `DeclaredWhereClausesOf { entity }` +
the §2 predicate, diffs it against the raw-walk answer for the same
`(subject, holder)` pair, and emits one line per difference:

```
ktrace!("audit-scope",
    "{verdict} kind={param|alias} subject={path} holder={path} \
     old=[{protocols}] new=[{protocols}] shadowed={bool}");
```

`verdict ∈ { SAME, ADD, DROP, SWAP }`. `ADD` = the new answer finds a bound the
old missed (permit-direction; today's wrong-*rejects*). `DROP` = the old answer
found one the new does not (reject-direction; today's wrong-*accepts* — the
dangerous column, and the one that sizes S2's suite risk). `shadowed` records
whether `ResolveTypePath` in `body_owner`'s scope and in `entity`'s scope
returned **different** entities, which separates "the name means something else
here" from "the name does not resolve there at all".

Detection only: `audit_scope` returns `()` and the production path is untouched,
byte-for-byte, when the category is off. Same contract as `audit_assoc_sub`.

### The sweep

Population, measured [verified @ `29de8da8`+wt]:

```
testdata .ks files                3665     of which contain `where`   368
lang/*.ks files                    191     of which contain `where`   100
`where` occurrences, both trees              1162
```

That is the *population the probe walks*, not a delta prediction. **No expected
delta is stated in this document.** Three predictions in `plan-3a.md` were
extended by analogy and refuted by measurement; the number this sweep produces
replaces every estimate, including the temptation to infer one from 1162.

Sweep command shape (implementer runs it, records the base commit):

```sh
KESTREL_DEBUG=audit-scope ./target/release/kestrel check <file> 2>&1 \
  | grep '^audit-scope'
```

over all 3665 testdata files plus a `lang/std` build, aggregated by
`(verdict, kind, shadowed)` and by `holder` entity path. Two figures decide the
sequencing:

1. **`DROP` count with `shadowed=true`** — genuine F12 aliasing being removed.
   These are the wins.
2. **`DROP` count with `shadowed=false`** — a name that resolves in `body_owner`
   but *not* in the holder's scope. Each one is a potential wrong-reject and
   must be inspected individually before S2 lands. If this column is non-zero,
   S2 does not land until every entry has an explanation.

---

## 6. Commit sequence

Each commit builds, passes, and is independently reviewable. Suite baseline at
HEAD is **3821 passed / 2 failed** (per the audit's C9 entry).

### S1 — detection only

- **Changes:** `audit_scope` in `resolve.rs` behind `kestrel_debug::is_enabled("audit-scope")`;
  `DeclaredWhereClausesOf` query + the `declared_where_clauses` split in
  `where_clauses.rs`; `clauses_in_force` extracted in `where_clauses.rs` with
  the five call sites left **unchanged** (the function is introduced, not yet
  adopted, so the diff is additive).
- **Expected suite delta:** **zero**, bit-identical. The probe is inert with the
  category off; the query split preserves `resolve_where_clauses`' output
  exactly (`declared + injections` is the same list in the same order).
- **Test:** a `where_clauses.rs` unit test that
  `DeclaredWhereClausesOf` ⊂ `WhereClausesOf` and that the difference is exactly
  the injected `Copyable`/`Cloneable`/`Static` bounds — this is the invariant the
  split is claiming, and it is the one that would silently rot.
- **Risk:** low. Only real risk is the query split perturbing memo identity;
  the bit-identical suite run is the check.
- **Deliverable:** the sweep numbers, appended to this file.

### S2 — the scope fix

- **Changes:** `gather_bounds_from_where_clause` deleted. The three targets read
  `clauses_in_force(ctx, root, self.body_owner)` + `DeclaredWhereClausesOf`,
  matching with `as_param()` (params) / `subject_receiver_of` (aliases) per §2.
  `collect_extension_where_clause_protocols` and
  `find_protocol_type_args_from_bounds` adopt `clauses_in_force` in the same
  commit — the latter gains the ancestor chain it is missing.
- **Expected suite delta:** whatever S1 measured. **Not predicted here.**
- **Test:** `assoc_shadow.ks` / `assoc_shadow_control.ks` promoted to
  `testdata/types/generics/` as a diagnostics pair. The positive asserts
  `// ERROR: no member 'show'` on the shadowing file; the control stays as the
  proof that the error is caused by the shadow and not by a generally absent
  check. Plus a `shadow_reject.ks`-shaped execution test for the wrong-reject
  direction *if* E439 can be side-stepped — with the alias shape it can, since
  the outer `Item` regaining its bound is what removes the spanless `E100`.
- **Risk:** **highest in the plan.** Reject-direction. Gated on S1's
  `DROP shadowed=false` column being zero or fully explained.

### S3 — G15 / A16

- **Changes:** `entailment.rs` tier 2 becomes
  `clauses_in_force(ctx, root, parent_of(param))`.
- **Expected suite delta:** permit-direction only; can only convert failures to
  passes or leave them. Not predicted.
- **Test:** an `entailment.rs` unit test that a bound declared on the param's
  *owner* is entailed — the assertion that is impossible to satisfy today.
- **Risk:** medium. Making a dead permit tier live is how G14-class wrong-accepts
  get in. Land it after S2 so the two deltas are separately attributable.

### S4 — cross-receiver member lookup (the §1 residual)

- **Changes:** `resolve_member`'s `TyKind::AssocProjection` arm passes its base
  through to `resolve_assoc_type_member`, which consults
  `assoc_projection_base_admits` before accepting a bound-derived member.
  Structurally the same move C4 made for `conforms_to`, at the sibling arm.
- **Expected suite delta:** not predicted; measure with the same probe extended
  to the member path.
- **Test:** `xrecv_member.ks` / `xrecv_member_control.ks` as a diagnostics pair.
- **Risk:** medium-high, reject-direction, and it needs `WorldResolver` to reach
  a resolved base — `assoc_projection_base_admits` already takes a pre-resolved
  `base_spine` computed by the caller precisely because only an `InferCtx` can
  resolve a `TyVar`, so the member path must do the same. Verify that plumbing
  exists before committing to this shape.

### Risk ranking

1. **S2** — reject-direction, unbounded until S1 reports, five call sites.
2. **S4** — reject-direction, and needs base plumbing that may not exist on the
   member path.
3. **S3** — permit-direction; safe for the suite, unsafe for soundness.
4. **S1** — additive and inert.

---

## 7. What does NOT get fixed

**Sixteen raw `AstWhereClause` walkers outside `kestrel-type-infer`**, none of
which this plan touches:

| # | site | scope it resolves in |
| --- | --- | --- |
| 1 | `kestrel-hir-lower/src/ty.rs:373` | its own `entity` |
| 2 | `kestrel-mir-lower/src/items/witness_lower.rs:153` | the witness `source` |
| 3 | `kestrel-mir-lower/src/items/function_sig.rs:320` | the function `e` |
| 4 | `kestrel-name-res/src/resolve_value.rs:569` | ancestor chain of `context`; **raw string** `Self`/name compare |
| 5-7 | `kestrel-name-res/src/resolve_type.rs:489, 662, 722` | ditto; `:722` matches by **last segment name** — the same collapse, one layer down |
| 8-9 | `kestrel-semantics/src/lib.rs:304, 447` | the `context` / `ext` entity |
| 10 | `kestrel-semantics/src/staticness.rs:159` | the `context` entity |
| 11 | `kestrel-analyze/src/body/move_tracking.rs:1968` | the ancestor `ent` |
| 12 | `kestrel-analyze/src/compilation/extension_conflict.rs:247` | the extension |
| 13 | `kestrel-analyze/src/decl/generics.rs:488` | `cx.entity` (E436/E437/E440) |
| 14 | `kestrel-analyze/src/compilation/constraint_cycles.rs:90, 107` | `entity` / `owner` |
| 15 | `kestrel-analyze/src/compilation/conformance_completeness.rs:641` | the protocol |
| 16 | `kestrel-doc/src/signature.rs:337, 524` | the documented entity |

**None is forced to change, and here is the plain reason:** every one of them
resolves against **its own entity**, not against an ambient `body_owner`. The
defect this plan fixes is specific to `WorldResolver`, which is the only type in
the tree that carries an inference-time body pointer into declaration-scoped
lookups. Sites 1-3 and 8-16 are correct-by-construction on the scope axis.

Sites 4-7 are **not** correct — they match subjects by raw string comparison on
the last path segment, which is the G17 *key* collapse reproduced independently
in name-res. They are out of scope here for a hard architectural reason, not a
soft one: `kestrel-name-res` does not depend on `kestrel-type-infer` (verified:
`kestrel-name-res/Cargo.toml` has zero references), and `WhereClausesOf` lives in
`kestrel-type-infer` and depends on `kestrel-name-res` and `kestrel-hir-lower`.
Routing them through the query is a **dependency cycle**. Fixing them means
either sinking the subject resolver below name-res or giving name-res its own
`WhereSubject`-shaped answer. Either is a separate finding; neither belongs to
G17's scope half. The same applies to `kestrel-semantics` and
`kestrel-hir-lower`. `kestrel-analyze` and `kestrel-mir-lower` *do* depend on
`kestrel-type-infer` and could migrate — but they are not leaking, so migrating
them would be churn.

Also not fixed: `WhereClause::TypeEquality` / `DirectEquality` entailment
(`entailment.rs:54` returns `false`), which is what C6 is blocked on.

---

## 8. Does G17 get checked off?

**S2 closes G17's scope half.** After it, no code in `kestrel-type-infer`
resolves a where-clause subject in a scope other than the clause holder's, and
the last raw `AstWhereClause` walker in the crate is gone — which is the literal
text of the finding ("re-derives where-clause bounds from raw `AstWhereClause`
instead of `WhereClausesOf` … and resolving subjects in the ambient `body_owner`
scope").

**G17 cannot be checked off at S2.** Three things block it:

1. **S4 — cross-receiver member lookup**, discovered while verifying this plan
   and not previously filed. `xrecv_member.ks` accepts at the frontend and dies
   post-mono. This is the *key* half, not the scope half, and it is the same
   class as `leak5`. G17 is a `high` finding because it emits wrong code; a
   surviving frontend-accept of the same shape keeps it open.
2. **C6 — `same_name_distinct_protocols`**, blocked as already recorded: the
   `solver.rs:2836` name fallback carries the whole `Iterable`/`Iterator` bridge
   with zero same-receiver hits, so narrowing it is a deletion. It needs
   re-planning against the equality clause (`TypeEquality`) first, which is the
   same gap §7 names in `entailment.rs:54`.
3. **The two known-failing tests** — `assoc_projection_bound_self_subject.ks`
   (annotation needs correcting to `// ERROR: member`, per C9) and the second of
   the 2 failures in the 3821/2 baseline.

Nothing *else* blocks it. In particular the scope half has no dependency on
G14, on the three-evaluator question, or on C6: S1/b/c and S4 are orderable
independently of all of them.

---

## Appendix — repro files

Under `temp/g17scope/` (gitignored). Promote the pairs named in §6 into
`lib/kestrel-test-suite/testdata/types/generics/` when their commit lands.

| file | shape | verdict at `29de8da8`+wt |
| --- | --- | --- |
| `assoc_shadow.ks` | method type param named `Item` under `extend Producer where Item: Show` | **leak live** — accepts |
| `assoc_shadow_control.ks` | same, param named `U` | correctly rejects |
| `shadow_accept.ks` | `struct Outer[T] where T: Show { func f[T] }`, uses inner `T` | leak visible but gated by `E439` |
| `shadow_reject.ks` | same, uses outer `T` via `self.v` | wrong-reject visible, gated by `E439` |
| `shadow_control.ks` | same, inner param renamed `U` | correctly rejects |
| `xrecv_member.ks` | `A.Item: Show` + `self.b.produce().show()` | **residual live** — frontend accepts, post-mono error |
| `xrecv_member_control.ks` | clause deleted | correct `E100` |

---

## S1 measurement — LANDED. Every estimate above is now replaced by a number.

**Base.** `pwd` `/Users/dino/Documents/Projects/kestrel`, branch `arch/fixes`,
no worktree. `git rev-parse HEAD` `1635e2d130f87cc6f5128ed13744a30f5819a443`
(`1635e2d1 docs(fragility): F12's wrong-accept is refuted…`) plus this commit's
probe. Tree clean apart from a pre-existing `rust-toolchain.toml` edit.
`cargo build --release --bin kestrel` clean.

**Method.** `KESTREL_DEBUG=audit-scope ./target/release/kestrel dump diagnostics
<file>`, one process per file, 8-way. **`dump diagnostics` equivalence to
`build` was verified before relying on it**, not assumed: on
`types/generics/same_param_multiple_separate_constraints.ks` the two paths
produce byte-identical probe output (3265 lines each), and on
`temp/g17scope/assoc_shadow_control.ks` both produce 3268.

### What was measured

The probe compares, for each `Bound` constraint the raw walker reads, the bound
set contributed to the queried entity under the **current** `body_owner`-scoped
resolution against a **per-holder** (`holder`-scoped) one. Both sides call the
*same* resolver — `where_clauses::resolve_type_entity` is
`WorldResolver::resolve_type_entity` with `context` as a parameter instead of
`self.body_owner` — so the only variable is the scope. Projections collapse on
both sides on purpose: this measures the **scope** half, not the key half.

**The brief's deviation from §6 is deliberate:** no `DeclaredWhereClausesOf`
split and no `clauses_in_force` extraction landed here. S1 compares *name
resolution*, not clause sets, so the Q1 implicit-injection problem does not
arise and the query does not need splitting yet. That stays S2's problem, and
S1's diff is a probe plus one `pub(crate)`.

### Corpus: `lib/kestrel-test-suite/testdata`, 3667 `.ks`

| | |
| --- | --- |
| total subject resolutions | **11,973,637** |
| `SAME` | 11,973,635 |
| `ADD` | **2** |
| `DROP` | **0** |
| `SWAP` | **0** |
| distinct files with ≥1 non-`SAME` | **2** |

### Floor vs contribution — the split that matters

The invariant floor is **3265 subject resolutions per file**, contributed by the
stdlib prelude, and **3422 of 3667 files sit exactly at it**. Unlike C1's
`audit-subject` floor, this one is a *volume* floor only:

> **The prelude's divergence floor is ZERO.** All 3265 baseline reads classify
> `SAME`. Every non-`SAME` in the corpus is per-file contribution.

So the raw "2 files diverge" is also the true per-file figure — 2 files out of
3667 contribute, and the 99.97% of reads that are prelude background contribute
nothing. Confirmed independently on an empty `module T` program: 3265 reads, 0
non-`SAME`.

### `lang/` — all 17 packages, 0 non-`SAME`

`clutch crypto datetime flock html-builder http jessup perch plume quill
quill-json quill-toml sdl std swoop talon-sqlite uuid`, each compiled as a unit.
`std` reads 6357 (3265 prelude + 3092 from compiling its own sources);
every other package sits between 3265 and 3387. **Zero divergences anywhere.**

### The `DROP shadowed=false` column

> ### **`DROP shadowed=false` = 0. `DROP` of any kind = 0.**
>
> Across 11,973,637 corpus resolutions plus all of `lang/`, the raw walk never
> finds a bound that per-holder resolution loses. §6's gate on S2 — "S2 does not
> land until that column is zero or every entry has an explanation" — **is met
> unconditionally**, with no entries to explain.

This inverts §6's risk ranking. S2 was ranked highest-risk on the grounds that
its reject-direction delta was "unbounded until S1 reports". It is now bounded,
and it is bounded at zero: there is no corpus program whose accepted bound the
scope fix removes. The two `ADD`s are permit-direction.

### The two corpus divergences — both are the F12 tests themselves

```
ADD kind=param subject=T asked=Holder.T holder=Test.Holder owner=Holder.f
    oldsubj=f.T newsubj=Holder.T old=[] new=[Test.Mapper] shadowed=true
ADD kind=param subject=T asked=Box.T   holder=Test.?(ext) owner=?.doMap
    oldsubj=doMap.T newsubj=Box.T old=[] new=[Test.Mapper] shadowed=true
```

- `types/generics/shadowed_type_param_does_not_borrow_assoc_type.ks`
- `types/generics/shadowed_type_param_in_extension_does_not_borrow_assoc_type.ks`

Both are `shadowed=true`, and both are the *outer* parameter silently **losing**
its `Mapper` bound while a method's own `[T]` is in scope — the wrong-**reject**
direction the ⚠ banner predicted, now measured. Neither file's `// ERROR`
annotations sit on the outer `T`, so S2 should leave both green; that is S2's
check, not a prediction made here.

### The A/B pair (§1, `temp/g17scope/`, outside the corpus)

| file | probe verdict |
| --- | --- |
| `assoc_shadow.ks` | **one `ADD` and one `DROP`, both `shadowed=true`** — `Producer.Item` gains `Show` back (`ADD`), the method's own `Item` gives it up (`DROP`) |
| `assoc_shadow_control.ks` | **all 3268 `SAME`**, zero divergence |

The pair behaves exactly as §1 describes: a *single* clause leaks in *both*
directions in the same file, and renaming the method parameter removes both
halves. Note this is the corpus's only `DROP` of any kind anywhere in this
measurement — and it is `shadowed=true`, i.e. a genuine F12 aliasing win, not a
name that fails to resolve in the holder's scope.
