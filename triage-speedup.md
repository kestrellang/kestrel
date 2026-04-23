# Triage Speedup

Original problem: `triage` launched one `file_tests --exact <one test>` subprocess
per test. Isolation was robust but expensive. On the 118-test `attributes`
subset, raw multithreaded took ~0.55s while `triage -j 4` took ~4.45s.

## Phase A — micro-optimizations (shipped)

- **In-memory status counters.** Previously every `TestCompleted` event
  re-queried SQLite for the full build summary. The counters are now seeded
  from `load_summary` once, then mutated in place as events stream in.
- **Skip log writes on pass.** Passing tests no longer create a log directory
  or hit `fs::write` twice — two syscalls per test is measurable when each
  test is only a few ms.
- **Adaptive poll backoff.** The subprocess wait loop now starts at 1 ms and
  grows to 50 ms instead of sleeping 100 ms every iteration. Fast tests no
  longer burn up to 100 ms of tail latency waiting for `try_wait` to notice
  they're done.

On the 118-test `attributes` subset, Phase A took `triage -j 4` from ~4.45s to
~3.63s (~18%). For longer-running `codegen`/`stdlib` tests the gains were
negligible — per-test stdlib initialization dominated.

## Phase B — batched worker strategy (shipped)

Root cause of the `codegen`/`stdlib` gap: each `file_tests` subprocess loads,
parses, and runs type inference on the Kestrel standard library. That's
~0.5–0.7s of fixed cost per subprocess. Raw `cargo test` amortizes it across
every test because they share one process; isolated triage paid it per test.

The batched strategy lets one subprocess execute many tests:

- **Harness change** (`lib2/kestrel-test-suite/tests/file_tests.rs`). Replaced
  the `datatest_stable::harness!` macro with a custom `main` built on
  `libtest-mimic` + `walkdir`. It adds a `--names-file PATH` flag — a file of
  one libtest name per line. When provided, only listed trials are run.
  Stock invocation (flags, `--list`, `--exact`, filters) is unchanged.
- **Strategy enum** in triage. `--strategy isolated|batch` (default `batch`)
  and `--batch-size N` (default 16, adaptively shrunk so small runs don't
  starve workers: `min(batch_size, test_count / (jobs*4))`).
- **`claim_batch`** atomically pulls up to N pending rows in one transaction
  and flips each from `pending` → `running`.
- **`run_batch`** spawns one `file_tests --test-threads=1 --names-file <tmp>`
  process per batch and streams stdout line-by-line. A `BatchParser` state
  machine maps `test NAME ... ok|FAILED|ignored` lines to results as they
  arrive and parses the tail `failures:` section for per-test failure blocks.
- **Crash handling.** A hard crash (abort/signal) or batch timeout:
  - The test that was mid-run at the time gets `crashed` / `timed_out` with
    the batch output attached.
  - Tests that had been claimed but had not yet started are moved back to
    `pending` so another worker picks them up. A new `TestRequeued` event
    keeps the in-memory counters honest (they slide `running → pending`
    instead of letting the test count as still-running).
- **Heartbeat loop.** While the subprocess runs, the worker heartbeats every
  run_id in its batch that hasn't reported a result yet, so the existing
  stale-reclaim logic still kicks in if the whole worker hangs.

Config is persisted in `.triage/config.toml` with `strategy` and `batch_size`
fields; env overrides `TRIAGE_STRATEGY` and `TRIAGE_BATCH_SIZE` work as well.

### Measured speedups (Phase A+B vs Phase A alone)

210-test `codegen.*` subset (stdlib-heavy):

| Strategy | Jobs | Wall    | User (CPU) |
|----------|------|---------|------------|
| isolated | 1    | 335.9 s | 280.7 s    |
| isolated | 4    |  86.7 s | 294.3 s    |
| batch    | 1    | 228.2 s | 191.3 s    |
| batch    | 4    |  63.1 s | 203.4 s    |

Batch is ~32% faster wall / 31% less CPU at -j1, ~27% faster wall / 31% less
CPU at -j4. The CPU drop is exactly the eliminated per-test stdlib init.

118-test `attributes.*` subset (cheap tests, no stdlib):

| Strategy | Jobs | Wall   | User (CPU) |
|----------|------|--------|------------|
| isolated | 1    | 4.69 s | 1.7 s      |
| isolated | 4    | 1.50 s | 1.7 s      |
| batch    | 1    | 1.72 s | 1.3 s      |
| batch    | 4    | 0.84 s | 1.4 s      |

Batch -j1 is 63% faster than isolated -j1 — subprocess spawn overhead
dominates here. Batch -j4 is 44% faster than isolated -j4.

## Remaining opportunities (not shipped)

- **JSON output from the harness.** `file_tests --format=json` (not yet
  exposed by libtest-mimic) would let us attribute failures without parsing
  pretty output. Would simplify `BatchParser`.
- **Work-stealing split.** At very high `-j` the static "claim up to N" can
  still leave a worker holding a long tail. A claim that can split an
  in-flight batch when other workers go idle would close the gap.
- **Fast-first-pass + isolated rerun.** Option 2 from the original sketch:
  run the full selection in a few big batches, then rerun only crashes /
  timeouts / stall-suspects in isolated mode. Useful for big CI runs.
