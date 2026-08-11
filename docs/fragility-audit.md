# Kestrel Fragility Audit — Tracking Checklist

Status of every confirmed finding from the 2026-08-08 fragility / single-source-of-truth audit.
**This file is the source of truth for what is fixed.** Update it in the same commit as the fix.

- Full findings with evidence and repro: [`fragility-audit-detail.md`](fragility-audit-detail.md) (1-43)
- Gap round + high-severity verdicts: [`fragility-audit-addendum.md`](fragility-audit-addendum.md) (G1-G17)

**Method.** 18 auditors across crate clusters and 6 cross-cutting dimensions, each finding
adversarially re-verified by an independent agent; then a completeness critic and 4 targeted
follow-ups. 118 candidates filed, 107 confirmed, 11 refuted. Findings 1-43 are the deduped
merge of 90 confirmed; G1-G17 are the gap round.

**Legend.** `fixed` done and verified · `partial` some sub-items done · `blocked` needs a
maintainer decision · `open` untouched. Severity is post-verification (verifiers corrected
inflated finder severities).

**Progress: 7 fixed · 2 partial · 2 blocked · 48 open** — 60 top-level (F1–F43, G1–G17).
F33 and F43 are roll-ups that expand into 19 independently-fixable sub-items, tracked
underneath them, so the real work item count is 79.

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
- Do not delete entries — mark them `fixed`. The ID is how other agents and commits refer to them.
- If verification disproves a finding, move it to **Ruled out** with the evidence, don't silently drop it.
- Severities here are post-verification. If you re-measure one, correct it and say so under
  **Corrections**, as was done for F41 and F28.

---


## Silent miscompilation and wrong behavior

- [ ] **F1** `high` `single-source-of-truth` — Range-overlap correction lives only in `check_match`; decision-tree codegen uses the raw overlap test and misroutes arms
- [ ] **F2** `high` `fragility` — LSP local rename replaces the whole `let` statement (and inserts at file offset 0 for parameters)
- [x] **F3** `high` `single-source-of-truth` — "Is this a stored instance field?" is re-derived in ~15 places with three different predicates — **fixed**
  - `FieldClass` component set once by the field builder; storage is stored, not derived from absent markers. Six sites migrated. All four failures (wrong-slot write, E500 FP, E449 FP, OSSA ICE) verified fixed by running. Design + decisions: [`docs/design/f3-stored-field-consolidation.md`](design/f3-stored-field-consolidation.md)
  - Fail-loud backstops landed too: struct construction binds labeled args by **name** (unknown label ICEs, never falls back to position), and the OSSA verifier requires every `InstKind::Struct` to supply each `FieldIdx` exactly once. 4 unit tests; neither fires anywhere in the suite, so they only catch new drift
  - Invariant recorded in `lib/kestrel-ast-builder/AGENTS.md`
- [ ] **F4** `medium` `fragility` — Escaping-closure box `init` is picked by arity alone; `RcBox` already has two 1-parameter inits
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
- [ ] **F8** `medium` `single-source-of-truth` — The LLVM backend never received the Bool-discriminant width fix that landed in cranelift (ef3fb801)
- [ ] **F9** `medium` `incremental-hazard` — `NominalCopySemantics`/`NominalStaticness` memos depend on a thread-local recursion stack that is not part of the cache key
- [x] **F10** `medium` `single-source-of-truth` — Static-member lookup truncates to the first `extend` block — **fixed**
  - `resolve_extension_static_method` now sources candidates from `TypeMembersByName` — the query that already declares itself the single source of truth for "what members does this type have?" — instead of its own staged `ExtensionsFor` + `ConformingProtocols` walk. That deletes the truncation at both levels (first matching extension, and first conforming protocol) in one move
  - Precedence follows the instance-member rule already shipped in `kestrel_type_infer::resolve_member`: `Direct`/`Extension` candidates compete equally, and a `ProtocolExtension` default joins the set only if its **label signature** is not already taken. A type's own `tag()` beats `extend SomeProtocol { static func tag() }` without the pair going ambiguous, and a protocol default with different labels stays reachable. Suppressing protocol-extension candidates outright would have been F10's mistake in the other direction
  - Rule recorded in `lib/kestrel-name-res/AGENTS.md` ("Member lookup goes through `TypeMembers`"), along with the inventory of remaining open-coded walks
  - `find_in_extensions` also merges across extensions now (its one remaining caller, `resolve_assoc_type_static_member`, only uses the result as an existence check — hir-lower discards the entity and re-resolves through `Field { base: Def(assoc_type) }`)
  - Verified by running: with the old first-extension-wins body restored, `Foo.make(b:)` split into a second `extend` block fails with "no matching overload"; with the fix it compiles and runs. 2 execution tests under `declarations/extensions/`; full suite green
- [ ] **F11** `medium` `single-source-of-truth` — The solver and the move checker ask `TypeParamCopyRequirement` with different `context`
- [x] **F12** `medium` `fragility` — Associated types on type params are matched by **name string** against ancestor where-clauses, and E439 can't see extension-target params — **fixed**
  - where-clause subjects now resolve through `ResolveName` in the *bearing entity's* scope and are
    compared by `Entity` (`SubjectParam` / `subject_denotes`); name is only a prefilter
  - `ExtensionLhsParams` query = THE answer to "which target params does the LHS bind", backed by an
    `ExtensionLhsParamNames` component written once at build time. Consumed by
    `check_type_param_shadowing` (E439 now fires for generic extensions) and
    `resolve_extension_type_param`; the RHS free-param scan shares the same list
  - closed a latent leak found while fixing it: `T` used to resolve inside `extend Box[Concrete]`
  - 4 tests under `types/generics/`, incl. a negative guard against false E439
- [ ] **F13** `medium` `single-source-of-truth` — A missing `;` after an expression statement in a function body is silently accepted
- [ ] **F14** `medium` `side-table` — Closure lowering's `SavedState` hand-mirrors `OssaBodyCtx`; three per-body fields are unsaved

## Diagnostics infrastructure

- [ ] **F15** `medium` `single-source-of-truth` — Every inference error is rendered twice by two divergent tables; the dedup rule lives in consumers and the LSP has none
- [ ] **F16** `low` `fragility` — Diagnostic-code registry has no ownership or reachability enforcement
- [ ] **F17** `low` `side-table` — Analyzer diagnostics bypass the world accumulator; `kestrel dump` drops them and exits 0
- [ ] **F18** `low` `fragility` — `kestrel dump mir` swallows the MIR-stage diagnostics it just accumulated

## Query-framework and incremental hazards

- [ ] **F19** `medium` `incremental-hazard` — `TypedBody`'s hand-written `Hash` omits five output fields and hashes `errors` by length only
- [ ] **F20** `medium` `ordering-dependency` — The LSP despawns *before* `begin_revision()`, erasing the only invalidation despawn produces
- [ ] **F21** `medium` `incremental-hazard` — `World::snapshot` clones query memos but resets the accumulator store — and both docs say the opposite
- [ ] **F22** `medium` `global-state` — Push/pop guards are not unwind-safe, and two hosts catch panics and keep the thread
- [ ] **F23** `low` `incremental-hazard` — Accumulated values are never pruned by revision or despawn

## Parser and CST integrity

- [ ] **F24** `medium` `fragility` — `add_token_or_missing` widens a diagnostic span by one raw **byte**, producing non-UTF-8-boundary offsets that make the LSP drop every diagnostic
- [ ] **F25** `medium` `fragility` — Module/import emitters re-derive `.` `(` `)` `,` `as` spans by byte arithmetic — and it fires on shipping stdlib source
- [ ] **F26** `medium` `single-source-of-truth` — Escape-sequence decoding is implemented three times, and one copy silently drops `\u` entirely
- [ ] **F27** `low` `single-source-of-truth` — `SyntaxKind`'s 258 variants are restated twice in the same file; a miss reads back as `Error`

## Remaining single-source-of-truth duplication

- [ ] **F28** `medium` `single-source-of-truth` — `lang.*` intrinsics are declared by a cross-product loop and lowered from a hand-written table; 105 names have no lowering — **partial**
  - coverage test added — gap measured at **121 of 367**; names still unlowered
- [ ] **F29** `low` `single-source-of-truth` — `@builtin(.X)` arguments are never validated; `Builtin`'s four hand-maintained tables already have a hole — **partial**
  - the `DefaultArrayLiteralType` hole is closed + guard test; **argument validation (E479) still open**
- [ ] **F30** `medium` `single-source-of-truth` — `desugar_logical_and` re-encodes the `SHORT_CIRCUIT_OP_PROTOCOLS` row and silently drops the RHS
- [ ] **F31** `medium` `fragility` — `lower_condition_chain` re-invokes `on_fail` per condition, duplicating diagnostics and lowering `else if` chains exponentially
- [ ] **F32** `low` `single-source-of-truth` — `substitute_resolved_ty` is a second, non-exhaustive copy of the substitution kernel
- [ ] **F33** `low` `single-source-of-truth` — Small duplicated predicates and magic strings
  - [ ] F33a — Trivia-kind set (`Whitespace | Newline | LineComment | BlockComment`) — 9 copies
  - [ ] F33b — `is_type_node` (14 variants) vs `is_type_kind` (12 — no `TyRef`/`TyMutRef`)
  - [ ] F33c — Root entity = the magic name `"<root>"`
  - [ ] F33d — `parent_is_type` (`Struct | Enum | Protocol | Extension`) — 7 copies
  - [ ] F33e — `member_lookup_name` (init/subscript sentinel)
  - [ ] F33f — Operator→`BinaryOp` map vs. parser's accepted-token list
  - [ ] F33g — `BinaryOp`→source text
  - [ ] F33h — Stdlib location precedence chain
  - [ ] F33i — Type-sugar `[T]`/`T?`/`[K:V]` binding
  - [ ] F33j — `BuiltinKind::Protocol`'s `requires_fields_conform` / `tuple_conformance_propagation`
  - [ ] F33k — kestrel-doc forks `kestrel_ast::pretty::format_type`

## Verifier and self-check gaps

- [ ] **F34** `medium` `fragility` — The OSSA verifier's linear-ownership check is block-local, and never runs after mono at all — **the prescribed fix is wrong; see Corrections**
  - The finding stands (the check *is* block-local and cross-block double-consumes are invisible). What is wrong is the proposed remedy: enforcing a "block-parameter live-in contract" would reject the stdlib. Measured 2026-08-11 — 121 of 354 `memory_model` tests fail, and the violations are in shipped stdlib bodies (e.g. `std.text.ClosedRange.readLines`). Lowering follows the ordinary SSA rule (use any *dominating* definition); the contract asserted at `verify.rs:6` is documentation of an invariant nobody maintains, and it is the reason the ownership walk was written block-local in the first place
  - Correct direction: make the ownership walk **dominance-aware / whole-function** (carry state along CFG edges, model conditionally-consumed values — Kestrel has drop flags precisely because a value may be consumed on one path only), rather than forcing every cross-block value through a block parameter
- [ ] **F35** `low` `side-table` — Borrow mutability lives only on the instruction, so threading a mut borrow through a block param downgrades it to shared
- [ ] **F36** `low` `side-table` — Mono's `WitnessCache` is built at collection cost, discarded with `let _ =`, and keyed by a lossy pair
- [ ] **F37** `medium` `fragility` — `find_inherited_assoc_type` recurses through protocol inheritance with no cycle guard its sibling has

## Editor and tooling

- [ ] **F38** `medium` `side-table` — `disk_line_indices` is a second copy of file text that `didOpen`/`didChange`/`didClose` never update
- [ ] **F39** `medium` `single-source-of-truth` — Completion open-codes member lookup instead of `TypeMembers`, missing every protocol-extension member
- [ ] **F40** `medium` `single-source-of-truth` — The two backends' `classify_named` disagree on a newtype over an aggregate field
- [x] **F41** `low` `single-source-of-truth` — Atomic RMW width is hardcoded `I64` in cranelift but taken from the operand in LLVM — **fixed**
  - width from value operand; 2 execution tests on both backends
- [ ] **F42** `medium` `global-state` — `unsafe impl Sync for StdlibCache` is unsound — **blocked**
  - not fixable by a Mutex — rowan CST refcounts are shared across snapshots; needs a design decision
- [ ] **F43** `low` `single-source-of-truth` — Smaller tooling defects
  - [ ] F43a — `Compiler::build` is call-once-per-entity but nothing enforces it
  - [ ] F43b — `PARAM_COUNTER` is a process-global counter whose doc claims it is reset
  - [ ] F43c — Diagnostic **message text** is built by iterating a `std::HashSet`
  - [ ] F43d — `module.witnesses` tail is appended in `HashMap` order
  - [ ] F43e — A type-blind copy of the irrefutability rule survives in analyze
  - [ ] F43f — Type-arg conformance failures deduped by a rendered display string
  - [ ] F43g — `ConformingProtocolInstantiations` dedup key embeds source spans
  - [ ] F43h — mir-lower accumulates E497/E503/ICE into a sentinel bucket

## Gap round (second pass)

- [ ] **G1** `medium` `ordering-dependency` — Init drop-flag setup reads `needs_drop` at Stage::Raw — before `drop_fix` populates it — and the `is_non_copyable` fallback misses every Cloneable aggregate (String, Array), so init field reassignment and failable-init failure returns silently leak
- [ ] **G2** `medium` `single-source-of-truth` — The mono layout work-list is seeded only from body VALUE types — it never walks `Op1/Op2/Op3` type operands or struct fields — so a type reachable only that way gets no `MonoStruct` at all, `verify_mono`'s missing-layout guard is structurally unable to fire, and codegen silently answers size 8 / offset 0
- [ ] **G3** `medium` `fragility` — The thunk pass identifies the closure environment parameter by the magic names `"env"`/`"_env"`; a user function whose first parameter is named `env`, used as a function value, has that parameter replaced by the environment pointer
- [ ] **G4** `medium` `single-source-of-truth` — `--target` reaches only `@platform` filtering; layout and both codegen backends hardcode the host, so `kestrel build --target <other-os>` silently emits a host binary compiled against the other OS's stdlib
- [ ] **G5** `medium` `single-source-of-truth` — Two independent pointer widths and two independent size tables: MIR layout is hardcoded to 8 bytes while codegen derives width from the triple; the one path that honors `--target` also compiles with the native ISA
- [ ] **G6** `medium` `fragility` — `@platform` fails open on every argument it does not recognize, and no validator for the argument exists anywhere — an unparsed or unsupported `--target` triple disables platform filtering entirely
- [ ] **G7** `low` `global-state` — `KESTREL_COPYPROP_LIMIT` silently changes emitted code from inside a MIR pass, and the documented set of output-affecting environment variables does not include it
- [ ] **G8** `medium` `single-source-of-truth` — "Does this loop diverge?" is decided three ways; `guard.rs` alone omits the break check, punching a hole in the E003 soundness gate
- [ ] **G9** `medium` `single-source-of-truth` — All three copies of `expr_contains_break` ignore `Break.label`, so a labeled break is attributed to the wrong loop — disagreeing with MIR's `find_loop`, which routes by label
- [ ] **G10** `medium` `single-source-of-truth` — `initializer.rs` runs a 5th, private loop model (`loop_break_stack`) that pairs every `break` with the innermost loop, so one labeled break disables the entire "all fields initialized" check
- [ ] **G11** `low` `fragility` — `dead_code.rs` never handles `HirExpr::Sugar`, so E002 is structurally blind inside every `for`-loop body in the language
- [ ] **G12** `medium` `single-source-of-truth` — The Never-typed divergence rule is copy-pasted into five analyzers; only `move_tracking` carries the documented `Loop` carve-out, and `dead_code` reads no types at all
- [ ] **G13** `medium` `single-source-of-truth` — The extension-bound evaluator SKIPS `Copyable`/`Cloneable` clauses because `type_satisfies` cannot answer them; conformance-completeness calls `type_satisfies` on exactly those clauses anyway and gets a hard `false`
- [ ] **G14** `medium` `single-source-of-truth` — A where-clause param the substitution can't map is a PERMIT in the solver's evaluator and a REJECT in the analyzer's, so every associated-type-subject clause on a protocol extension (`extend Iterator where Item: Equatable`) is unentailable
- [ ] **G15** `low` `fragility` — `constraint_entailed_by`'s "param-declared bounds" tier queries `WhereClausesOf` on the TypeParameter entity, which never carries a where clause — the whole branch is unreachable
- [ ] **G16** `medium` `single-source-of-truth` — E101's condition-conformance test is a private `ConformingProtocols` lookup that only understands `ResolvedTy::Named`, so `if` on a `T: BooleanConditional` param or on `Self` is a false error
- [ ] **G17** `medium` `single-source-of-truth` — `TypeResolver` re-derives where-clause bounds from raw `AstWhereClause` instead of `WhereClausesOf`, collapsing `T.Assoc: P` onto the bare associated type and resolving subjects in the ambient `body_owner` scope

---

## Ruled out (do not re-file)

Refuted during verification. Re-raise only with new evidence.

- `kestrel-copy-fold` is genuinely one decision tree; the apparent `FnKind::Mutating` divergence is documented, self-consistent with `needs_drop`, and pinned by a shipped test.
- hECS `QueryKey` is not a second identity decision — derived from the memo key by one function, and all 59 query key types derive `Hash` structurally.

## Corrections to earlier severity claims

Established by running the code, not reading it:

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
