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
| `h_selfq_neg.ks` | shipped negative | correctly rejects |
| `g_param_subject_control.ks`, `g2.ks` | **`TypeParameter`** subject, unsatisfiable | wrong accept → post-mono failure |
| `f_stdlib_iter.ks` | shipped `extend Iterator where Item: Equatable` | wrong accept → post-mono, span in stdlib |
| `i_a15_projection.ks`, `i2_control.ks` | G17 projection collapse | wrong accept; control gives correct `E100` |

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
