# G13 — decisions

## 1. Fix (b) — teach `type_satisfies` to answer — not fix (a), duplicate the skip

Two fixes killed the E454.

**(a)** Copy the `is_copy_builtin` skip to the analyze site
(`conformance_completeness.rs:1642-1649`), so it stops asking about Copyable.
Two lines, no blast radius.

**(b)** Give `type_satisfies` a real answer for Copyable/Cloneable, and delete
the skip.

**Decision: (b).** (a) was rejected because it treats the false reject as the
whole bug when it is the *smaller* half. The two bugs share one cause — nobody
could answer "does this type satisfy `Copyable`?" through `type_satisfies` — and
(a) resolves that by agreeing never to ask. The immediate cost is that Bug 2
stops being a bug and becomes the language rule: `where T: Copyable` on an
extension would be permanently, by construction, unenforced, with
`RcBox[NC].getValue()` compiling to a SIGILL forever. The stdlib would keep
documenting an invariant (`rcbox.ks:180-183`, "only a box over a non-Copyable
payload loses these two methods") that the compiler does not implement.

The deeper objection is that (a) makes the divergence permanent. The solver
already answers this question correctly and routes `a.dup()` through the
extension; (a) leaves the analyzer answering a *different* question and calls it
agreement. (b) makes both read the same classifier, so there is one story.

## 2. A top-level arm, not a new case inside `match ty`

The arm sits between the `Builtin::Static` arm and the `match ty`, at the same
position class as `Static` — deliberately **not** inside the match.

Every arm of `match ty` funnels into `nominal_satisfies` →
`ConformingProtocols`, which is the wrong oracle for a structural builtin: it
reports declared conformances, and `Copyable` is never declared. Adding a
Copyable case *inside* would mean patching `Struct`, `Enum`, `Protocol`, `Ref`,
`Tuple`, `Never` and the catch-all separately, and getting the same answer seven
times. A top-level arm subsumes all of them, including the `HirTy::Struct` shape
the E454 repro actually hits.

`Static` is the precedent and the proof that this position class is right: it is
structural for the same reason and was already lifted out for the same reason.

## 3. The abstract-position permit, and the three distinct reasons for it

Checked **first**, before delegating to the classifier:

```rust
HirTy::Param(..) | HirTy::SelfType(..) | HirTy::AssocProjection { .. }
    | HirTy::Opaque { .. } | HirTy::Infer(..) | HirTy::Error(..)  =>  true
```

This is not one reason applied six times. It is three.

**(i) Contract preservation — `Param`, `Opaque`.** This module's documented
contract (`conformance.rs:15-25`) is "reject only on a provable *concrete*
violation; permit every abstract position", and the existing catch-all at the
bottom of `match ty` already buckets exactly these as permit for every other
protocol. `HirCopyLayer` does **not** share that contract: it can return a
definite `NotCopyable` for a `not Copyable`-bounded `Param`, or for a
`some P and not Copyable` `Opaque`. Delegating without the guard would import a
stricter contract into a query that callers rely on being conservative, and
would spuriously reject generic bodies whose bound is satisfied abstractly.
Per-instantiation precision is the solver's job (`type_conforms_copyable`), not
this best-effort completeness gate's.

**(ii) A resolution mismatch — `SelfType`.** Different problem entirely.
`HirCopyLayer::member_semantics` resolves `SelfType` to the **declaring**
entity's nominal semantics. Inside a protocol-extension body the declaring
entity is the *protocol*, not the eventual conformer, so the classifier would be
answering about the wrong type. `lang/std/memory/cowbox.ks:52` depends on this.

**(iii) Nothing to answer — `AssocProjection`, `Infer`, `Error`.** No type is
known yet (or ever will be, for `Error`). Rejecting on absence of information is
the incompleteness-rejection this module exists to avoid.

`root` is passed as the `context` argument, mirroring the `Static` arm.
Safe because `TypeParamCopyRequirement::execute`
(`kestrel-semantics/src/lib.rs:289-296`) pushes `parent_of(self.param)` before
walking `self.context`, so the param's own declaring parent is consulted first
regardless of what context is handed in.

The predicate is copied verbatim from `TypeResolver::conforms_to`
(`resolve.rs:511-524`) — `Copyable` iff `sem != NotCopyable`, `Cloneable` iff
`sem == Cloneable` — so the analyzer and the solver cannot drift.

## 4. USER-FACING BEHAVIOR CHANGE — flag for the maintainer

**This turns a runtime SIGILL into a compile-time diagnostic on public stdlib
surface.** Code that compiled yesterday will not compile today. It was never
code that *worked* — it trapped with exit 132 — but the failure moved from run
time to build time, and that is a visible break, not a silent improvement.

Newly rejected:

| surface | member | on |
|---|---|---|
| `RcBox[T]` (`rcbox.ks:183`) | `getValue`, `deepClone` | a `not Copyable` payload |
| `Pointer[T]` (`pointer.ks:351`) | `pointee` | a `not Copyable` pointee |

Both are the documented intent of the bounds. Neither had a working behavior to
preserve. The full suite showed no other program changed status.

Deliberately **not** affected, and verified: `core/error.ks:25`,
`result/optional.ks:543,656`, `result/result.ks:447` are
`extend X: Copyable where …` — extensions that *add* the conformance. Their
per-instantiation copy-ness is computed by `ConditionalCopyableParams` /
`instance_semantics`, a disjoint mechanism that never consults
`extension_bounds_hold`. `collections/set.ks:1290` (`deepClone` gated on
`T: Cloneable`) and `memory/cowbox.ks:112` also still work — the latter via the
`SelfType`/`Param` permit above.

**One residual gap, pre-existing and separately tracked.** Reaching a
`where T: Copyable`-gated member through a *generic protocol bound* fails at
mono:

```
error: Call: type 'Test.Cell[Int64]' does not implement 'one' required by 'Test.HasOne'
       (no matching conformance for this instantiation)
```

`type_conforms_at_mono` (`kestrel-mir/src/mono/witness.rs:395`) evaluates a
witness's `where` constraint by searching the witness table for a witness whose
protocol is `Copyable` — and Copyable, being copy-semantics rather than a
declared conformance, has no witnesses. So the constraint is unsatisfiable
there by construction. This is mono's own answer to the same question, on a code
path G13 does not touch; the `where T: Equatable` spelling of the identical
program compiles and runs. Pinned as a documented exclusion in
`declarations/extensions/copyable_bound_permits_abstract_receiver_arg.ks`.

## 5. `entailment.rs` — a comment, no logic

`bound_entailed` needs no Copyable case, and adding one would be a bug. Unlike
`type_satisfies` it never touches `ConformingProtocols`: it does plain `Entity`
containment over the protocols named by `WhereClause::Bound` nodes, and
`expand_protocol_closure` seeds its output with the input set
(`conformances.rs:271-274`). So a context clause `where T: Copyable` matches a
target clause `where T: Copyable` by protocol-entity identity, and the question
"does `T` *declare* Copyable?" — the one `ConformingProtocols` answers wrongly
for structural builtins — is never asked.

That makes the path sound **by construction, not incidentally**, which is the
whole reason to write it down. The failure mode being insured against is
concrete: someone reads the new `type_satisfies` arm, notices entailment has no
matching case, infers symmetry, and pastes an `is_copy_builtin`-style skip in —
reopening a Bug-2-shaped hole in the one evaluator that was always correct. The
comment names that trap.

## 6. Eight answers to "is this Copyable" — fold, don't add a ninth

The diagnosis found the question answered independently in eight places:

| # | where | over |
|---|---|---|
| 1 | `HirCopyLayer` — `kestrel-semantics/src/lib.rs:585` | `HirTy` |
| 2 | solver layer + `type_conforms_copyable` — `kestrel-type-infer/src/solver.rs:2490,2582` | `TyVar` |
| 3 | `TypeResolver::copy_semantics_of` / `conforms_to` — `kestrel-type-infer/src/resolve.rs:2122,511` | `TyKind` |
| 4 | move tracking — `kestrel-analyze/src/body/move_tracking.rs:1844` | `ResolvedTy` |
| 5 | MIR `ty_query` — `kestrel-mir/src/ty_query.rs:71` | `TyId` |
| 6 | mono layer — `kestrel-mir/src/mono/mod.rs:1055` | mono `TyId` |
| 7 | `type_conforms_at_mono` — `kestrel-mir/src/mono/witness.rs:395` | witness table |
| 8 | `is_copy_builtin` skip — `conformance.rs:298` | *nothing* — answered "yes, always" |

1–6 are `CopyLayer` implementations over different type representations, sharing
one decision tree through `kestrel-copy-fold`'s `instance_semantics`. That is
the sanctioned family: a layer per representation, one algorithm. 7 and 8 are
the outliers — 7 asks a structurally unanswerable question, 8 declined to ask.

The fix removes #8 and routes `type_satisfies` through #1 instead of inventing a
ninth. This matters more than the line count suggests: `type_satisfies` is
consumed by both the analyzer and the solver, so a bespoke answer here would
have been a *ninth* opinion sitting between two existing ones. Delegating to
`HirCopyLayer` — and copying `conforms_to`'s predicate verbatim rather than
re-deriving it — means the count went from eight to seven, not eight to nine.

#7 is left alone on purpose. Fixing it means teaching the mono witness selector
that two protocols are structural and have no witnesses, which is a mono-side
change with its own blast radius; folding it into this one would have bundled
two unrelated risks into a single change.
