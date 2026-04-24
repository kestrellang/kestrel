---
name: triage
description: Low-token operational guide for the `triage` CLI. Use when the user says run triage, triage status, rerun failures, run tests in the background, inspect triage history, or asks about `.triage/` / `triage.db`.
---

# triage

Use `triage` for broad Kestrel test runs. It wraps `kestrel-test-suite`, stores results in `.triage/triage.db`, skips tests already recorded for the current build, and is safe alongside other agents.

## Default Moves

```bash
triage 'codegen.*'              # run matching tests in foreground
triage 'codegen.*' -j 4         # parallel workers; quote patterns with *
triage --async                  # full suite in background
triage status                   # current build progress
triage status --failures        # non-passing tests
triage status --messages        # failures plus messages
triage history some.test.path   # timeline for one test
triage builds                   # recent builds
triage cancel <build_id>        # mark pending/running rows canceled
```

Prefer `triage --async` for long runs. **Do not poll `triage status` in a loop** — the full suite takes ~10 min and every poll is a round-trip. Use `Monitor` with an until-loop that exits when `running|pending` drops from the counts line:

```bash
until s=$(triage status <build_id> 2>&1 | grep '^Counts:'); ! echo "$s" | grep -qE "running|pending"; do sleep 8; done; echo "DONE: $s"
```

The async command only reports detach success; real results are in the DB.

## Iteration Loop

For fixing a failing test, **run the targeted pattern, not the full suite**:

```bash
triage 'validation.initializers.*'     # seconds
triage --async                          # ~10 min
```

Reserve the full suite for pre-commit validation. Every edit-verify cycle on `--async` is ~10 min of wall clock.

## Build Profile Must Match

`.triage/config.toml` specifies the build (usually `--release`). A debug `cargo build -p kestrel-analyze` **does not affect triage** — the release binary is what runs. When iterating on analyzer changes:

```bash
cargo test --release -p kestrel-test-suite --no-run
```

After the release rebuild, a new triage run picks up the change (source edits invalidate the build hash → fresh rows).

## Results Are Cached Per Build

Triage keys results by build hash. Running `triage <pattern>` with no source change returns the same results — it won't re-execute already-recorded rows. To force re-run, edit a source file (the build hash changes) or use `triage cancel <build_id>` on the prior rows.

## Patterns

Patterns match dotted test paths. `*` matches any sequence, including dots.

```text
*                         all tests
codegen.*                 everything under codegen
*.enums.*                 any path containing an enums segment
declarations.structs.x    exact test
```

Shells expand `*`, so quote wildcard patterns.

## Inspect Output

### Get full output

```bash
triage status --messages
```

### Get status

```bash
triage status <build_id>
```

## Useful Flags

```bash
triage '*' --json
triage status --json
triage status --json --jq '.counts'
triage '*' --batch-size 32
triage '*' --strategy isolated
```

Use default batching unless isolating a crash, hang, or flaky test. Default `-j` is intentionally conservative because some codegen tests collide at higher parallelism.

## Gotchas

- `triage` always builds before scheduling; this can take a few seconds.
- Any current-build row counts as done, including `canceled`, `skipped`, or `hung`.
- Source edits create a new build hash and a fresh set of rows.
- Multiple agents can run triage concurrently; total parallelism is the sum of all `-j` values.
- `.triage/triage.db` is the source of truth; logs live under `.triage/logs/`.
- `kestrel dump diagnostics` does NOT include analyzer diagnostics — the compiler CLI only emits codespan-level diagnostics. Analyzer output (E005, E302, etc.) only surfaces via the test harness. To verify an analyzer change, run the target test(s) through `triage`.
