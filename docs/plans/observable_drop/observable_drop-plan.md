# Observable Drop Implementation Plan

## Test strategy
- Diagnostic: E508 fires on an unmarked `Cloneable` type whose deinit reaches a
  non-memory extern (directly and through a Kestrel wrapper); no warning when
  marked, when the deinit only frees, or when the type is `not Copyable`.
- Execution: a marked `Cloneable` type's copy is dropped at scope end, not at
  its last use (`deinit` order observable via `println`); an unmarked
  memory-only type still gets the early drop (COW mutation stays in place).
- Location: `lib/kestrel-test-suite/testdata/memory_model/observable_drop/`.

## Phases

### Phase 0 — Tests
- [ ] `lint_unmarked_cloneable_close.ks` (WARN), `lint_through_wrapper.ks`
      (WARN), `lint_marked_no_warning.ks`, `lint_free_only_no_warning.ks`,
      `lint_not_copyable_no_warning.ks`
- [ ] `exec_marked_copy_lives_to_scope_end.ks`,
      `exec_nested_observable_in_rcbox.ks`

### Phase 1 — Attribute plumbing (rename + invert)
- [ ] `KNOWN_ATTRIBUTES`: `memoryonlydrop` → `observabledrop`
- [ ] `MirModule`/`MonoModule::observable_drop`, `record_observable_drop`
- [ ] `TypeFacts::observable` (fields, payloads, type args, pointees)
- [ ] `drop_at_last_use` uses `!observable`
- [ ] Stdlib marks; `lib/kestrel-mir/AGENTS.md` section

### Phase 2 — Lint E508
- [ ] `lib/kestrel-analyze/src/decl/observable_drop.rs`, `AnalyzerId`, registration
- [ ] `AGENTS.md` ID list: E508

## Verification
- [ ] Targeted triage on `memory_model.observable_drop.*` (blocked: triage not
      installed in the cloud container — fall back to `kestrel build` + run,
      report as unverified by the harness)
- [ ] Benchmarks unchanged (`temp/perceus/long/`)

## Revision — E508 moved post-monomorphization
The analyzer version (Phase 2) could not follow generic/protocol calls, so a
deinit that only prints was missed. Replaced by
`kestrel_mir::passes::copy_lifetime::unmarked_observable_drops`, reported via
`Compiler::mono_warnings` (printed by `kestrel build`). The lint tests became
execution tests; the harness checks `// WARN:` in execution tests (and
`// warnings: checked` asserts none).
