# Stage 1 — Errors

Codes allocated 2026-06-09 (extending the stage-0.5 E480–E489 block; full
registry in `lib/kestrel-analyze/AGENTS.md`):

| plan name | code | home |
|---|---|---|
| E-REF-10 local escape | **E494** | MIR `verify::check_escapes` (coded `VerifyError`) |
| E-REF-11 mutable root | **E495** | MIR `verify::check_escapes` |
| E-REF-12 ambiguous source (decl half) | **E493** | analyze `decl/ref_return.rs` — free fns with ≥2 non-consuming params; METHODS root at the receiver (so `at(index:) -> &T` is legal). The expression half needs no hook: arms decay to owned copies, the merged value roots Local → E494 |
| E-REF-13 consuming self | **E496** | MIR `verify::check_escapes` |
| E-REF-15 ref across merge | **E497** | mir-lower `set_terminator` |
| E-REF-16 fn-as-value | **E491** | type-infer |
| E-REF-17 effect sugar | **E490** | hir-lower |
| E-REF-19 generic-arg leak | **E492** | type-infer |
| E-REF-20 mutating through `&` | **E207** | analyze `access_mode.rs` (E203–E206 family; `util::ref_place` is the single classifier, consulted FIRST — the receiver check accepts temporaries, so a shared-ref receiver would otherwise silently pass) |

The first four are the safety core.

| # | Trigger | Notes |
|---|---|---|
| E-REF-10 | returned ref roots at a **Local** | the escape error. Diagnostic points at the return expression *and* the local's definition — there is no `&` token to anchor on (`references-syntax.md` §2). Wording names the root and the line: "…borrows local `x`, which does not outlive the call; only parameter-rooted or `Pointer`-derived references can be returned. Pointer-derived references are not verified by the compiler." |
| E-REF-11 | `-> &mutating T` rooted at a non-mutable source | the const-cast guard (mutable-root rule, `references-gaps.md` §10.4) |
| E-REF-12 | multi-source return (provenance joins two eligible roots, incl. `if c { … } else { … }` result shapes) | "ambiguous borrow source; v1 supports a single parameter root" |
| E-REF-13 | `consuming` receiver returning a ref of `self` | self is destroyed at return |
| E-REF-14 | ~~named binding of a ref-typed value~~ **dissolved by Q8(a)**: `let x = peek();` *decays* (copy-out) instead of erroring (`semantics.md` §value contexts); `let r: &T = …` annotations are already the stage-0.5 type-position walk | number kept reserved in case the maintainer vetoes binding decay |
| E-REF-15 | ref live across a block merge / loop back-edge | replaces today's silent force-EndBorrow with a diagnostic for ref-typed values |
| E-REF-16 | ref-returning function used as a value / captured / stored | the `ret_borrow` ABI is not expressible in function types (`references-gaps.md` §5.1) — silent-miscompile backdoor |
| E-REF-17 | `throws -> &T`, or a ref inside any sugar wrapper (`Result`, `?`) | ref-in-enum-payload backdoor (§5.2) |
| E-REF-18 | ~~ref-typed `match` scrutinee~~ **dissolved by Q8(a)**: scrutinee is a value context → copy-out; a ref never enters match machinery | NotCopyable scrutinee errors via the copy guards, not a ref-specific code |
| E-REF-19 | `T := &U` at generic instantiation (inference-side guard at `kind_to_resolved`) | backstops the stage-0.5 annotation walk against *inferred* leakage (§5.3) |
| E-REF-20 | mutating use through a shared `&T` — `mutating` method, `mutating`-param argument, compound assign, setter base | the new shared-ref class in `classify_mutability` (access_mode.rs:249); sibling of E203-E206 and may be allocated in that family (E207) instead of the REF block |

Two Q8(a) wording extensions that are **not** new codes:

- The existing copy-guard / NotCopyable diagnostics fire on copy-out of a
  ref with a non-copyable, non-cloneable pointee (`let x = r;`,
  `consume(r)`, `match r`); their wording should name the reference, not
  just the type.
- E205 ("temporary passed to mutating parameter") must *not* fire for a
  `&mutating T`-typed call result — that is precisely the case E-REF-20's
  classifier arm makes Mutable.

PointerDerived note: `.value` / `.mutatingValue` results and their
propagation are **not** errors — that is the §10.3 inherited-contract model.
The checked/unchecked line is carried by E-REF-10's wording.
