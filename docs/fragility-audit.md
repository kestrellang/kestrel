# Kestrel Fragility Audit — Tracking Checklist

Status of every confirmed finding from the 2026-08-08 fragility / single-source-of-truth audit.
**This file is the source of truth for what is fixed.** Update it in the same commit as the fix.

- Full findings with evidence and repro: [`fragility-audit-detail.md`](fragility-audit-detail.md) (1-43)
- Gap round + high-severity verdicts: [`fragility-audit-addendum.md`](fragility-audit-addendum.md) (G1-G17)
- Per-finding diagnosis + decision records for the harder ones: [`fragility/`](fragility/)

**Method.** 18 auditors across crate clusters and 6 cross-cutting dimensions, each finding
adversarially re-verified by an independent agent; then a completeness critic and 4 targeted
follow-ups. 118 candidates filed, 107 confirmed, 11 refuted. Findings 1-43 are the deduped
merge of 90 confirmed; G1-G17 are the gap round.

**Legend.** `fixed` done and verified · `partial` some sub-items done · `blocked` needs a
maintainer decision · `open` untouched. Severity is post-verification (verifiers corrected
inflated finder severities).

**Progress: 42 fixed · 3 partial · 3 blocked · 18 open** — 64 top-level (F1–F43, G1–G21).
F33 and F43 are roll-ups that expand into 19 independently-fixable sub-items, tracked
underneath them, so the real work item count is 79.

Completed items live in [**Fixed**](#fixed) at the bottom of this file.

## Blocked on a maintainer decision

These are not "unstarted" — they were investigated and the obvious fix is wrong.

- **F42 — `unsafe impl Sync for StdlibCache`.** A `Mutex` does **not** fix it. `World::snapshot()`
  clones query memos holding rowan CST nodes, whose refcounts are non-atomic and shared with the
  cache; a snapshot outlives any lock, so one thread's drop races another's snapshot. Confirmed
  structural: adding `Send + Sync` to `QueryFn` compiles workspace-wide, but adding it to
  `QueryFn::Output` fails on `ParseResult` (`NonNull<rowan::cursor::NodeData>`). Live, not
  theoretical — `libtest_mimic` runs trials as threads in one process and triage batches many tests
  per process. Options: process-per-test, exclude CST-bearing memos from snapshots, single-thread
  per process, or a thread-local cache. All have real costs.
- **F29 — E479 for `@builtin` argument validation.** Seven testdata files already exist for
  malformed/typo'd `@builtin` (`@builtin()`, `@builtin(42)`, `@builtin(.Copable)`, …), all
  `// test: diagnostics` with **zero `// ERROR:` annotations** — the diagnostic was designed and
  never built. `kestrel-analyze/AGENTS.md` says next free code is E479. Adding it means updating
  those seven files.
- ~~**F3**~~ — **resolved 2026-08-08.** The classification is stored, not derived (`FieldClass`, set
  by the field builder), and a bodyless `{ get set }` on a concrete type is **storage**. See the
  design doc's "Decisions" section.
- **F11 — `TypeParamCopyRequirement`'s `context`**: changes which programs are accepted. See the
  detail doc's "Tier 3" section.

## Keeping this current

- Update the checkbox and add a one-line note **in the same commit as the fix**.
- Do not delete entries — mark them `fixed` and move them to the **Fixed** section at the bottom.
  The ID is how other agents and commits refer to them.
- If verification disproves a finding, move it to **Ruled out** with the evidence, don't silently drop it.
- Severities here are post-verification. If you re-measure one, correct it and say so under
  **Corrections**, as was done for F41 and F28.

---


## Silent miscompilation and wrong behavior

- [ ] **F1** `high` `single-source-of-truth` — Range-overlap correction lives only in `check_match`; decision-tree codegen uses the raw overlap test and misroutes arms
- [ ] **F2** `high` `fragility` — LSP local rename replaces the whole `let` statement (and inserts at file offset 0 for parameters) — **partial: it now fails closed**
  - Stage 1 landed. Rename writes to the user's real files, so the first job was to stop the corruption, not to make it work. Reproduced **three** defects, not the two filed: (1) renaming a `let` local replaces the whole statement; (2) renaming a parameter inserts at file offset 0 and never touches the declaration, leaving a file that starts `renamedmodule Test`; (3) **renaming *from* a declaration site silently renames the enclosing function workspace-wide** — `hir_expr_at` only sees `Local` *use* sites, so a binding's own identifier falls through to `enclosing_decl_at`. That is the most natural rename gesture. A fourth was found by running: clicking a type reference in a body does the same, because `rename`'s `target_at` is the only one of its three copies lacking a `type_at_cursor` pre-check
  - Guard A: the edit span must literally spell the symbol's name. Chosen over the proposed `is_synthetic() || starts_with('$')` heuristic, which misses closure-destructure synthetics (`_cparam_N` gets a non-synthetic span and an ordinary name — both heuristics pass, the text does not match). Guard B: `enclosing_decl_at` results are kept only when the offset lands on the decl's own name span, copied to the two hand-rolled twins in `references.rs` and `document_highlight.rs`
  - 7 tests on what was **entirely untested surface**, each documenting that refusal is the *stage 1* target so a later reader flips the assertion rather than deleting it. Non-vacuity proved by stubbing each guard and watching the corrupting edit reappear
  - Stages 2-4 (`Local::name_span`, `AstParam::name_span`, then declaration-site resolution) are planned in [`docs/fragility/F2/decisions.md`](fragility/F2/decisions.md), along with a Stage-2 trap: `AstPat::Binding.span` covers `"var count"`, not `"count"` — the audit's "exact" example was a non-`mut` binding. `Local::name_span` also repairs go-to-definition, document-highlight, find-references, and the `let`→`var` quickfix, whose backward 20-char search for `"let"` can never find its own keyword and may rewrite a *previous* one
- [ ] **F9** `medium` `incremental-hazard` — `NominalCopySemantics`/`NominalStaticness` memos depend on a thread-local recursion stack that is not part of the cache key
- [ ] **F11** `medium` `single-source-of-truth` — The solver and the move checker ask `TypeParamCopyRequirement` with different `context`
- [ ] **F13** `medium` `single-source-of-truth` — A missing `;` after an expression statement in a function body is silently accepted
- [ ] **F14** `medium` `side-table` — Closure lowering's `SavedState` hand-mirrors `OssaBodyCtx`; three per-body fields are unsaved

## Query-framework and incremental hazards

- [ ] **F19** `medium` `incremental-hazard` — `TypedBody`'s hand-written `Hash` omits five output fields and hashes `errors` by length only
- [ ] **F20** `medium` `ordering-dependency` — The LSP despawns *before* `begin_revision()`, erasing the only invalidation despawn produces
- [ ] **F21** `medium` `incremental-hazard` — `World::snapshot` clones query memos but resets the accumulator store — and both docs say the opposite
- [ ] **F22** `medium` `global-state` — Push/pop guards are not unwind-safe, and two hosts catch panics and keep the thread
- [ ] **F23** `low` `incremental-hazard` — Accumulated values are never pruned by revision or despawn

## Remaining single-source-of-truth duplication

- [ ] **F28** `medium` `single-source-of-truth` — `lang.*` intrinsics are declared by a cross-product loop and lowered from a hand-written table; names have no lowering — **partial**
  - coverage test added — gap measured at **69 of 331** (all `cast_*`); names still unlowered
- [ ] **F29** `low` `single-source-of-truth` — `@builtin(.X)` arguments are never validated; `Builtin`'s four hand-maintained tables already have a hole — **partial**
  - the `DefaultArrayLiteralType` hole is closed + guard test; **argument validation (E479) still open**

## Verifier and self-check gaps

- [ ] **F35** `low` `side-table` — Borrow mutability lives only on the instruction, so threading a mut borrow through a block param downgrades it to shared
- [ ] **F36** `low` `side-table` — Mono's `WitnessCache` is built at collection cost, discarded with `let _ =`, and keyed by a lossy pair

## Editor and tooling

- [ ] **F42** `medium` `global-state` — `unsafe impl Sync for StdlibCache` is unsound — **blocked**
  - not fixable by a Mutex — rowan CST refcounts are shared across snapshots; needs a design decision
- [ ] **F43** `low` `single-source-of-truth` — Smaller tooling defects
  - [ ] F43a — `Compiler::build` is call-once-per-entity but nothing enforces it
  - [x] F43b — `PARAM_COUNTER` is a process-global counter whose doc claims it is reset — **fixed**. Both sentences of its doc comment were false: the names are `_param_N`, not `_0/_1`, and nothing ever reset it — `grep` found only the declaration and the `fetch_add`. Now a local threaded through `extract_params` (3 call sites, one per declaration, never re-entrant — default-value bodies take a different path). It matters because `_param_N` reaches user-visible E611/E613 text: in the LSP's long-lived `Compiler` the same unedited source yielded `_param_0`, then `_param_7`, then `_param_23` across rebuilds. Rule added to `kestrel-ast-builder/AGENTS.md`
  - [x] F43c — Diagnostic **message text** is built by iterating a `std::HashSet` — **fixed**. `initializer.rs`'s `all_fields` → `IndexSet`: measured 17 distinct field orderings in 25 runs before, 1 in 25 after. Declaration order, not alphabetical — `children_of_kind` already provides it free and it echoes what the user wrote. `duplicate_callable.rs`'s `seen` → `IndexMap`; that one is a **different defect than filed** — the message text was deterministic, the *emission order* was not (14 orderings in 20 runs). `extension_conflict.rs` is **refuted**: neither file of that name has the defect
  - [x] F43d — `module.witnesses` tail is appended in `HashMap` order — **fixed**. `IndexMap`, plus the worklist drain changed from `pop()` (a reverse iteration) to forward, so the tail follows `module.structs`. Currently unobservable — 12 MIR dumps at 3 stages were byte-identical — because every shim witness's `implementing_type` is a distinct nominal. That is one invariant deep: `select_most_specific`'s doc comment claimed "a deterministic candidate is chosen", which was false as written (it takes whichever tied candidate is first in `witnesses`). Corrected to say so and to name the dependency it cannot enforce itself
  - [ ] F43e — A type-blind copy of the irrefutability rule survives in analyze
  - [ ] F43f — Type-arg conformance failures deduped by a rendered display string
  - [ ] F43g — `ConformingProtocolInstantiations` dedup key embeds source spans
  - [ ] F43h — mir-lower accumulates E497/E503/ICE into a sentinel bucket

## Gap round (second pass)

- [ ] **G2** `medium` `single-source-of-truth` — The mono layout work-list is seeded only from body VALUE types — it never walks `Op1/Op2/Op3` type operands or struct fields — so a type reachable only that way gets no `MonoStruct` at all, `verify_mono`'s missing-layout guard is structurally unable to fire, and codegen silently answers size 8 / offset 0
- [ ] **G3** `medium` `fragility` — The thunk pass identifies the closure environment parameter by the magic names `"env"`/`"_env"`; a user function whose first parameter is named `env`, used as a function value, has that parameter replaced by the environment pointer
- [ ] **G4** `medium` `single-source-of-truth` — `--target` reaches only `@platform` filtering; layout and both codegen backends hardcode the host, so `kestrel build --target <other-os>` silently emits a host binary compiled against the other OS's stdlib
- [ ] **G5** `medium` `single-source-of-truth` — Two independent pointer widths and two independent size tables: MIR layout is hardcoded to 8 bytes while codegen derives width from the triple; the one path that honors `--target` also compiles with the native ISA
- [ ] **G6** `medium` `fragility` — `@platform` fails open on every argument it does not recognize, and no validator for the argument exists anywhere — an unparsed or unsupported `--target` triple disables platform filtering entirely
- [ ] **G7** `low` `global-state` — `KESTREL_COPYPROP_LIMIT` silently changes emitted code from inside a MIR pass, and the documented set of output-affecting environment variables does not include it
- [ ] **G14** `medium` `single-source-of-truth` — A where-clause param the substitution can't map is a PERMIT in the solver's evaluator and a REJECT in the analyzer's, so every associated-type-subject clause on a protocol extension (`extend Iterator where Item: Equatable`) is unentailable
- [ ] **G15** `low` `fragility` — `constraint_entailed_by`'s "param-declared bounds" tier queries `WhereClausesOf` on the TypeParameter entity, which never carries a where clause — the whole branch is unreachable
- [ ] **G19** `high` `fragility` — **NEW (2026-08-20).** Nothing in CI builds a *debug* compiler and compiles Kestrel with it, so `debug_assert!`s in the pipeline are never exercised against the corpus
  - `ci.yml` runs `cargo build --workspace` (debug) but never invokes the resulting `kestrel` on any `.ks`; the same job excludes `kestrel-test-suite`, and its own comment already concedes "some cases hit debug-only rowan asserts that don't fire in release". `bootstrap` and `triage` both build `--release`. So the release compiler is exercised ~3800 ways and the debug compiler zero ways
  - This is the **third** recorded debug-only compiler panic, and the **second** where the assert encoded a false invariant. Same shape as F8's "the suite never runs LLVM"
  - Rule worth adopting, recorded from the G3 follow-up: don't add `#[cfg(debug_assertions)]` invariants to the pipeline — either the property is worth checking in release or it isn't. Promoting an assert to unconditional makes the full-suite run real evidence that it holds, which a `debug_assert` never is
- [ ] **G20** `medium` `fragility` — **NEW (2026-08-20, found while fixing G1).** `verify_ossa`'s four `addr_*` checks are inert for **every initializer body in the language**
  - `AddrKind::SubField` state is created only by `InstKind::Uninit`, and an init's `self` is a `@mut_borrow` **parameter**, so it never gets a `state.addrs` entry and all four checks silently no-op. That includes `addr_store_init` ("store_init on field {..} but field already init"), which would have caught G1's own reassignment shape
  - Seeding it would activate `addr_require_init` across all 253 init-bearing files at once. Run detection-only first to size the impact
- [ ] **G21** `medium` `fragility` — **NEW (2026-08-20, found while fixing F40).** Aggregate-by-value across `@extern(.C)` is broken in **both** backends, independently
  - A plain 2-field `FFISafe` struct passed to C returns garbage under each, with *different* garbage (`875371297` cranelift vs `1428004377` llvm, expected `34`); scalars are fine. `abi.rs::build_extern_signature` passes every `Aggregate` as a bare `ptr_ty`, which is neither AAPCS64 nor SysV
  - Distinct from F40 and not fixed by it. Note `enum`s cannot conform to `FFISafe` (E422), so `IoError` can never cross `@extern` — F40's blast radius was contained by luck, not design
- [ ] **G16** `medium` `single-source-of-truth` — E101's condition-conformance test is a private `ConformingProtocols` lookup that only understands `ResolvedTy::Named`, so `if` on a `T: BooleanConditional` param or on `Self` is a false error — **blocked, do not fix in isolation**
  - Confirmed and **wider than filed**: `Param`, `SelfType`, `Opaque`, `AssocProjection` *and* `Ref` all get a false E101 (six reproduced shapes). `&Bool` in an `if` is a false E101 with no protocol involved — that one goes through `is_bool`, not `conforms_to_protocol`, so fixing only the filed predicate leaves it
  - **G18, its stated prerequisite, is now fixed** — `lower_if` calls `boolValue()`. The remaining blockers are the three below, all in type-infer
  - All three candidate fixes fail today: blanket-permitting abstract positions also destroys the one *true* positive (`func f[T](flag: T)` with no bound); `type_satisfies` can't be routed to because the private `resolved_ty_to_hir` bridge collapses every abstract variant to `HirTy::Infer`, which permits unconditionally; and `TypeResolver::conforms_to` — the semantically right predicate — takes `&TyKind`, whose `TyVar` is `pub(crate)` and whose `AssocProjection` variant cannot be constructed outside the crate at all. The right shape is a `ResolvedTy`-taking `conforms_to_resolved` entry point on the type-infer side. Sequence after G13. Full evidence: [`docs/fragility/G16/`](fragility/G16/)
- [ ] **G17** `medium` `single-source-of-truth` — `TypeResolver` re-derives where-clause bounds from raw `AstWhereClause` instead of `WhereClausesOf`, collapsing `T.Assoc: P` onto the bare associated type and resolving subjects in the ambient `body_owner` scope

---

## Fixed

Completed findings, moved here from their original sections. Grouped by the section they came from.

### Silent miscompilation and wrong behavior

- [x] **G1** `medium` `ordering-dependency` — Init drop-flag setup reads `needs_drop` at Stage::Raw, before `drop_fix` populates it — **fixed**
  - `lower_items`' doc comment claimed its two-pass split "ensures all TypeInfo (CopyBehavior, DropBehavior) is available when function bodies are lowered". True for `CopyBehavior`; **false for `DropBehavior`** — `fix_drop_behaviors` had exactly one call site, gated on `Stage::DropFix`, strictly after the read. That false comment is what made the ordering bug invisible to review, and correcting it is part of the fix
  - **Proven with `leaks`**: a `String` field assigned twice in an `init`, 200 iterations → **400 leaks / 265600 bytes**, now **0**. The `init?`-returns-null shape identically. MIR shows the second store was `store_init` where only `StoreAssign` gets the destroy-old expansion, and the failable init's failure block lacked the guarded-destroy diamond entirely
  - Fixed by hoisting `fix_drop_behaviors` between the two passes — the single-source-of-truth answer, since grepping the whole crate finds exactly **one** consumer of `needs_drop`. The existing later call is **kept**: body lowering synthesizes closure-environment structs mid-pass that the hoisted call cannot see. Both sites now carry a comment saying they are not duplicates. Idempotence is load-bearing and was verified by reading — the pass is monotone and additive, and its outer loop already runs to a no-change fixed point by design
  - **The audit's suggested `copy_behavior != Bitwise` stopgap was rejected after being compiled**: a `deinit` does not affect copy semantics, so a default-`Copyable` struct droppable only through a field is `Bitwise` and leaks under it too. That counterexample is now a test
  - The 8 existing fixtures all used `not Copyable` + `deinit`, satisfying both disjuncts — the suite was structurally blind. 6 new fixtures, each verified against a pre-fix compiler built in an isolated worktree
  - Filed not fixed: `verify_ossa`'s four `addr_*` checks are **inert for every initializer body in the language** — `AddrKind::SubField` state is created only by `InstKind::Uninit`, and an init's `self` is a `@mut_borrow` parameter. That includes the `addr_store_init` rule which would have caught G1's own shape
- [x] **F4** `medium` `fragility` — Escaping-closure box `init` is picked by arity alone; `RcBox` already has two 1-parameter inits — **fixed** (severity understated: it is a latent silent SIGSEGV, see Corrections)
  - The predicate is now the protocol requirement's *shape*, not its arity: `params.len() == 1 && params[0].label.is_none() && params[0].is_consuming` — exactly `SharedBox.init(consuming value: Target)`. Arity alone could not tell `RcBox`'s `public init(consuming value: T)` from its `private init(inner: Pointer[RcBoxStorage[T]])`, so the right answer came from declaration order in `rcbox.ks` and nothing else
  - **`find_box_member` no longer takes the first of N matches, or returns `None` on zero.** It collects every match and hard-`panic!("ICE: …")`s unless there is exactly one. The distinction that makes this safe: the legitimate "no box → stack environment" fallback is the `ResolveBuiltin` `?` *earlier* in `resolve_box_common` (real for `// stdlib: false`); by the time `find_box_member` runs the box type is already proven resolved, so 0 or 2+ matches is always a broken stdlib. Applied to all four call sites — `sharedMutRef`/`takeValue`/`destroy` had the same first-match-wins hole
  - Witness-based lookup (`find_protocol_witness_init`, as used for literal inits) was investigated and rejected: `UniqueBox` declares no protocol at all, so the shared helper would need two strategies for one call. Revisit if a `UniqueBox` protocol is ever added — [`docs/fragility/F4/decisions.md`](fragility/F4/decisions.md)
  - Proved end-to-end by building two compilers and swapping the two inits in `rcbox.ks`: old → exit 139 (SIGSEGV), fixed → exit 0. 3 unit tests pin order-independence and both panic paths without touching the stdlib; `escaping_primitive_only_capture.ks` covers the bare-primitive capture shape the directory lacked
  - Found in passing, recorded not fixed: `conformance_completeness.rs::signatures_match` compares arity and labels but **not** `is_consuming`, so a `SharedBox` impl omitting `consuming` passes E454/E458 and then hits the zero-match ICE. Loud, not silent — but the real fix is in the analyzer
- [x] **F8** `medium` `single-source-of-truth` — The LLVM backend never received the Bool-discriminant width fix that landed in cranelift (ef3fb801) — **fixed**
  - `discriminant_width()` returns a real width only for a `MonoEnum` and defaults to `I32` for everything else. `Bool` is a struct newtype over `lang.i1`, so LLVM emitted `load i32` off an `alloca [1 x i8], align 1` — three bytes of over-read plus an alignment lie — read adjacent stack garbage and always took the default arm. Now loads at the scalar's own width and truncates/z-extends to the tag width, as cranelift has since `ef3fb801`
  - One deliberate difference from cranelift: the arm is guarded on `scalar_ty.is_int()`. Cranelift's `ir::Type` makes `Ptr` just `I64`; inkwell would panic in `.into_int_value()` on a pointer or float scalar, so those keep the prior path rather than aborting
  - Verified by running: `match b { true => 1, _ => 2 }` gave 1 on cranelift and 2 on LLVM; both give 1 now, and the IR reads `load i8 … zext i8 %disc to i32`
  - **Why it survived 2.5 months**: `bool_match_with_wildcard_default.ks` is the regression test written *for this exact bug*, and it had no `// backends:` header — so it only ever ran cranelift. Now pinned `// backends: cranelift,llvm`. The deeper gap is unfixed and worth naming: `.github/workflows/ci.yml` excludes `kestrel-test-suite` entirely, so **nothing runs the `.ks` suite under LLVM on any PR**. A shared implementation is not feasible (incompatible IR builder types); the two arms carry reciprocal "twin — keep in sync" comments instead
- [x] **F3** `high` `single-source-of-truth` — "Is this a stored instance field?" is re-derived in ~15 places with three different predicates — **fixed**
  - `FieldClass` component set once by the field builder; storage is stored, not derived from absent markers. Six sites migrated. All four failures (wrong-slot write, E500 FP, E449 FP, OSSA ICE) verified fixed by running. Design + decisions: [`docs/design/f3-stored-field-consolidation.md`](design/f3-stored-field-consolidation.md)
  - Fail-loud backstops landed too: struct construction binds labeled args by **name** (unknown label ICEs, never falls back to position), and the OSSA verifier requires every `InstKind::Struct` to supply each `FieldIdx` exactly once. 4 unit tests; neither fires anywhere in the suite, so they only catch new drift
  - Invariant recorded in `lib/kestrel-ast-builder/AGENTS.md`
- [x] **F5** `medium` `fragility` — `break`/`continue` validation leaks across the closure boundary; MIR then silently no-ops the `break` — **fixed**
  - `loop_labels` save/restore in `lower_closure` + MIR backstop diagnostic
- [x] **F6** `medium` `single-source-of-truth` — `Copyable`/`Cloneable` lang protocols are identified by `name.ends_with(...)` in five MIR sites — **fixed**
  - lang items on MirModule; all 5 `ends_with` sites gone
- [x] **F7** `low` `single-source-of-truth` — Four predicates answered "which function is T's clone" — **fixed** (severity was lowered and the failure mode corrected first; see Corrections)
  - `TypeInfo::clone_impl` / `drop_impl`, written once by the shim passes that decide them; all four re-derivations deleted, `clone_method_self_nominal` gone
  - Root cause of the name-based predicate found by instrumenting it: `std.result.Optional.clone()` lived in a bare `extend Optional[T] {}` with **no conformance clause** — the "extend doesn't surface a witness" comment was a misdiagnosis. Declared `extend Optional[T]: Cloneable where T: Copyable` (`T: Cloneable` would be stricter than the body needs and would drop `Optional[Int64].clone()`), and the `.ends_with(".clone")` scan is gone. **A method merely named `clone()` no longer makes a type Cloneable.**
  - Witness-method key now sourced from the `Cloneable` declaration and compared as a full `WitnessMethodKey`, not by `.name` (that was a partial-key match)
  - Backstop: `verify_mono` asserts `copy == Clone(_)` ⇒ `clone_impl.is_some()` — one-directional; the converse is false for conditional containers and primitive-only structs
  - NOT fixed here: `build_clone_lookup`'s key still drops `parent_self` (97 entries / 78 keys — 19 benign overwrites per build, all same-`source`). That is shim over-instantiation, tracked separately below.
- [x] **F10** `medium` `single-source-of-truth` — Static-member lookup truncates to the first `extend` block — **fixed**
  - `resolve_extension_static_method` now sources candidates from `TypeMembersByName` — the query that already declares itself the single source of truth for "what members does this type have?" — instead of its own staged `ExtensionsFor` + `ConformingProtocols` walk. That deletes the truncation at both levels (first matching extension, and first conforming protocol) in one move
  - Precedence follows the instance-member rule already shipped in `kestrel_type_infer::resolve_member`: `Direct`/`Extension` candidates compete equally, and a `ProtocolExtension` default joins the set only if its **label signature** is not already taken. A type's own `tag()` beats `extend SomeProtocol { static func tag() }` without the pair going ambiguous, and a protocol default with different labels stays reachable. Suppressing protocol-extension candidates outright would have been F10's mistake in the other direction
  - Rule recorded in `lib/kestrel-name-res/AGENTS.md` ("Member lookup goes through `TypeMembers`"), along with the inventory of remaining open-coded walks
  - `find_in_extensions` also merges across extensions now (its one remaining caller, `resolve_assoc_type_static_member`, only uses the result as an existence check — hir-lower discards the entity and re-resolves through `Field { base: Def(assoc_type) }`)
  - Verified by running: with the old first-extension-wins body restored, `Foo.make(b:)` split into a second `extend` block fails with "no matching overload"; with the fix it compiles and runs. 2 execution tests under `declarations/extensions/`; full suite green
- [x] **F12** `medium` `fragility` — Associated types on type params are matched by **name string** against ancestor where-clauses, and E439 can't see extension-target params — **fixed**
  - where-clause subjects now resolve through `ResolveName` in the *bearing entity's* scope and are
    compared by `Entity` (`SubjectParam` / `subject_denotes`); name is only a prefilter
  - `ExtensionLhsParams` query = THE answer to "which target params does the LHS bind", backed by an
    `ExtensionLhsParamNames` component written once at build time. Consumed by
    `check_type_param_shadowing` (E439 now fires for generic extensions) and
    `resolve_extension_type_param`; the RHS free-param scan shares the same list
  - closed a latent leak found while fixing it: `T` used to resolve inside `extend Box[Concrete]`
  - 4 tests under `types/generics/`, incl. a negative guard against false E439

### Diagnostics infrastructure

- [x] **F15** `medium` `single-source-of-truth` — Every inference error is rendered twice by two divergent tables; the dedup rule lives in consumers and the LSP has none — **fixed**
  - `InferError::render(detail) -> RenderedInferError { code, message, label, notes }` in `kestrel-type-infer/src/error.rs` is now the ONE description of an inference error. `ResolvedInferError::to_diagnostic` is a thin wrapper; `kestrel-analyze/src/body/type_check.rs` (the second table, ~200 lines) and `AnalyzerId::TypeCheck` are **deleted**, along with the duplicated `vis_label` and the E624 `kind_mismatch_note`
  - Both open-coded `E100` filters are gone with the duplicate they guarded — CLI (`src/main.rs`) and test harness (`kestrel-test-suite/src/compiler.rs`). The LSP never had one; the double squiggle is now impossible by construction, not by filtering
  - Inference errors carry `E100`, the umbrella `docs/error-codes.md` always documented. Previously the codespan copy was **uncoded** and only the analyzer copy carried E100, so the two renderings of one mistake could disagree on code as well as wording (`E624` vs `E100` for a closure-kind mismatch)
  - Verified by running: `let x: Int64 = "s";` produced two error blocks at the same span before, one now. The five-file "adding an `InferError` variant" checklist is a three-file checklist; `AGENTS.md`, `docs/contributing/workflows.md` and `type-inference.md` updated
- [x] **F16** `low` `fragility` — Diagnostic-code registry has no ownership or reachability enforcement — **fixed**
  - **Ownership is enforced at emit time.** `kestrel-analyze/src/lib.rs::assert_owned` runs on every analyzer result: a diagnostic's code must be in that analyzer's `descriptors()`, or in a new `Describe::borrowed_descriptors()` for the legitimate case of two analyzers reporting one fact from different positions. The suite's 3700 files are the corpus
  - **The E436 collision is fixed.** `decl/generics.rs`'s `TypeResolution::NotFound` arm emitted `E436` (`non_protocol_bound`) carrying E476's message — a leftover from the 2026-07 renumbering that moved that meaning *off* E436. `struct Set[T] where T: NonExistent {}` now reports `error[E476]: cannot find type 'NonExistent' in this scope` (verified by running); `GenericsAnalyzer` declares the borrow
  - **Positional aliases removed.** `const E210: usize = 5; const E499: usize = 6;` indexing a positional DESCRIPTORS array is gone from `body/access_mode.rs`, and the same pattern from `decl/type_alias_validation.rs`. Both select by code through a `descriptor(id)` helper, so inserting a descriptor mid-array is a no-op instead of silently reassigning every later code, severity and docs link
  - **Silent reservations are gone.** `registry.rs::RESERVED_UNEMITTED` lists the seven registered-but-unemitted codes with a reason each (E206, E303, E446, E448, E502, E600, E602), checked **both ways** — a reservation naming a nonexistent descriptor fails, and emitting a reserved code fails
  - **New tests** in `registry.rs`: descriptor `name` uniqueness, `AnalyzerId` uniqueness within each list (a duplicate makes the second analyzer *never run*, since `find_*` is linear first-match), reservations name real descriptors, and every registered code is documented
  - **Docs made honest.** E480–E487 / E489 (the `RefPosition` family from hir-lower) are documented; E600/E602 are marked reserved instead of shown with worked transcripts they cannot produce; the ~37 fabricated `E05xx`/`E06xx` codes in `docs/language/pattern-matching.md` and `modules.md` are replaced — pattern-matching points at the real E300–E316 table, and modules.md now says plainly that **no diagnostic is emitted at all** for `import Some.Missing.Module` (confirmed by running)
- [x] **F17** `low` `side-table` — Analyzer diagnostics bypass the world accumulator; `kestrel dump` drops them and exits 0 — **fixed**
  - `CompilerDriver` now owns both halves: `analyze_all` records its summary, and `emit_diagnostics()` / a new `has_errors()` read the accumulator **and** the analyzer diagnostics. No consumer can see only one half, which is what `kestrel dump` was doing (`driver.analyze_all(false);` with the result unbound, then gating the exit code on the accumulator alone)
  - Emission is idempotent — the accumulator is append-only within a revision, so `emit_diagnostics` prints only what is new. That also fixes a latent double-print in `build`, which flushes once before codegen and again after and used to repeat every warning
  - `src/main.rs` loses its own `has_errors`, `cli_emittable_analyze_errors` and `emit_analyze_errors`; analyzer diagnostics now render with `.with_code()` like every other diagnostic (`error[E304]: …`) instead of a hand-appended `" [E304]"` suffix
  - Verified by running: a file whose only error is an empty `match` printed nothing and exited 0 from `kestrel dump diagnostics`; it now prints `error[E304]` with the span and exits 1
- [x] **F18** `low` `fragility` — `kestrel dump mir` swallows the MIR-stage diagnostics it just accumulated — **fixed**
  - `dump`'s arms return `Result<(), String>` instead of returning early, so the accumulator is flushed **before** the summary line in every branch. `dump_mir` and a new `dump_cranelift` no longer print-and-abort
  - Verified by running: `return &local;` printed `error: compilation failed with 1 error(s)` from `dump mir` and now prints the full `error[E494]` with its span, labels and notes
  - Two neighbours fixed with it: the pre-mono stage arm discarded its verify errors (`let (mir, _errors) = …`) where the post-mono arm surfaces them as warnings; and `Compiler::lower_to_mir_stage`'s doc comment claimed it "never accumulates diagnostics", which is false — `lower_module` deposits E497/E503 straight into the context

### Parser and CST integrity

- [x] **F24** `medium` `fragility` — `add_token_or_missing` widens a diagnostic span by one raw **byte**, producing non-UTF-8-boundary offsets that make the LSP drop every diagnostic — **fixed**
  - The diagnostic is emitted over the **whole last real token** (`last_real_token_span`), not `anchor_end - 1 .. anchor_end`. A token span is a char boundary by construction; the sink holds no source text, so it cannot walk back one character even if a narrower underline were wanted
  - Backstop landed too: `LineIndex::offset_to_position` clamps a non-boundary offset down to the character start instead of panicking on `&self.text[line_start..offset]`. It runs in `refresh`, *outside* the worker's `catch_unwind`, so one bad span took the whole `publish_diagnostics` call with it and nothing was logged
  - Proven non-inert: reverting the clamp makes `offset_inside_a_multibyte_char_clamps_instead_of_panicking` panic at the exact slice the audit cited
- [x] **F25** `medium` `fragility` — Module/import emitters re-derive `.` `(` `)` `,` `as` spans by byte arithmetic — and it fires on shipping stdlib source — **fixed**
  - The parsers **carry** the separator spans instead of the emitters inventing them: `module_path_parser_internal` returns `ModulePathSpans { segments, dots }`, and imports return `ImportSpans` / `ImportItemsSpans` with the real `.` `(` `,` `as` `)`. `AttributeArgValue::Path` had the same defect and now shares `ModulePathSpans`
  - Grammar is unchanged — only which spans get recorded. (An intermediate version accidentally began accepting a trailing comma in an import list; reverted.)
  - Fail-loud guard: `TreeBuilder` `debug_assert`s that a fixed-lexeme kind is emitted over text that actually spells it. Restoring the old arithmetic makes it fire with the audit's exact evidence — `token RParen was emitted over the text "\n"`. Zero-width spans are exempt: a recovery branch may legitimately synthesize an absent token, and running the guard over the whole stdlib surfaced exactly one — the missing `;` of an expression statement, which is **independent confirmation of F13** (still open)
  - 2 regression tests over the shapes that were broken (space around `.` and `as`, the multi-line stdlib form): well-formed imports must produce **no `SyntaxKind::Error` tokens**, and the CST must still round-trip. The round trip alone is what kept this invisible — it passed while the token *kinds* were wrong — so both are asserted
- [x] **F26** `medium` `single-source-of-truth` — Escape-sequence decoding is implemented three times, and one copy silently drops `\u` entirely — **fixed**
  - One table: `kestrel_ast::escape::decode_escape`, in the deepest crate both `kestrel-ast-builder` and `kestrel-hir` can reach. `EscapeErrorKind` / `UnicodeEscapeErrorReason` moved there too; `kestrel_hir::body` re-exports them so every existing path still resolves. All three callers keep only what differs — span arithmetic and error recovery. 10 unit tests on the kernel
  - **Two miscompiles verified fixed by compiling and running.** `"\u{41} \(x)"` printed `u{41} 1` (the interpolation path's private decoder had no `\x` arm, no `\u` arm and no error path) and now prints `A 1`. `'\u{00000041}'` compiled to `'A'` while the string form was rejected — the char decoder read hex with an unbounded loop, no close-brace requirement, no digit limit — and is now `error[E702] … at most 6 hex digits`, spanned on the escape
  - **Char literals now report E700–E703, in both positions.** Errors are data on `HirLiteral::Char { value, escape_errors }`, exactly as `String` already carried them, and `StringEscapeAnalyzer` reads both through one accessor. They used to be `ctx.accumulate`d ad hoc with no E-code — and skipped entirely in pattern position, where the decoder ran with `ctx: None`, so `'\u{D800}'` in a `match` arm silently became NUL. Confirmed by running: it now reports E702 from a pattern
- [x] **F27** `low` `single-source-of-truth` — `SyntaxKind`'s 258 variants are restated twice in the same file; a miss reads back as `Error` — **fixed**
  - The 258 `const NAME: u16` declarations and 258 `match` arms over `raw.0` are deleted (~520 lines). `kind_from_raw` indexes `SyntaxKind::ALL`, a single declaration-order table, because `kind_to_raw` is `kind as u16`
  - `ALL` is hand-written but **proved**: `syntax_kind_table_round_trips` asserts it is ordered (`ALL[n] as u16 == n`), complete, and that out-of-range still reads `Error`. Completeness needs the variant count, which comes from a `#[doc(hidden)] __NotAKind` end-marker — without it a *truncated* table round-trips happily, since every entry it holds is correct and the missing kinds are simply never tested
  - Verified non-inert: deleting the last entry fails with "SyntaxKind::ALL is missing 1 kind(s)"

### Remaining single-source-of-truth duplication

- [x] **F30** `medium` `single-source-of-truth` — `desugar_logical_and` re-encodes the `SHORT_CIRCUIT_OP_PROTOCOLS` row and silently drops the RHS — **fixed**
  - `desugar_logical_and` is **deleted**. `lower_if_conditions` joins comma-separated conditions with `desugar_binary_hir(BinaryOp::And, ..)` — the same table-driven path a written `and` takes — so `lookup_short_circuit_op` is now the one place that knows which protocol/method a short-circuit operator maps to
  - The old fallback (`protocol not found` → return `lhs`) **silently discarded the RHS**: `guard a, b` compiled to `guard a`. It now reports `unsupported binary operator 'and'` and yields `HirExpr::Error`. Regression test `expressions/short_circuit/comma_conditions_report_missing_and_operator.ks`
  - Found one testdata file living off the bug: `patterns/guard_let/chains/guard_multiple_bool.ks` was `// stdlib: false` with `lang.i1` operands, which can never resolve `logicalAnd` (`Bool` is the only conformer). Given the stdlib and `Bool` params, so it tests what it always intended
- [x] **F31** `medium` `fragility` — `lower_condition_chain` re-invokes `on_fail` per condition, duplicating diagnostics and lowering `else if` chains exponentially — **fixed**
  - The fail continuation is lowered **once**, in `lower_condition_chain`, and the recursion moved to `lower_condition_chain_with_fail(.., fail: HirExprId)` which shares that one id across every level. Duplicating was only ever *semantically* safe (guard/while fail arms diverge, if-let's else runs on one path); it was never cheap
  - The compounding case is the real one: an `else if` is itself lowered through this function, so a per-level `on_fail` lowered the tail of a chain 2^depth times, multiplying the findings of every HIR-walking analyzer with it
  - 2 tests count literal occurrences in the HIR arena (`a_conditions_else_body_is_lowered_once_per_chain`, `an_else_if_let_chain_does_not_lower_its_tail_exponentially`)
- [x] **F32** `low` `single-source-of-truth` — `substitute_resolved_ty` is a second, non-exhaustive copy of the substitution kernel — **fixed**
  - The `_ => ty.clone()` arm is gone. `Ref` and `Opaque` both carry nested `ResolvedTy`s and were silently *not* recursed into, so a type param survived into MIR unsubstituted inside `&mutating T` and inside an opaque's bounds/`origin_args`. Leaf variants are now listed by name, so a new `ResolvedTy` variant is a compile error rather than a silent skip
  - Unit test `substitution_reaches_inside_refs_and_opaques`, which also pins that `mutating` and `not_copyable` survive the rebuild
- [x] **F33** `low` `single-source-of-truth` — Small duplicated predicates and magic strings — **fixed** (all 11)
  - Two sub-items turned out to be live user-visible bugs, not latent duplication; see **Corrections**
  - [x] F33a — Trivia-kind set — the set lives on `Token::is_trivia`; `SyntaxKind::is_trivia` is its image under `From<Token>`, and `trivia_agrees_with_the_lexer` pins both directions. 9 copies + 3 duplicate `skip_trivia` parsers deleted
  - [x] F33b — `is_type_node`/`is_type_kind` → one `SyntaxKind::is_type`. `every_ty_kind_is_a_type_node` *derives* the set from the enum: a `Ty*` variant must be a type node or be excused by name in `NON_TYPE_TY_KINDS` (only `TyList`). Re-proved by deleting `TyRef`/`TyMutRef` — reproduces the original divergence exactly
  - [x] F33c — `Name::ROOT` + `Name::is_root()`; all 5 production consumers and ~40 test literals swept, one spelling left in the tree. `the_root_entity_is_recognized_as_root` connects the producer (kestrel-compiler) to the fail-open consumer (name-res visibility), which live in different crates
  - [x] F33d — `NodeKind::is_type_scope()`, written as an **exhaustive match with no wildcard**, so the audit's scenario (adding `NodeKind::Class`) is now one compile error instead of 7 silent misses. Verified by adding the variant
  - [x] F33e — the two analyzer forks now delegate to `kestrel_name_res::helpers::member_lookup_name`. They keyed subscripts on `NodeKind::Subscript` while the real lookup keys the `Subscript` **marker**; `subscripts_carry_both_the_node_kind_and_the_marker` pins the invariant that made them accidentally agree
  - [x] F33f — the `unwrap_or(BinaryOp::Add)` / `unwrap_or(UnaryOp::Neg)` fallbacks are gone (unrecognized operator → `AstExpr::Error`, not a different program). Map agreement is covered by F33g's round-trip
  - [x] F33g — one `BinaryOp::symbol()` in `kestrel-ast`. `operator_spellings_round_trip_through_the_lexer` walks the proven-complete `SyntaxKind::ALL` through the parser's own maps and re-lexes each spelling — no hand-written list. Catches the historical `&&`/`||`/`...` rows directly
  - [x] F33h — one `kestrel_compiler::stdlib_path`. Was **5** copies, not 4 (`examples/build_test.rs` too); the CLI's was the only one with the `exists()` check, so the test suite could take a stale `KESTREL_STD` and load zero files. The `io/libc_shims.c` path is now written once. **The merge itself first shipped a regression — see Corrections**
  - [x] F33i — **live bug.** `[T]`/`T?`/`[K:V]` now resolve through the `@builtin(.XTypeOperator)` lang item in the *stdlib's* scope. See Corrections
  - [x] F33j — `requires_fields_conform` and `tuple_conformance_propagation` now have readers: `ProtocolFieldConformanceAnalyzer` iterates conforming protocols and reads the flags instead of hardcoding `Builtin::FFISafe`. Proved load-bearing by flipping the flag (17 passed → 16 passed, 1 failed)
  - [x] F33k — **live bug.** kestrel-doc's `format_type` fork deleted; `kestrel_ast::pretty::format_type` is now `pub`. See Corrections

### Verifier and self-check gaps

- [x] **F37** `medium` `fragility` — `find_inherited_assoc_type` recurses through protocol inheritance with no cycle guard its sibling has — **fixed**
  - The two functions were not siblings, they were **one walk duplicated**. `find_inherited_assoc_type` is deleted; `search_protocols_for_assoc` now calls `resolve_name::resolve_inherited_protocol_member` (widened to `pub(crate)`), seeding one `visited` set before its loop. Net −38 lines. Both already bottomed out in the same `find_assoc_type` leaf
  - **Real crash, not theoretical**: a qualified-path protocol cycle (`protocol A: Test.B` / `protocol B: Test.A`) plus an associated-type reference through it overflowed the stack and aborted — exit 134, 527 frames at the same call site. E459 does not protect this path; `ProtocolCycleAnalyzer` is a `CompilationCheck` that *consumes* name-res queries, so resolution runs underneath it. After the fix all four shapes terminate and report E459 plus a truthful "cannot find type"
  - **A second bug was hiding inside the first.** The deleted copy computed `parent_of(scope)` and passed the result down as the next level's `scope`, so the resolution anchor climbed one *extra* ancestor per recursion level. That is why the bare-name cycle never crashed — the drift walked out past the module root and `ResolveTypePath` stopped finding the protocol, so the recursion bottomed out **by accident**. Writing the cycle as `Test.B` keeps the target resolvable from any ancestor and removes the accidental brake. The shared function anchors on `parent_of(protocol)`, computed fresh per level, which is what a conformance path should resolve against — where it is *written*
  - 4 tests under `validation/cycles/`, including the one that matters most: a bare-name cycle whose assoc type genuinely exists must **still resolve**, since `visited` is now the only thing bounding the walk
  - Deliberately not touched: `resolve_type.rs:528`'s same-shaped `parent_of(scope)` anchor. It does not self-recurse so it cannot be this crash, and changing it moves scope resolution across 5 call sites — [`docs/fragility/F37/decisions.md`](fragility/F37/decisions.md)
- [x] **F34** `medium` `fragility` — The OSSA verifier's linear-ownership check is block-local, and never runs after mono at all — **fixed** (the audit's *prescribed* remedy was wrong; see Corrections)
  - The finding stood (the check *was* block-local and cross-block double-consumes were invisible). What was wrong is the proposed remedy: enforcing a "block-parameter live-in contract" would reject the stdlib. Measured 2026-08-11 — 121 of 354 `memory_model` tests fail, and the violations are in shipped stdlib bodies (e.g. `std.text.ClosedRange.readLines`). Lowering follows the ordinary SSA rule (use any *dominating* definition); the contract asserted at `verify.rs:6` was documentation of an invariant nobody maintained, and it is the reason the ownership walk was written block-local in the first place
  - Fixed along the corrected direction instead — the walk is now **whole-function and dominance-aware**: an RPO walk to fixpoint over a per-block `FlowState` joined along CFG edges (`fec27941`, `a46c077e`), with address aliasing so a take through a derived address is seen at the storage it came from, and a Cooper-Harvey-Kennedy dominator check replacing the rejected live-in contract. Enforced by default in every build as of `3fc35b88`; `KESTREL_VERIFY_FLOW=off` is the escape hatch. The post-mono half runs via `verify_ossa_mono` (`8390ea5c`), gated by `KESTREL_VERIFY_FLOW_MONO`
  - It found a real bug: closure-call arguments took a value out of its slot to hand over a borrow, which broke *every* debug build (hello world included) and silently miscompiled in release — 931 files / 4655 violations → 0 (`3662a94e`). A second, related lowering bug (a mono-dependent local taken on every read rather than only its last) was fixed in `5d334b70`
  - **Caveats, so this isn't read as more than it is.** The post-mono walk currently reports *nothing* — mono IR is well-formed OSSA and the instantiation-specific double-free class is semantic, not an ownership violation; it lands as infrastructure and should be deleted if it is still finding nothing in a few months. And the dominance check is likewise silent corpus-wide, proven non-inert only by unit tests. Suite: 3719 passed, 1 failed (`stdlib.os.os_fs_result`, failing identically in every historical run)

### Editor and tooling

- [x] **F39** `medium` `single-source-of-truth` — Completion open-codes member lookup instead of `TypeMembers`, missing every protocol-extension member — **fixed**
  - The hand-rolled `children_of` + `ExtensionsFor` walk is replaced by a dispatch on `NodeKind`: `ProtocolMembers` for a protocol-typed receiver, `TypeMembers` for everything else. They stay **separate on purpose** — `collect_members_transitive` passes `include_parent_direct_children: false` for `TypeMembers`, so unifying would drop an inherited protocol's own direct requirements
  - Impact was not marginal: every concrete `Comparable` conformer was missing `lessThan`/`greaterThan`/`isAtLeast`/… , and all ~16 `extend Iterator` blocks (`map`, `filter`, `zip`, `sum`, `enumerate`, …) were invisible on every concrete iterator type
  - **Visibility filtering had to be added, not inherited.** `TypeMembers` is deliberately unfiltered — the filter normally lives in `TypeMembersByName`. Completing across modules used to offer `private` and `fileprivate` members; now every push goes through one `IsVisibleFrom` choke point. Proven non-vacuous by stubbing the gate and watching the test fail
  - Nested types needed a separate `children_of` pass (neither query's member filter admits them), and direct children suffice — the grammar has no nested-type arm in an extension body
  - 5 new unit tests; none of the 10 existing ones changed. Open question recorded in [`docs/fragility/F39/decisions.md`](fragility/F39/decisions.md): `TypeMembers` returns constrained-conformance members unconditionally, so completion now *over*-offers where a where-clause is unsatisfied — a false positive replacing a false negative, accepted for an IDE
- [x] **F38** `medium` `side-table` — `disk_line_indices` is a second copy of file text that `didOpen`/`didChange`/`didClose` never update — **fixed**
  - **Deleted, not synchronized.** A hand-synced second copy of text that `sources` already owns is the bug class itself: its four writers were all path-derived while the three editor-driven handlers never touched it. Indices are now built on demand in `refresh` — live `docs` buffer first, else from `sources`
  - The audit's "7-ish read sites" did not hold up: there was exactly **one** genuine read, and its plumbing deep-cloned ≥1.4 MB of `String` on every debounced keystroke. Building on demand is therefore strictly *cheaper* than what it replaced — and only for files a diagnostic actually points at, verified by checking that every `FileMap::lookup` call site is label-derived. The 84-file stdlib is no longer indexed at all
  - Two reproduced failures, both closed: after `didClose` on an unsaved buffer, diagnostics re-anchored to the stale disk text (a 5-line slide, plus a range spanning 3 lines because F24's clamp turned the overrun into a plausible position rather than a panic — which is exactly why it was invisible); and a file opened but never visited by the workspace walk had its diagnostics **dropped entirely**. Both new integration tests were proven non-vacuous by restoring the old sources and watching them fail with the documented symptoms
  - `convert.rs` untouched — `FileMap` stays a borrow over an owning local. Unresolvable ids now produce a WARNING naming them instead of vanishing at a `?`
- [x] **F41** `low` `single-source-of-truth` — Atomic RMW width is hardcoded `I64` in cranelift but taken from the operand in LLVM — **fixed**
  - width from value operand; 2 execution tests on both backends

### Gap round (second pass)

- [x] **G12** `medium` `single-source-of-truth` — The Never-typed divergence rule is copy-pasted into five analyzers — **fixed**
  - Worse than filed: **three** different `Loop`-handling mechanisms across the five, and a sixth analyzer (`dead_code`) consulting no types at all. Two live bugs, both reproduced from the CLI
  - **A false E500.** `move_tracking` is the only analyzer whose flawed formula wasn't accidentally rescued by a trailing Never-check (it explicitly excluded `Loop`), so a breakless `loop { doWork(); }` was called non-diverging and a value consumed after it reported "use of moved value" — on a line `dead_code` simultaneously called unreachable
  - **Missing E002** after a `-> !` call at top level, inside a `while`, and after an all-arms-diverging `match`
  - **The carve-outs' stated justification was false.** Two comments claimed "every loop is typed Never", but `generate.rs:590-605` unifies a loop's `break_tv` with unit at every break that targets it. Both corrected
  - **Ordering is load-bearing and is now pinned**: structural-first, Never as the leaf fallback *only*. `guard.rs` checked Never first, which would let inference override the structural `Loop` verdict — exactly the hazard G8 removed. And the `Loop` rule is `!contains_break_for` **alone**; the `body_state.diverged &&` conjunct two analyzers carried is provably wrong for `loop { doWork(); }`, which is the false-E500 root cause
  - `control_flow.rs` gains a clearly-marked Tier 2 taking `&BodyContext` — one named exception to the pure-predicate contract, because the `Loop` case of "does this diverge" *is* `block_contains_break_for`, so a separate file would import Tier 1 for its only non-trivial branch. `AGENTS.md` §5 amended consistently with the G8/G9/G10 amendment
  - `dead_code`'s `in_loop` parameter drops out entirely (break/continue are unconditionally Never-typed), which fixes its labeled-break conservatism for free — the same three lines. `block_always_returns`/`expr_always_returns` were verified provably dead before deletion
  - Also fixed in passing: only one of the six copies handled `HirStmt::Let { value }`, so `let x = fatalError();` didn't make the rest of the block unreachable
  - There was **zero** coverage of `-> !` divergence for E002/E001/E004/E005/E500 — only two E003 files. 7 tests added; 9 of the 11 affected tests were proven to fail against the pre-fix analyzers, and the implementer flagged the other two as honestly vacuous rather than dressing them up. Full suite 3795 → 3802, zero failures
- [x] **G8 + G9 + G10** `medium` `single-source-of-truth` — one label model for loops, replacing four copies and six divergence answers — **fixed as one campaign**
  - Fixed together because they are one bug wearing three hats, and fixing G8 alone would have created a *fifth* copy of the duplicated predicate. The audit's counts were low: **four** `contains_break` triads (`dead_code`, `exhaustive_return`, `definite_assignment`, `move_tracking`), not three, and **six** independent "does this loop diverge?" answers, not three
  - **G9** — all four copies did `Break { .. } => true`, ignoring the label, and none recursed into a nested loop. Live consequences: a false E002 on the checked-in `break_from_nested_loop_3_levels.ks`, flagging code that demonstrably runs; and E001 *suppressed* one nesting level down — `func f() -> Int64 { outer: loop { loop { break outer; } } let z = 1; }` compiled clean and returned garbage from a function with no return on any path
  - **G8** — `guard.rs` had `Loop { .. } => true` with no break check at all, so **any** breakable loop satisfied the divergence gate, not just the desugared `while`. `guard x > 0 else { while true { break; } }` with `x == 0` fell straight through and returned 99
  - **G10** — a labeled break landed in the innermost frame, the outer frame popped empty, and the "all fields initialized" check was **skipped entirely**, not weakened. `S()` constructed with its field never stored
  - **Two tiers, deliberately not one.** The atomic fact — `kestrel_hir::label_selects_loop` — goes in the pure-data crate and is now called by *both* `mir-lower`'s `find_loop` and analyze, so the rule is shared by construction rather than by comment. The walk goes in a new `kestrel-analyze/src/body/control_flow.rs`; it needs `Sugar` handling and analyzer stop rules and does not belong in a crate with zero walking logic. G10 keeps its own stack — it is *reachability*-aware (it captures `InitState` at each break), strictly stronger than the syntactic walk — and shares only the predicate
  - The subtle part is a `crossed` flag that is **not** expressible via the target label alone: it separates "still directly inside this loop" (where an unlabeled break counts) from "inside a nested loop" (where only a matching labeled break still reaches out). Ten unit tests, including the shadow case where a nested loop reuses the target's exact label
  - **`AGENTS.md` §5 was the real blocker** — it sanctioned exactly this duplication ("control flow analysis lives as private functions in the analyzer file") while the same file's "One analyzer per fact" rule forbade it. Amended, so the next agent doesn't re-fork the helper by the book
  - The drift had already started: `dead_code.rs` alone had gained a `Sugar` arm from the G11 fix one cycle earlier. That also let this campaign close G11's deliberately-deferred `guard.rs` Sugar arm — safe only once the break check landed, and pinned by a test written *before* the arm so its correctness moved from accidental to principled
  - Full suite 3758 → 3768, zero new failures. Every new test proven durable by restoring the old analyzers and watching it fail. Left as a noted follow-up: `dead_code.rs`'s divergence arm is still `in_loop && label.is_none()`, so dead code *after* a labeled break is a conservative false negative
- [x] **G18** `high` `fragility` — **NEW (2026-08-20, found while diagnosing G16).** `BooleanConditional` is analysis-only: conditions branch on the value **raw**, never calling `boolValue()` — **fixed**
  - Reproduced with a conformer whose `boolValue()` inverts its payload: `v=200` took the TRUE branch while `boolValue()` was `false`, `v=0` took FALSE while it was `true`. MIR was `branch %v4` on an `@owned Test.Inverted` — a struct branched on as if it were an `i1`. The witness was always fine; an *explicit* `x.boolValue()` lowered to a real call. Only the implicit path skipped it, and the backends made it silent rather than crashing (`icmp_imm(NotEqual, cond, 0)`)
  - Invisible because the only shipping conformer is `Bool`, a single-field `lang.i1` wrapper whose raw layout *is* its `boolValue()`. All 13 `testdata/builtins/boolean_conditional/` files were `diagnostics`-kind — two of them the exact miscompiling shape, only type-checked. Both are now `execution`
  - Fixed in **mir-lower**, the only layer that can see the type: hir-lower has no type information (its `AGENTS.md` states it as a hard rule) and type-infer deliberately records no constraint on the condition tyvar. One helper, two call sites — `HirExpr::If` is the single shape `if`/`else if`/`while`/`guard else`/the non-binding `if let a, cond` link all lower into, and the match guard is the only other production point
  - **The performance gate is the whole design.** `std.core.Bool` is a nominal struct, *not* `MirTy::Bool`, so "skip when already `lang.i1`" does not cover it — and `Bool` is the condition type of nearly every `if`. With ~1835 branch sites after stdlib mono and no MIR inliner, a naive fix would regress Cranelift and every debug build. The gate is on the resolved `Builtin::Bool` **entity**, never structural: a "single-field struct wrapping `lang.i1`" test would silently swallow a user's own `struct MyFlag`. Pinned by a test with a user `struct Bool` *and* a structurally identical `MyFlag`, both dispatching correctly
  - `Builtin::Bool` resolved purely by name with no `@builtin` anchor and no `from_attribute_name` arm — strategy-1-only with no fallback, the recorded inert-lang-item shape. Added both; proven live by deleting the arm and watching `every_stdlib_builtin_annotation_is_recognized` fail
  - Measured, not assumed: `Branch` counts identical before/after (1834→1834, 1835→1835), and **zero** `boolValue` calls emitted on a program that only branches on `Bool`. Full suite 3750 → 3758, delta exactly the 8 new tests, zero regressions
  - Found while implementing: the `consume`-the-scalar approach in the design fails OSSA verify; leaving it scope-tracked lets `lower_if`'s existing liveness threading handle it, which is why `lower_if` never needed an `extra_vals` mechanism. A sixth surface position (the non-binding link of `while let p = e, cond`) and two further `emit_branch` sites were found and documented; both already branch on `MirTy::Bool`
- [x] **G13** `medium` `single-source-of-truth` — The extension-bound evaluator SKIPS `Copyable`/`Cloneable` clauses because `type_satisfies` cannot answer them; conformance-completeness calls `type_satisfies` on exactly those clauses anyway and gets a hard `false` — **fixed**. **Not latent — two live bugs pointing opposite ways**
  - **False reject**: a protocol extension carrying a `Copyable` where-clause whose member witnesses a requirement gave `error[E454]: type 'BoxC' does not implement method 'dup'`. The same program with `where T: Equatable` compiled clean, and calling `.dup()` directly compiled clean — the solver routed it through the extension and only the analyzer disagreed
  - **Unsound accept, live in the shipped stdlib**: because the solver's evaluator *skipped* copy bounds, a `Copyable` where-clause did not gate member selection at all. `RcBox[NC].getValue()` on a `not Copyable` payload compiled clean and **exited 132 (SIGILL)** — against `rcbox.ks`'s own comment that "only a box over a non-Copyable payload loses these two methods". `Pointer[NC].pointee` the same
  - Fixed by teaching `type_satisfies` to answer, not by duplicating the skip. Option (a) would have killed the E454 while cementing the SIGILL as a permanently unenforced language rule. The arm sits beside the existing `Builtin::Static` arm and delegates to `hir_type_copy_semantics` — the same `instance_semantics` kernel `TypeResolver::conforms_to` and the solver already bottom out in, so this takes the count of independent answers to "is this Copyable" from eight down to seven rather than adding a ninth
  - Abstract positions permit **before** delegating. `HirCopyLayer` is not conservative — it can return a definite `NotCopyable` for a `not Copyable`-bounded `Param` — which would break the module contract. `SelfType` needs the permit for a different reason: at a protocol-extension body it resolves to the *protocol*, not the eventual conformer
  - **User-facing behavior change**: a runtime SIGILL is now a compile-time diagnostic on public stdlib surface. Full suite 3745 → 3750, delta exactly the 5 new tests, zero new failures
  - Found not fixed: reaching a `where T: Copyable`-gated member through a *generic protocol bound* fails at mono — `type_conforms_at_mono` evaluates it by searching the witness table for a `Copyable` witness, which structurally never exists. Mono's own independent answer, on a path this fix doesn't touch
- [x] **G3** `medium` `fragility` — The thunk pass identifies the closure environment parameter by the magic names `"env"`/`"_env"` — **fixed**
  - `FunctionKind::takes_env_param()` replaces both name sniffs; forwarding is now a structural `.skip(1)`, so **no name-based filtering survives in the pass**. The discriminator already existed — `mono/collect.rs:679` used exactly this `matches!` for the same question — and the env `ParamDef` is pushed unconditionally for both closure kinds, so kind ⟺ leading-env-param with no gap. No new `ParamDef` field
  - Four silently miscompiled shapes, all legal code: `func combine(env: Int64, x: Int64)` used as a function value returned **3 instead of 307** (the value lands in the wrong slot, the last argument is dropped, and `env` receives the environment pointer). `_env` the same; `env` at position 2 and `self` as a parameter name were hard backend-verifier failures. All six repros now give 307
  - The audit addendum's claim that the `self` twin "is safe only because `self` is reserved" is **wrong** — `self` is not a keyword and `func combine(self: Int64, ..)` is legal. Corrected in place
  - **The design's reachability claim was false, and following it would have shipped a fifth miscompile.** `Type.instanceMethod` *can* reach `ApplyPartial`: the old `self` filter was accidentally turning it into a hard error, so removing the filter made `apply(Box.doubled, 7)` compile and print garbage. Root cause is in name-res — `walk_path_from`'s direct-children branch applies no member-kind filter, and `is_static_method` guards only the *extension* fallback, so an instance method declared in the type's own body sails through. Closed in hir-lower by sharing the predicate and emitter that already rejected the *call* form `Box.doubled(b, 7)`; the value form is the same rule with the parens removed. That also gave the call form the `E100` code it always should have carried, and turned `P.hop`-as-a-value from an ICE into a diagnostic
  - Backstops: a per-position **type** check on the thunk's forwarded arguments (a pure arity check would NOT have caught the repro — the count coincidentally matched), plus a general `InstKind::Call` arity check in `verify_ossa`. That one fired immediately on two `drop_shim` test fixtures which stubbed a zero-parameter deinit while a real deinit declares `mutating self` — the fixtures asserted an inconsistent module verified, and were corrected
- [x] **G11** `low` `fragility` — `dead_code.rs` never handles `HirExpr::Sugar`, so E002 is structurally blind inside every `for`-loop body in the language — **fixed**
  - Four transparent `Sugar { inner, .. }` arms. Only one changes behavior — `check_expr_inner`'s. `for` lowers to `Sugar{ForLoop} → Block → Loop → Match → user body`, so a single arm restores the whole subtree; `while` lowers to a bare `Loop`, which is why `while` always worked
  - E002 had **zero testdata coverage** — that is how it stayed invisible. 6 files added, including two negatives, one of which pins `expr_diverges(Sugar{ForLoop}) == false` so the fix can't start calling code *after* a `for` unreachable. Two-way matching proven non-vacuous by perturbation, not assumed. The `for` tests need `// stdlib: true`, unlike their siblings: without `Builtin::IterableProtocol` the desugar short-circuits to a `Sugar`-wrapped `Error` and would prove nothing
  - **The addendum's advice to add the same arm to `guard.rs` was rejected, and following it would have been a regression.** `guard.rs:167` has `HirExpr::Loop { .. } => true` with no break check (that is G8). Today `guard flag else { for … {} }` is correctly rejected *only* because `Sugar` falls into `_ => false`; making it transparent before G8 lands turns a correct rejection into a false acceptance. G8 is a prerequisite, not an unrelated neighbour
  - Converting the `_` catch-alls to exhaustive matches was also rejected: `HirExpr` has 25 variants, it would take ~76 variant mentions across 4 sites, and five sibling analyzers use the same `_` idiom — hardening one of six identical sites buys false confidence, not safety
  - Adjacent, filed not fixed: `exhaustive_return.rs` has the same blindness (masked by an inference-error bail), and `check_stmt_inner` never walks `HirStmt::Let { value }` or call arguments, so dead code in a `let`-bound closure is still missed

---

## Ruled out (do not re-file)

Refuted during verification. Re-raise only with new evidence.

- `kestrel-copy-fold` is genuinely one decision tree; the apparent `FnKind::Mutating` divergence is documented, self-consistent with `needs_drop`, and pinned by a shipped test.
- hECS `QueryKey` is not a second identity decision — derived from the memo key by one function, and all 59 query key types derive `Hash` structurally.

## Corrections to earlier severity claims

Established by running the code, not reading it:

- **F40 was under-rated `medium` and is raised to `high`.** The audit filed it as two backends disagreeing — a consistency smell. It is a **live silent data-corruption miscompile on the default backend, in shipped stdlib code**: `IoError` read `errno = 1 1` where the answer is `2 77`, and a newtype over `Optional[Int32]` in an `Array` produced `999 999` and SIGBUS. The suite's one long-standing failure, `stdlib.os.os_fs_result` — failing on every run since 2026-07-23 — was a casualty and now passes.

- **F4 is a latent silent SIGSEGV, not an overload-selection nit.** The audit filed it as "picked by arity alone". Reproduced 2026-08-20 by building two compilers and swapping the order of `RcBox`'s two 1-parameter inits in `rcbox.ks` — nothing else: the old predicate then selects `private init(inner: Pointer[RcBoxStorage[T]])`, hands it the raw environment struct, and **every escaping closure in the program** stores its captured value straight into `RcBox.ptr` as a forged handle. Exit 139, with no diagnostic at any stage. The pre-mono MIR dumps of the two builds are byte-identical (220595 lines, empty `diff`) — both print `call std.memory.RcBox.init[E](...)` — so the wrong pick is invisible until the mangled symbol after mono. Correctness today rests entirely on the declaration order of two lines in a stdlib file with no comment warning against reordering.

- **Unifying duplicated logic by taking the *union* of the copies is a mistake, and F33h proved it on me.** Each of the 5 stdlib-location chains differed, and I merged them by keeping every step. But the LSP's `~/.jessup/bin/kestrel` symlink step exists *only* because a bundled VSIX binary can use neither the exe-relative nor the in-repo candidate — and at the LSP's position (3rd) it outranks in-repo. Result: on any machine with jessup installed, every repo-built `kestrel` silently compiled against the installed **0.16.0** toolchain's stdlib. 27 suite tests failed on stdlib features that toolchain predates (`extend Int64: Exitable` → E616 on `attributes.main.exitable.*`, closure kinds, a missing `Formatter`), with every diagnostic pointing at the *test files* and nothing failing at the resolver. Fixed by moving the symlink step last and pinning the outcome with `a_repo_build_resolves_to_the_repo_stdlib`, which asserts the resolved path *is* `repo_std_path()` — re-proved by swapping the order back. **Before collapsing N copies, diff them and establish what each difference was for.** A difference is a decision someone made, not noise.
- **F33i was under-rated `low`: it is a live, reproducible miscompile of correct code.** The audit filed it as an unread annotation. It is that, but the same line is also the bug: `lower_sugar_type` resolved the hardcoded string `"Array"` with `context: owner` — the *user's* scope. So `struct Array[T] {}` in a user file captured `[Int]`, and `let xs: [Int] = [1, 2, 3];` failed with `Array[Int64] !: _ExpressibleByArrayLiteral`. The four `@builtin(.*TypeOperator)` items existed precisely to prevent this and had zero readers. Fixed by resolving through the lang item and following the alias in the *stdlib's* scope; regression test `types/type_operators/array_operator/user_type_does_not_shadow_array_sugar.ks`. All four sugars (`[T]`, `T?`, `[K:V]`, `T throws E`) were affected.
- **F33k's wrong output was shipped, not hypothetical.** `docs/stdlib/std.core.md` published `public func fatalError(String) -> Never` — the language spells the never type `!`, and the source says `-> !`. Both copies of the renderer were wrong, so unifying them was not enough; the canonical one had to be corrected too. Verified by regenerating: the signature now reads `-> !`. Only that one line of the checked-in docs was updated — the other three files differ from a regeneration because of *other* agents' in-flight stdlib edits.
- **F41** did not silently corrupt memory. At HEAD a narrow atomic was a hard cranelift verifier error (`arg 1 (v8) has type i32, expected i64`) — narrow atomics were unsupported, not miscompiled.
- **F28** does not silently degrade. An unlowered intrinsic ICEs at post-mono verify (`Callee::Direct not resolved`). Still an ICE where a diagnostic belongs.
- **F7's stated failure mode did not reproduce, and it is lowered `medium` → `low`.** The
  claimed shim-vs-user-clone collision requires an out-of-line `clone()` that surfaces no
  `WitnessDef`. Four configurations were compiled and run (2026-08-08) with
  `KESTREL_DEBUG_CLONE=1`: inline `clone()`; `extend Handle { clone }` with conformance on the
  struct; `extend Handle: Cloneable { clone }`; generic `extend Box[T]: Cloneable where T: Cloneable`;
  and a cross-module `extend Lib.Handle: Cloneable`. In **every** case exactly one clone function
  was registered for the nominal, the user `clone()` won, and no shim was synthesized — the
  `clone_method_self_nominal` fallback at `expand.rs:258` already covers the extend cases. The
  `clone_shim.rs:218-223` comment ("doesn't *always* surface a witness") is hedged and no longer
  names a reachable case; **the double-free scenario is unsubstantiated.**
  What *is* live is a different defect: `build_clone_lookup`'s key `(nominal, type_args)` drops
  `parent_self`, which `InstantiationKey` carries. A trivial hello-world against the stdlib inserts
  **97 entries under 78 distinct keys — 19 silent overwrites per build**, e.g. six MonoFuncIds
  colliding on `(IoError, [])` and six on `(IoErrorKind, [])`. All colliding entries share the same
  `source` entity, so they are re-instantiations of one generic function and picking any is
  semantically equivalent — benign today. But it means **the audit's proposed "hard error on
  duplicate key" fix would ICE on every build**; the guard must fire only when two entries disagree
  on `source`. Repros: `temp/f7/*.ks`.
- **F34's proposed fix does not survive contact.** The audit says to "add a `check_block_local_defs` pass
  requiring every operand to be defined by that block's params or an earlier instruction in it." That was
  implemented and measured (2026-08-11): **121 of 354 `memory_model` tests fail**, and the reported
  violations are inside shipped stdlib bodies (`std.text.ClosedRange.readLines` and friends), not test code.
  MIR lowering uses the ordinary SSA dominance rule — a block may reference any definition that dominates
  it — so the "block-parameter live-in contract" claimed at `verify.rs:6` was never an invariant of this
  compiler. It is the *false premise* that justified a block-local ownership walk, and enforcing it would
  mean reworking lowering to thread every cross-block value through block parameters. The finding itself is
  unchanged; only the remedy is wrong. Fix the walk (dominance-aware, whole-function, modelling
  conditionally-consumed values) instead of the lowering.
- **F3 was understated, and is raised `medium` → `high`.** All four failure scenarios were reproduced by compiling and *running* programs (2026-08-08): the wrong-slot write on `S(a: 1, c: 3)` (prints `c=0`, no diagnostic at any stage), an E500 copy-fold false positive from a `static var` of a non-`Copyable` type, an E449 "cannot contain itself" false positive from a `static var` self-reference, and an **OSSA ICE** (`block arg 0 to BlockId(1): type mismatch`) on a struct pattern binding a computed property. The site count is ~15 real decision sites, not 8; three the audit missed are `mir-lower/items/mod.rs:60-67`, `mir-lower/body/expr.rs:1699`, and `mir-lower/body/mod.rs:2593`. `mir-lower/src/ty.rs:761` was miscited — it is inside `#[cfg(test)] mod tests`, not a production layout authority. `struct_cycles.rs`/`recursive_enum.rs` are `Field && !Callable`, missing *both* filters rather than just `Static`.
