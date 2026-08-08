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

**Progress: 4 fixed · 2 partial · 2 blocked · 51 open** — 60 top-level (F1–F43, G1–G17).
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
  - Not yet done: the two fail-loud backstops (name-checked struct construction, `InstKind::Struct` verifier rule) that would make a *future* divergence loud
- [ ] **F4** `medium` `fragility` — Escaping-closure box `init` is picked by arity alone; `RcBox` already has two 1-parameter inits
- [x] **F5** `medium` `fragility` — `break`/`continue` validation leaks across the closure boundary; MIR then silently no-ops the `break` — **fixed**
  - `loop_labels` save/restore in `lower_closure` + MIR backstop diagnostic
- [x] **F6** `medium` `single-source-of-truth` — `Copyable`/`Cloneable` lang protocols are identified by `name.ends_with(...)` in five MIR sites — **fixed**
  - lang items on MirModule; all 5 `ends_with` sites gone
- [ ] **F7** `medium` `single-source-of-truth` — "Does T have a user clone?" is answered by witnesses in `clone_shim` but by method name in `expand`; the lookup silently last-write-wins
- [ ] **F8** `medium` `single-source-of-truth` — The LLVM backend never received the Bool-discriminant width fix that landed in cranelift (ef3fb801)
- [ ] **F9** `medium` `incremental-hazard` — `NominalCopySemantics`/`NominalStaticness` memos depend on a thread-local recursion stack that is not part of the cache key
- [ ] **F10** `medium` `single-source-of-truth` — Static-member lookup truncates to the first `extend` block
- [ ] **F11** `medium` `single-source-of-truth` — The solver and the move checker ask `TypeParamCopyRequirement` with different `context`
- [ ] **F12** `medium` `fragility` — Associated types on type params are matched by **name string** against ancestor where-clauses, and E439 can't see extension-target params
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

- [ ] **F34** `medium` `fragility` — The OSSA verifier's linear-ownership check is block-local, and never runs after mono at all
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
- **F3 was understated, and is raised `medium` → `high`.** All four failure scenarios were reproduced by compiling and *running* programs (2026-08-08): the wrong-slot write on `S(a: 1, c: 3)` (prints `c=0`, no diagnostic at any stage), an E500 copy-fold false positive from a `static var` of a non-`Copyable` type, an E449 "cannot contain itself" false positive from a `static var` self-reference, and an **OSSA ICE** (`block arg 0 to BlockId(1): type mismatch`) on a struct pattern binding a computed property. The site count is ~15 real decision sites, not 8; three the audit missed are `mir-lower/items/mod.rs:60-67`, `mir-lower/body/expr.rs:1699`, and `mir-lower/body/mod.rs:2593`. `mir-lower/src/ty.rs:761` was miscited — it is inside `#[cfg(test)] mod tests`, not a production layout authority. `struct_cycles.rs`/`recursive_enum.rs` are `Field && !Callable`, missing *both* filters rather than just `Static`.
