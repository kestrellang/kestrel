# G14 + G17 + A16 — nothing owns "resolve a where-clause subject to a type"

`medium` · G14 = false reject (spurious `E454`) **and** false accept ·
G17/A15 = **unsound accept** · crates: `kestrel-type-infer`, `kestrel-analyze`

> **Status: diagnosed, not designed.** This file is verified ground truth.
> Open decisions live in [`decisions.md`](decisions.md) — that is the
> collaboration surface. Add findings here; add questions and answers there.

A where-clause subject has four spellings — `T`, `Self`, bare `Item`, and
`I.Item`. Resolving one to a concrete type is a single question, and no
function owns it. Three audit findings are three consequences of that:

| finding | subject shape | failure |
| --- | --- | --- |
| **G14** | any, on a *protocol* extension | substitution comes out empty; the clause is defaulted, not evaluated |
| **G17** (`A15`) | projection `I.Item` | base discarded; a bound on `A.Item` is granted to every `_.Item` in scope |
| **A16** | param `T` | bounds live on the parent decl, so the entailment tier that reads them is inert |

## Three evaluators, not two

The audit describes a two-way disagreement. There are three, and the one it
does not mention is the one that decides the common case:

| evaluator | location | default on an unmappable subject |
| --- | --- | --- |
| solver | `kestrel-type-infer/src/conformance.rs:313` `extension_bounds_hold_impl` | **permit** (`:366`) |
| analyzer, cross-protocol witness | `kestrel-analyze/src/compilation/conformance_completeness.rs:1625` `extension_clauses_entailed` | **reject** |
| analyzer, same-protocol default | `conformance_completeness.rs:266-271` + `:369` | **permit — where clauses are never read at all** |

`check_protocol_requirements` builds `default_methods` from every
`ProtocolMembers` entry with `member.extension.is_some()`, with no clause
evaluation, and `:369` short-circuits `E454` on a name+signature+receiver match
(`protocol_default_method_matches`, `:753-766`).

**Any unification that touches only the first two is theater** — the third
still permits unconditionally on the path most programs take.

## Three corrections to the audit

### 1. §A13's failure scenario does not reproduce

Built verbatim (`protocol Base { type Item; … }` + `extend Base where Item:
Equatable` + `struct Counter: Base { type Item = Int64 }`): clean build,
exit 0, no `E454`. The third evaluator masks it — requirement and constrained
extension belong to the same protocol, so `:369` matches on signature and
neither of the other two evaluators is consulted.

**The real `E454` shape is the cross-protocol witness** (the `#213` shape):
the extension member must *witness a requirement of a different protocol*.

```kestrel
extend Counter: Equatable { }   // ← E454 here, on a legal program
```

Rewrite §A13's scenario before it becomes a test.

### 2. G14 is not about bare associated types

`extension_bounds_hold` (`conformance.rs:198-207`) zips
`LowerExtensionTargetTypeArgs` — the *protocol's* type-arg positions — against
`hir_args(recv)` — the *conformer's own* args (`:440-449`). That is a category
error whenever conformer arity ≠ protocol arity, which is the common case.

A plain `TypeParameter` subject fails identically:

```kestrel
protocol Container[T] { func item() -> T }
extend Container[T] where T: Equatable { … }
struct BoxC: Container[NotEq] { … }   // no type params of its own
```

`hir_args(BoxC)` is `[]` → `zip([Param(T)], [])` → empty subst →
`continue // Unknown param — permit`. Bare-assoc is **one instance, not the
class**.

The only shape the solver actually evaluates on a protocol extension is
`where Self: Q`, via `Some(*param) == target_entity` (`conformance.rs:363`).
That branch is correct — the shipped negative test still rejects.

`conformance.rs:245-253` already documents this hole for blanket extensions.
Same defect, one case wider.

### 3. A16's own correction is wrong

A16 argues its tier 2 is live because an associated type gets an
`AstWhereClause`. The ordinary bound `type Item: Equatable` is stored as
**`Conformances`**, not `AstWhereClause` (`kestrel-ast-builder/src/builders/type_alias.rs:80-83`);
only a rare trailing `where` on the alias produces one. Tier 2 is inert for the
common spelling too. Only a `Self` subject resolving to a protocol that carries
its own `where` reaches it.

## What is live vs latent

**Live, shipped stdlib — the solver path.** All five
`extend Iterator where <assoc>: P` sites: `lang/std/iter/iterator.ks:846`
(`contains`), `:866` (`min`), `:1032` (`sum`), `:1049` (`product`), `:1071`
(`flatten`). Calling `.contains()` on an iterator whose `Item` is not
`Equatable` is accepted by the frontend and fails post-mono, **pointing at
stdlib source with no user-code span**:

```
error: Call: type 'Test.NotEq' does not implement 'isEqual' required by 'std.core.Equatable'
    ┌─ lang/std/iter/iterator.ks:857:37
error: unsupported: post-mono verification failed with 1 error(s)
```

**Live — G17/A15.** With `A.Item: Equatable` present, an unrelated unbounded
`B.Item` type-checks. Delete that one clause and `E100` correctly fires
(`B.Item !: Equatable`). **Adding a bound to one type makes a different type
compile.** Unsound accept.

**Latent — the analyzer path.** Zero stdlib members are dropped by G14 today;
no bodyless `contains`/`min`/`max`/`sum`/`product`/`flatten` is a protocol
requirement anywhere in `lang/std`.

**Latent — the `where Self: Q` twin.** Five shipped testdata files
(`declarations/extensions/constrained_protocol_extension_applies.ks:11`,
`protocol_extension_calls_constraint_method.ks:11`,
`more_constrained_extension_wins.ks:16`,
`multiple_constraints_more_specific.ks:19`,
`protocol_extension_multiple_where_clauses.ks:14`) supply inherent members that
witness nothing. Make one witness a requirement and `E454` fires.
`protocol_extension_mixed_self_constraints.ks:15` (`Self.Item: Equatable`) is
doubly latent — never called *and* never witnessing.

## Machinery that already exists and is directly reusable

`type_compare_env_for_conformance` (`conformance_completeness.rs:1048-1087`)
already builds the map a unified binder needs:

```
ProtocolAssociatedTypes { protocol }
  → find_associated_type_binding_entity(cx, type_entity, &name, declaring_protocol)   // :1281
  → LowerTypeAnnotation { entity: binding }                                           // ty.rs:938
  → AssocBinding { assoc: member.entity, name, ty }                                   // compare.rs:31-35
```

Three properties that matter:

- `AssocBinding.assoc` is **the protocol's `TypeAlias` entity** — exactly the
  `param` entity `WhereClausesOf` yields for a bare-assoc subject. Entity-keyed
  lookup, no name matching.
- `AssocBinding.ty` is already `HirTy` — the representation
  `extension_bounds_hold_impl` wants. No conversion.
- `find_associated_type_binding_entity` handles qualified vs unqualified
  bindings, walks extensions and the conformed-protocol closure, and falls back
  to the protocol's own default (`:1073-1078`).

It is called only from the signature-comparison path (`:941`, `:964`, `:1874`),
never from member provision. **The gap is wiring, not a missing lookup.**

## Two hard constraints on any design

### It must be a plain depth-threaded function, not a memoized query

`conformance.rs` contains **no `impl QueryFn` at all**; every function is a
plain `fn` over `&QueryContext<'_>` and `depth` is an argument. Memoizing forces
a choice between keying on `depth` — which defeats memoization, since the same
`(ty, protocol)` pair is asked at many depths — and dropping it, which removes
the only cycle guard. That guard's fallback is `return true` (`:69-71`), so
losing it means **non-termination, not a wrong answer**.

Two recursion axes already meet here (`:57-61`): structural
(`nominal_satisfies` → `extension_bounds_hold_impl` → `type_satisfies_at_depth`
at `:368`) and refinement (parent-protocol recursion at `:302`). A binder closes
a third loop through the same counter.

*Aside:* `mono/witness.rs:373` caps at 16 while `conformance.rs` caps at 32. If
the two ever need to agree, that is a separate mismatch.

### `resolved_ty_to_hir` is safe but weak — and that ceiling should be stated up front

`conformance_completeness.rs:1604` collapses every abstract position to
`HirTy::Infer`, and `type_satisfies` permits `Infer` unconditionally
(`conformance.rs:186`). It is nonetheless *safe*, by a non-obvious mechanism:
`self_type_for_compare` (`:1093`) yields `HirTy::Struct { args: [Infer, …] }`;
`nominal_satisfies` keys `ConformingProtocols` on the entity (unaffected by
`Infer`), and extension selection runs `target_args_apply` → `hir_ty_matches`,
whose catch-all is `_ => false` (`:430`) — so an `Infer` arg **excludes** a
specialized extension rather than wrongly selecting one. With
`best == None && parent_protocols.is_empty()`, `:310` returns `true`.

Net: `Box[T]` → `Box[Infer]` degrades to today's permit. **No new wrong-rejects,
and no new correctness for generic conformers.** That is the honest ceiling.

## Substitution-builder inventory

26 consumers build subject→type substitutions. Grouped by miss policy:

- **permit / silently drop** — `conformance.rs:332`, `:361-367`;
  `solver.rs:3524`, `:4418`, `:5859`; `generate.rs:1921`, `:1993`;
  `lib.rs:666`, `:969` (mints a fresh *unconstrained* `TyVar` on a miss,
  `:1000`); `mono/collect.rs:603-640`; `mono/mod.rs:255`;
  `mono/witness.rs:364` (`None => true`)
- **reject** — `entailment.rs:62`; `conformance_completeness.rs:1662`
  `substitute_clause`; `resolve.rs:589` `conforms_to`'s `AssocProjection` arm
- **reject with a diagnostic** — `analyze/decl/generics.rs:483` (`E437`/`E440`),
  which matches subjects by **name string** and does not recognise `Self`
- **subject discarded entirely** — `lib.rs:912`, `:931`; `solver.rs:5768`,
  `:5783`: the clause is applied to the alias `TyVar` regardless of what it
  constrained

The canonical resolver, `WhereClausesOf` (`where_clauses.rs:35`/`:51`), **drops
a clause it cannot resolve** (`:71-73`), so downstream cannot distinguish a
dropped clause from an absent one. `resolve_projection_subject` (`:275`)
requires the base to be a `TypeParameter` (`:294-299`), so `Self.Item: P`
(shipped at `protocol_extension_mixed_self_constraints.ks:15`) falls through to
`resolve_type_entity` and **collapses to bare-assoc** — G14's shape.

Also bypassing `WhereClausesOf` with their own raw `AstWhereClause` walks:
`kestrel-semantics/src/lib.rs:304`, `:447`; `staticness.rs:159`;
`kestrel-mir-lower/src/items/function_sig.rs:320` (drops `Self` subjects at
`:413`, silently removing the clause from the MIR signature);
`kestrel-analyze/src/body/move_tracking.rs:1968`; `decl/generics.rs:488`;
`kestrel-name-res/src/resolve_type.rs:604` and `resolve_value.rs:583` (match
`Self` by **raw string compare**).

## Repros

`temp/g14/` (untracked; regenerate if pruned). Build with
`cargo build --release --bin kestrel` — the `kestrel` binary is owned by the
root crate.

| file | shape | observed |
| --- | --- | --- |
| `a_assoc_subject.ks` | §A13 verbatim, satisfiable | clean, exit 0 — **no E454** |
| `b_assoc_subject_unsat.ks` | §A13, unsatisfiable, body unused | clean, exit 0 — wrong accept |
| `c_assoc_witness.ks` | cross-protocol witness, satisfiable | **E454 on a legal program** |
| `d_solver_permit.ks` | same extension, unsatisfiable, direct call | clean, exit 0 — wrong accept |
| `e_self_q.ks` | `where Self: Q`, member witnesses `Equatable` | **E454** — twin is live in the witness shape |
| `h_selfq_neg.ks` | shipped negative — byte-identical to `testdata/declarations/extensions/unconstrained_protocol_extension_not_found_when_constraint_not_met.ks`, so the suite covers it [added 2026-08-23, C9] | correctly rejects |
| `g2.ks` | **`TypeParameter`** subject, unsatisfiable, body *uses* the bound | wrong accept → post-mono failure |
| `g_param_subject_control.ks` | same, body does **not** use the bound | **clean build, exit 0 — no diagnostic at any stage** |
| `f_stdlib_iter.ks` | shipped `extend Iterator where Item: Equatable` | wrong accept → post-mono, span in stdlib |
| `i_a15_projection.ks`, `i2_control.ks` | G17 projection collapse | wrong accept; control gives correct `E100` |
| ~~`control.ks`, `leak.ks`~~ | superseded | **UNRUNNABLE — invalid when written.** Both use `func get()`; `get` is a lexer keyword (`kestrel-lexer/src/lib.rs:588`), so the protocol never parses and everything after is cascade. Not a compiler change. The author's own `leak2`–`leak5`/`control2` replacements use `produce()`; no audit claim cites these two |

**Corpus re-verified in full at `296e3076`: 22 CONFIRMED, 1 DIVERGED
(`g_param_subject_control`, split above), 2 UNRUNNABLE (`control`, `leak`).**
Both findings survive; G14's stdlib-anchored span still lands at
`lang/std/iter/iterator.ks:857:37` exactly as documented.

### What the divergence means

`g_param_subject_control.ks` and `g2.ks` differ **only** in the extension body —
`g2` writes `self.item() == other.item()`, the control returns `true`. Post-mono
only fires if the body actually *uses* the unsatisfied bound.

So G14 is **worse** than filed, not better: a `TypeParameter`-subject wrong
accept whose body never exercises the bound reaches a **shipped binary with no
diagnostic at any stage**. The post-mono error everyone has been treating as
G14's safety net is contingent on the body, not guaranteed.

## Inferred, not observed — do not treat as verified

- The same-arity generic conformer case (`struct Box[T]: Container[T]`), where
  the zip would accidentally line up. Read from `hir_args`, not run.
- Whether any *stdlib* `Iterator` conformer ships a non-`Equatable` `Item`
  reachable from `.contains()`. Stdlib compiles clean today.
- A16: its stated counter-example is refuted above, but no live wrong answer was
  constructed from tier 2 being inert. Consistent with its "low / latent" rating.
- `resolve_conformance_type_arg` (`conformance_completeness.rs:1204`) recurses
  **uncapped**. Surfaced in the sweep, not triggered. Out of scope here, but it
  sits directly in the substitution path a unified binder would use.

---

# Second pass (2026-08-20) — G17 is a silent miscompile, not an unsound accept

> ## ⚠ Provenance — read before citing anything below
>
> This section has **two sources with different trust levels**, and they were
> conflated when first written. Corrected same day.
>
> **VERIFIED on `arch/fixes`** — run by the orchestrator against
> `target/release/kestrel` built from the parent checkout at `HEAD`:
> the miscompile (`leak5.ks`), both A/B controls
> (`distinct_samename`/`distinct`, `structcase`/`structcase_nobound`), and the
> three stdlib projection-bound sites. **These stand.** G17's severity re-rating
> rests only on these.
>
> **MEASURED ON `v0.16.0`, NOT THIS BRANCH — now adjudicated.** The probe ran
> in `.claude/worktrees/agent-*`, pinned at `789bb779` = `v0.16.0` = `main`,
> **173 commits behind** (audit finding **G24**). Every claim it made has since
> been re-checked against source at `296e3076`:
>
> | probe claim | verdict at HEAD |
> | --- | --- |
> | `where_clause_assoc_subs` keyed base-free, incl. the `solver.rs:2836` cross-protocol **name** fallback | **CONFIRMED** — all six cited sites exact |
> | `conforms_to`'s `AssocProjection` arm discards the base; `WorldResolver` has no `InferCtx` to resolve it | **CONFIRMED** — and `resolve.rs:589` is right, `:512` was the stale line |
> | the fabrication chain through `get_or_create_subject_tv` | **CONFIRMED mechanism, MOVED to `lib.rs:969-1000` (re-base at `:991`) — but its example is WRONG**: it emits a *depth-2* projection rooted at `Self`, not `I.Item.Item`. No depth-3 shape is produced |
> | `param_tyvars` aliasing via `emit_where_clauses` (`lib.rs:314`) | **REFUTED** — no such function exists at HEAD *or* at `v0.16.0`; `lib.rs:314` is a comment. The `T.Assoc` path was fixed by #184; the aliasing moved into `where_clause_assoc_subs` (row 1) |
> | seven `ProjectionBound` match sites, four of them bugs | **REFUTED (counts)** — there are **8** sites and **6** no-ops; it missed `lib.rs:958` and `solver.rs:5798` |
> | the 6124-fire sweep | **VOID** — `ProjectionBound` and `resolve_projection_subject` **did not exist at `v0.16.0`**, so the sweep measured a compiler without the feature. Says nothing about this branch |
> | strict-mode suite green (3062) | **still unverified** — needs a `triage` run here |
>
> **The root cause of every wrong Group-B claim is one fact:** projection
> subjects were unconditionally collapsed at `v0.16.0` because the machinery
> under test had not been written yet.
>
> **VERIFIED on `arch/fixes`** [@ `296e3076`] — the behavioural corpus, re-run
> in full: 22 confirmed, 1 diverged, 2 invalid-when-written. The miscompile and
> both A/B controls stand. G17's severity rests only on these.

Everything in the VERIFIED group was **run**, not read. Binary
`target/release/kestrel` from the parent checkout, repros in `scratchpad/g17/`
(untracked).

## G17 emits wrong code

```
leak5.ks  →  builds clean, exit 0, prints  result=int:<heap pointer>
```

> **Do not cite a specific number here.** The value is a heap address under
> ASLR and differs every run (`4352464672`, `4373960480`, … all observed for
> the same binary). Two agents reporting "the same" repro with different
> numbers should have been the tell that this was a pointer, not a value. Any
> regression test for G17 must assert **that the build is rejected**, never
> what the miscompiled program prints.

`bad[A, B](…) where A: Producer, B: Producer, A.Item: Show` calls
`needsShow(b.produce())`. `B.Item` is `String`, which has no `Show` witness.
The clause on `A.Item` makes `B.Item` type as `A.Item` (`Int64`), mono selects
`Int64`'s witness, and the `String` pointer is reinterpreted as an integer.
**Type confusion in a shipped binary**, not a missing diagnostic.

Re-rate G17 `medium` → `high`. It is the only silent miscompile left in the
audit.

| repro | shape | observed |
| --- | --- | --- |
| `leak5.ks` | `A.Item: Show`, call on `B.Item` | **clean build, garbage output** |
| `distinct_samename.ks` | two *unrelated* protocols, both aliases named `Item` | **clean build** — wrong accept |
| `distinct.ks` | same, aliases renamed `ItemA`/`ItemB` | correct `E100` |
| `structcase.ks` | container-level `A.Item: Show` | frontend accepts, post-mono failure |
| `structcase_nobound.ks` | same, clause deleted | correct `E100` |

Two independent statements of the bug, both verified: **renaming an unrelated
protocol's associated type changes whether your program compiles**, and
**adding a constraint silences a correct error**.

## Two independent aliasing mechanisms — corrected

[verified @ `296e3076`] An earlier draft claimed three. `param_tyvars` was one
of them and is **refuted**: no `emit_where_clauses` exists at HEAD, and
`lib.rs:314` is a comment. The `T.Assoc` path it described was fixed by #184
(`lib.rs:447-461`, documented in-source). Two remain, and the first is the one
that miscompiles:

1. **`where_clause_assoc_subs`** — a `Vec<(Entity, TyVar)>` keyed on the
   **assoc entity alone** (`ctx.rs:178`), pushed at `lib.rs:461` with the base
   dropped. Six reads ignore the base — `generate.rs:2313`/`:2314`,
   `solver.rs:2822`, `:3907`, `:6015`/`:6016` — two with an explicit
   `let _ = base_tv;`. Worst, **`solver.rs:2836`** falls back to matching on
   **`Name` equality across different protocols**, which is the
   `distinct_samename.ks` result. `find()` returns first, so two subjects
   sharing an assoc entity collide.
2. **`collect_assoc_type_protocol_bounds(assoc)`** — the `conforms_to` arm at
   `resolve.rs:589`, base discarded. `base` is a `TyVar` and `WorldResolver`
   (`resolve.rs:214-218`) holds no `InferCtx`, so the arm is *structurally
   incapable* of comparing bases where it currently sits.

**A fix that addresses only (2) leaves the miscompile intact** — and so does
D7's `WhereSubject` on its own, because (1) is a *key* defect, not a
*representation* defect. See the scope limit in `decisions.md` D7.

## The collapse fabricates the obligation it then discharges

> **Provenance** [checked @ `296e3076`]: the *mechanism* below is CONFIRMED but
> has **MOVED** to `lib.rs:969-1000` (re-base at `:991`). Its **example is
> wrong** — `ctx.associated(self_tv, …)` builds a *depth-2* projection rooted
> at `Self`, not `I.Item.Item`; no depth-3 shape is produced on this path. The
> 6124-fire sweep quoted below is **VOID**: it ran on `v0.16.0`, where
> `ProjectionBound` and `resolve_projection_subject` **did not exist at all**,
> so it measured a compiler without the feature under test. Re-run it here
> before quoting a number.

~~Instrumented sweep over all testdata: **6124 fires of the arm, 6124 leaky, 0
legitimate** — exactly two per compilation, all from one stdlib site, none
from any test file.~~ Traced:

1. `adapters.ks:668` declares `where I: Iterator, I.Item: Iterator`
2. `resolve_where_clauses` collapses the subject to
   `Bound { param: Iterator.Item }` — base `I` gone
3. `get_or_create_subject_tv` (`lib.rs:811-843`) receives a bare alias, cannot
   tell what it was based on, and **re-bases it onto `Self`** via
   `ctx.associated(self_tv, …)`
4. For `FlattenIterator`, `Self.Item` *is* `I.Item.Item` — so the obligation
   becomes `I.Item.Item: Iterator`, a claim the source never makes
5. the base-blind arm discharges it, because the fabricated projection's
   `assoc` is the entity the clause collapsed onto

**The arm's entire production use is discharging an obligation the same
base-blindness invented two passes earlier.** Requiring a matching base costs
nothing real.

Note this produces a **depth-3** projection, which
`resolve_projection_subject` cannot even represent (`segments.len() != 2`).

## Correction to the first pass

> "`grep '\.Item:' lang/std` finds exactly one projection bound"

Wrong. There are three, and any fix must account for all of them:

```
adapters.ks:397  PeekableIterator[I]    … I: not Copyable, I.Item: Copyable
adapters.ks:666  FlattenIterator[I]     … I.Item: Iterator, I.Item: not Copyable
adapters.ks:866  IntersperseIterator[I] … I: not Copyable, I.Item: Copyable
```

The *conclusion* survives — `Copyable`/`Cloneable` are answered structurally at
the top of `conforms_to`, before the arm — but "only one exists" is false.

## Two findings that needed their own IDs — one was a fourth staleness artifact

~~**One unrenderable diagnostic destroys every diagnostic.**~~ **STRUCK
2026-08-20 — not live at HEAD, never filed.** The claim was that
`Span::synthetic(0)` → `FileMissing` → the `?` in `emit_all` aborts the whole
loop → `kestrel build` exits 1 with zero output. `emit_all` no longer `?`s;
HEAD has `emit_one`, which strips labels and retries on any span-lookup error.
Observed working at `296e3076`: `e_self_q.ks` and `h_selfq_neg.ks` each print
a spanless `E100` rendered as *"(no source location available…)"* **alongside**
their other diagnostics — nothing is swallowed. `emit_one` does not exist in
`v0.16.0`, so this is the same worktree-staleness as the withdrawn G22
(finding **G24**) — the fourth artifact from it.

**The suite cannot see stdlib-anchored diagnostics.** Independent of the above
and **still open** — filed as **G23**. The `file_id`-anchored matcher
(`diagnostic_matcher.rs:191`) discards a diagnostic anchored in `lang/std`, and
execution tests still codegen and run. Sibling of G19: a genuine regression
here ships green. Note this one needs a *suite run* to demonstrate, not a repro
build, so it remains unverified on this branch.

## Unverified — do not rely on

- **RESOLVED: the 3062-vs-3815 discrepancy was staleness.** The probe reported
  3062 passing; the full suite ran 3815 the same day. Cause: the probe's
  worktree is `v0.16.0`, which has fewer testdata files. The "suite is green
  under strict" claim is therefore about a 10-week-old compiler and says
  nothing about this branch. Re-run before leaning on it — it is the claim
  that makes this fix look cheap.
- **The "only one stdlib projection bound" claim was also staleness**, not a
  bad grep: `v0.16.0`'s `adapters.ks` differs from this branch's, which has
  three (`:397`, `:666`, `:866`, verified here).
- `Iterator.flatten()` reported as already unusable in-tree (mangler ICE on
  `AssociatedProjection`, `mono/mangle.rs:229`), independent of any change
  here. Not reproduced by me.
