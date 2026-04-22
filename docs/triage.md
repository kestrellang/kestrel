# triage

Tracks the progress of the test suite across runs, prevents multiple agents from doing irrelevant runs. Groups tests by their root cause of failure.

## Tech Stack

triage is a single-binary Rust CLI backed by a SQLite database. The binary is `triage`, installed via `cargo install` or built from the workspace.

Data lives in `.triage/` at the repo root:

- `.triage/triage.db` — the SQLite database.
- `.triage/config.toml` — per-project configuration; see [Configuration](#configuration).
- `.triage/.gitignore` — self-ignoring, so the directory doesn't get committed.
- `.triage/logs/<test_run_id>/{stdout,stderr}` — captured output from each invocation of the test binary.
- `.triage/binaries/<invocation_id>/{file_tests,pid}` — per-invocation scratch copy of the built test binary and its owner PID file; cleaned up on graceful exit or by the next startup sweep.
- `.triage/runs/<build_id>-<shortid>.log` — output stream from a detached `--async` run. Format matches whatever the foreground command would have emitted (text by default, NDJSON if `--json` was set).

Override the DB location with `--db PATH` or `TRIAGE_DB=PATH`; the other subdirectories are always siblings of the DB file.

The database runs in WAL mode so multiple agents can read while one writes — without this, concurrent agents serialize on the default rollback journal. All timestamp columns (shown as `timestamptz` below) are stored as ISO-8601 TEXT; SQLite has no native timestamptz type.

**Repo root discovery.** triage walks up from the invocation's working directory looking for an existing `.triage/` directory. If none is found, the first `.git/` it encounters becomes the repo root and a fresh `.triage/` is created there with the gitignore, default config, and an empty DB. This means subcommands work from any subdirectory of the project.

**First-run init.** When triage creates the `.triage/` dir it writes `.triage/.gitignore` (contents: `*`, self-ignoring), `.triage/config.toml` with defaults, and `.triage/triage.db` with the schema applied via the initial migration.

## Configuration

Per-project settings live in `.triage/config.toml`. The file is committed if the user wants it shared across the team, or left gitignored along with the rest of `.triage/` — their choice. Defaults cover the common case; only non-default values need to appear in the file.

```toml
# Cargo package whose test binary triage should build and run.
package = "kestrel-test-suite2"

# Glob (relative to target/release/deps/) for locating the built binary.
binary_glob = "file_tests-*"

# Prefix stripped from libtest test names to form the stored path
# (also re-prepended when invoking the binary with --exact).
harness_prefix = "run_ks_test::"

# Extension stripped from libtest test names to form the stored path.
test_extension = ".ks"

# Working directory for invoking the test binary, relative to the repo root.
# datatest-stable resolves testdata/ relative to cwd, so this must be the
# directory containing testdata/.
binary_cwd = "lib2/kestrel-test-suite"

# Cargo build command (invoked from the repo root). "--no-run" is appended.
build_command = ["cargo", "test", "-p", "kestrel-test-suite2", "--release"]

# Stall threshold in seconds — a running test_run row whose heartbeat is
# older than this is reclaimed as `hung` by the next worker that scans.
stall_threshold_seconds = 30

# Parallelism default when neither -j nor TRIAGE_JOBS is set.
jobs = 4
```

A triage built outside the kestrel repo picks up whatever `config.toml` the target project supplies; the defaults above are chosen to match kestrel but the intent is portability.

## Schema migrations

The DB uses forward-only migrations. A top-level `schema_migrations(version INTEGER PRIMARY KEY, applied_at TEXT)` table records which migrations have been applied. Migration files are numbered `.sql` files bundled into the triage binary at compile time; on startup triage applies any migrations whose version is not yet in `schema_migrations`. The entire migration sequence runs inside `BEGIN IMMEDIATE ... COMMIT`, so two triages starting simultaneously serialize cleanly — the second waits for the first to finish, then finds the migrations already applied and proceeds. There is no rollback — if you need to reverse a change, write a new forward migration. Schema changes to this document require a corresponding migration file.

## Build

triage considers each build of the test suite / project as a separate build. Builds are stored in the builds table, which looks like this:

```
build {
  id:          uuid;
  binary_hash: string;       // sha256 of the built test executable
  commit_sha:  string;       // git HEAD at build time
  branch:      string?;      // git branch at build time, if on one
  dirty:       boolean;      // true if the working tree had uncommitted changes
  created_at:  timestamptz;
}
```

Every `triage` invocation builds the configured test package unconditionally (cargo handles incremental rebuilds, so no-op rebuilds are cheap). If `cargo test --no-run` exits non-zero, triage prints cargo's output, returns cargo's exit code, and writes nothing to the database — no `build` row, no `test_run` rows, no scratch dir. A build failure leaves the DB in exactly the state it was in before the invocation. It then (on success):

1. `sha256`s the produced test executable → `binary_hash`.
2. `INSERT OR IGNORE` into `build`. If the hash already exists, the existing row's `build_id` is reused; otherwise a new row is created with the current git `commit_sha`, `branch`, and `dirty` state.
3. Copies the executable into a per-invocation scratch dir (`.triage/binaries/<invocation_id>/file_tests`) and runs every test against that copy for the duration of the invocation. This insulates the run from a subsequent `cargo build` stomping the artifact mid-run.
4. Deletes the scratch dir on graceful exit. On startup, triage sweeps `.triage/binaries/*` whose owning PID (recorded in a `pid` file alongside each binary) is no longer alive — any invocation that crashes or gets killed leaves behind a binary that the next `triage` cleans up.

Note that because every invocation rebuilds, two `triage` calls a minute apart against the same source produce the same `binary_hash` and re-use the same `build` row — the build step is cheap when nothing has changed, and the DB stays deduplicated.

## Test

Each test in the suite has a stable identity across builds. The `test` table is the source of truth for that identity and for any per-test metadata that outlives a single build (quarantine flags, skip reasons for known-hanging tests, etc.). Rows are created lazily — the first time a given `path` is observed, an entry is inserted.

The `path` is a dotted identifier derived from the libtest name. For the libtest emission `test run_ks_test::attributes/declarations/case_with_attribute_and_args.ks ... ok`, the stored path is `attributes.declarations.case_with_attribute_and_args`. The transform is fixed and reversible:

- **store:** strip the `run_ks_test::` prefix, strip the trailing `.ks`, replace `/` with `.`.
- **invoke:** replace `.` with `/`, append `.ks`, prepend `run_ks_test::`, and pass the result to the test binary with `--exact`.

The dotted form is the identifier users type (`triage declarations.structs.*`, `triage *.enums.*`). This assumes no directory or file name under `testdata/` contains a literal `.` — currently true and worth keeping as a convention.

```
test {
  id:               uuid;
  path:             string;       // dotted identifier, e.g. "declarations.structs.empty_struct"
  first_seen_build: uuid;         // fk: build.id
  last_seen_build:  uuid?;        // fk: build.id — set when the test stops appearing in --list
  removed_at:       timestamptz?; // non-null means the test file no longer exists in the binary

  quarantined:      boolean;      // excluded from normal runs
  skip_reason:      string?;      // e.g. "hangs in macOS UNE kernel state"

  @unique(path)
}
```

### Test file deletion

During discovery (see [Running Tests](#running-tests)), triage compares the paths returned by `$BIN --list` against the `test` rows that have `removed_at IS NULL`. The diff drives three operations:

- **New path (in list, not in `test`)** → insert a row with `first_seen_build = <current build>`, `removed_at = null`.
- **Missing path (in `test`, not in list)** → update `removed_at = now()` and `last_seen_build = <previous build id for this path>`. Scheduling from that point onward ignores the row — no new `test_run` rows are created, and quarantine is moot for it.
- **Revived path (in list, but existing `test` row has `removed_at` set)** → clear `removed_at` and `last_seen_build`. The old `test_run` history stays attached to the same `test.id`, so a deleted-then-restored test keeps its history.

Historical `test_run` rows still join cleanly to the retired `test` row — there's never an orphan and no foreign key ever breaks. `SELECT * FROM test WHERE removed_at IS NULL` is the live set; the full table is the audit trail of every test the suite has ever contained.

Renames are not tracked as renames. A rename looks like one path disappearing and a different path appearing, and triage treats them that way: the old row goes retired, the new row starts with its own `first_seen_build` and no history. Correlating the two is a human call, not a heuristic.

## Test Run

When triage is run, it creates a new test run for each of the tests in the test suite. If that test has already been run for the current build, it skips it. Each run row carries both the scheduling info and the eventual result — there is no separate result table.

```
test_run {
  id:              uuid;
  build_id:        uuid;          // fk: build.id
  test_id:         uuid;          // fk: test.id

  created_at:      timestamptz;
  started_at:      timestamptz?;
  completed_at:    timestamptz?;

  status:          enum;          // pending | running | passed | failed | skipped | timed_out | hung | crashed | panicked | canceled
  exit_code:       int?;
  duration_ms:     int?;
  failure_message: string?;       // raw text from libtest's "failures:" block when status != passed; null otherwise

  worker_id:       string?;       // "{hostname}:{pid}:{uuid}" — set when a worker claims the row
  heartbeat_at:    timestamptz?;  // last-seen timestamp while running; a stale value means the worker died or the test hung

  @unique(build_id, test_id)
}
```

Captured stdout and stderr live on disk, not in the database: `.triage/logs/<test_run.id>/stdout` and `.triage/logs/<test_run.id>/stderr`. The paths are derivable from the row's id, so there are no `stdout_path` / `stderr_path` columns — one source of truth. A missing file means the process produced nothing on that stream (common for passing tests).

## CLI

Running tests is the default action — there is no `run` subcommand, and there is no separate `list` command. `triage <pattern>` handles everything: build, schedule, execute. Everything else is a subcommand.

```
triage [pattern]                 # default: build, schedule, run (sync). Pattern defaults to "*".
triage [pattern] --async         # same, but detach and return immediately
triage cancel <build_id>         # stop an in-flight run; see below
triage status [build_id]         # build summary: counts, in-flight, pending. Defaults to current build.
triage history <test>            # this test's test_run rows across recent builds
triage builds                    # recent build rows with pass/fail totals
triage quarantine <test> <reason>
triage unquarantine <test>
```

A positional first argument is interpreted as a subcommand if it matches one of `status`, `history`, `builds`, `quarantine`, `unquarantine`; otherwise it is treated as a pattern. Test path segments therefore cannot collide with subcommand names — not a live concern today (top-level dirs under `testdata/` are all multi-word) but worth preserving as a convention.

**Pattern syntax.** Dotted identifiers matching `test.path`. `*` is the only wildcard and matches any sequence of characters, including dots — so `declarations.*` matches every test under `declarations/` regardless of depth, `*.enums.*` matches any test whose path contains an `enums` segment, and `*` alone matches everything. Patterns are expanded against the `test` table in SQL (translated to `LIKE` with `%` substituted for `*`) — the test binary's own substring filter is not involved.

**Global flags:**

- `-j N` / `--jobs N` — parallelism; env `TRIAGE_JOBS`; default 4.
- `--db PATH` — SQLite path; env `TRIAGE_DB`; default `.triage/triage.db`.
- `--binary PATH` — explicit path to a `file_tests-*` executable; otherwise triage finds the most recent one under `target/release/deps/`.
- `--json` — emit machine-readable output instead of text. See below.
- `--jq <expr>` — shell out to `jq` with the given expression against the JSON output. Implies `--json`. Errors if `jq` is not on `$PATH`.
- `--async` / `-a` — on the run command, detach and return immediately instead of waiting. See Execution Modes.

### Execution modes

The run command (`triage [pattern]`) has two modes, driven by the audience. Users want to watch it go; agents want to fire-and-forget.

**Sync (default)** — foreground execution, intended for humans. Workers run in the current process; when they're done, triage prints the final summary to stdout and exits. While running:

- If stderr is a TTY, a single-line progress bar is drawn on stderr (carriage-return updates, ANSI colors): `[===>        ] 123/456 · 3 failed · 0:12`. Per-test completion messages are *not* streamed — only the progress bar updates.
- If stderr is not a TTY, the progress bar is suppressed. triage runs silently until done, then prints the summary.
- The final summary goes to **stdout** either way, so `triage > results.txt` produces a clean file with no ANSI and no progress noise.
- Exit code is 0 if all scheduled tests passed, non-zero if any failed.

**Async (`--async` / `-a`)** — detached execution, intended for agents. triage schedules the work, forks a detached worker process that survives the parent, writes its output to a log file, and the foreground invocation exits immediately with a short report:

```
Build:    <build_id>
Output:   .triage/runs/<build_id>-<shortid>.log
Progress: triage status <build_id>
```

The detached process runs workers to completion (or until killed) and writes the same summary + per-event lines to the log file. Agents can poll `triage status <build_id>` against the database for structured progress, or `tail -f` the log for the raw stream. Exit code of the foreground invocation is 0 if the detach succeeded; the actual test outcome is derived from the DB later.

Multiple async invocations against the same build are fine — they each get their own log file (the `<shortid>` disambiguates) and their workers contend for claims through the same SQL-level mechanic as everything else. The Concurrency & Multiple Instances section covers the details.

`--async` only applies to the run command; the read-only subcommands (`status`, `history`, `builds`) always return synchronously.

### Canceling a run

`triage cancel <build_id>` is the graceful way to stop an in-flight run — especially an async one where there's no terminal to ctrl-C.

Semantics:

- Every `test_run` row for that build with `status = 'pending'` is flipped to `'canceled'`. No new workers will pick these up.
- Every `test_run` row with `status = 'running'` is also flipped to `'canceled'`. The worker currently executing it is still running its subprocess — when it finishes and tries to `UPDATE ... WHERE status = 'running'`, zero rows match, and the worker detects the cancellation, writes no result, and exits its loop. In-flight test binaries are not killed; they run to completion and their output is discarded.
- After a cancel, the build's `test_run` rows are a mix of the statuses that had already completed (`passed`, `failed`, etc.) plus the freshly-flipped `canceled` rows. `triage status <build_id>` reflects the new state immediately.
- Canceling a build that has no pending or running rows is a no-op; triage prints how many rows it affected (zero).

Cancel is DB-only — it does not send signals to worker processes, and it does not need to know which process owns which row. That keeps it cross-host-safe and race-free.

### JSON output

Every subcommand supports `--json` for machine-readable output. The shape depends on whether the command is point-in-time or streaming:

- **Point-in-time** (`status`, `history`, `builds`, `quarantine`, `unquarantine`) — emits one JSON document to stdout, then exits. `--jq` runs once against that document.
- **Streaming** (`triage [pattern]` — the run command) — emits NDJSON, one object per line, one line per event (worker started, test_run completed, worker idle, etc.) plus a terminating summary line. `--jq` pipes the stream through `jq` line-by-line, so filters like `triage '*' --jq 'select(.kind == "failed") | .path'` work as live filters over in-flight results. The text progress output is suppressed when `--json` is active.

A `triage status --json` invocation during an active run still returns a point-in-time snapshot — the streaming mode is specific to the run command.

**Streaming event kinds.** Every NDJSON line emitted by a run includes a `kind` field so consumers can filter without inspecting shape. The enumerated kinds are:

- `build_started` — the build row is resolved (new or reused) and pattern expansion is done. Payload includes `build_id`, `pattern`, `test_count` (how many tests matched and are scheduled).
- `worker_spawned` / `worker_idle` — worker lifecycle. Payload includes `worker_id`.
- `test_started` — a worker has claimed a row and spawned the binary. Payload includes `test_run_id`, `test_path`, `worker_id`.
- `test_completed` — the binary exited and the row has been updated. Payload includes `test_run_id`, `test_path`, `status`, `exit_code`, `duration_ms`, and (on failure) `failure_message`.
- `build_summary` — terminal line. Payload includes counts by status and total wall-clock duration.

New kinds may be added without a version bump; consumers should ignore `kind` values they don't recognize. Renaming or removing a kind is a breaking change.

**Stability contract.** The per-subcommand JSON shapes are a public interface. Field additions are backwards-compatible; field renames and removals are breaking changes and get a version bump. Agents and scripts should depend on field names, not field order. (The exact shapes are defined per-subcommand in the implementation and not reproduced here — they'll live next to the code.)

## Running Tests

`triage [pattern]` expands the pattern against the `test` table, drops any test that already has a `test_run` row for the current build (regardless of outcome — passed, failed, skipped, hung, etc. are all "done"), and schedules the remainder. That's the whole "don't do irrelevant runs" mechanic: the DB remembers, and any test with a current-build row is skipped automatically.

Each test is executed as a **separate invocation** of the test binary. One process, one test, one `test_run` row.

**Why not a single invocation with a filter list.** libtest's single-process model gives you one exit code and one duration for the whole batch — useless for per-test metadata. A crash or hang in test *N* also kills every test after it in the batch, so a single bad test can mask dozens of other failures. Per-invocation isolation costs a few tens of milliseconds of process startup per test (negligible compared to actual test time) and buys: real per-test exit codes, real per-test wall-clock durations, and a blast radius of exactly one test when something goes wrong.

### Discovery

On the first run against a new `build_id`, triage calls the binary once with:

```
$BIN --list --format=terse
```

This prints every test name libtest knows about, one per line. triage transforms each into the dotted form (see [Test](#test)) and upserts it into the `test` table. From then on, all scheduling works off the rows in `test` — the binary is never asked to enumerate again for that build.

### Invocation

For each pending `test_run`, a worker spawns:

```
$BIN --test-threads=1 --exact run_ks_test::<path_with_slashes>.ks
```

The worker wall-clocks the process, writes its stdout to `.triage/logs/<test_run.id>/stdout` and stderr to `.triage/logs/<test_run.id>/stderr`, and parses the libtest output for the single `... ok` / `... FAILED` token plus (on failure) the contents of the `failures:` block. It then writes `status`, `exit_code`, `duration_ms`, and `failure_message` back onto the row.

Per-invocation wall clock is the source of truth for `duration_ms` — libtest does not emit per-test timing. `exit_code` is the test binary's own exit code (0 for pass, 101 for any failure/panic, a signal number for crashes), meaningful precisely because each invocation runs exactly one test.

**Hard crashes with no libtest output.** If the test binary dies before emitting `... ok` or `... FAILED` (segfault, SIGBUS, OOM kill), the worker sees a nonzero exit code or a signal and no parseable result line. It records `status = crashed`, `exit_code = <signal or code>`, and synthesizes a `failure_message` of the form `"no libtest output; exited with signal <N>"` or `"no libtest output; exit code <N>"`. The stdout and stderr files still hold whatever the binary managed to write before dying — often the actual crash reason.

### Parallelism

triage runs N worker processes in parallel within a single invocation. Because the claim model is a row-level uniqueness check, workers do not coordinate with each other directly — they race for rows in SQLite and the loser moves on. This is also how parallelism composes across multiple agents: two `triage` invocations against the same database just add more workers to the pool, and the claim constraint keeps them from double-running.

- **Default:** `4` — a conservative fixed value that leaves headroom for the OS, the IDE, and the scanning worker that detects stalls on any machine triage is likely to run on.
- **Override:** `--jobs N` / `-j N` on the CLI, or `TRIAGE_JOBS=N` in the environment. CLI wins over env.
- **`-j 1`** runs serially — useful when debugging flakes that only reproduce in isolation, or when a test's resource usage would thrash under contention.
- triage does not try to be clever about CPU pinning or performance/efficiency cores; the OS scheduler handles that.

## Concurrency & Multiple Instances

triage has no daemon. Coordination — between workers inside one invocation and between separate invocations on the same machine — happens entirely through SQLite. The shared primitive is the `@unique(build_id, test_id)` constraint on `test_run`.

### How workers claim a test

A worker picks an unrun test and tries to `INSERT` a `test_run` row with `status = running`, its own `worker_id`, and `heartbeat_at = now()`. If the insert succeeds, the worker owns the run. If the insert fails with a uniqueness violation, another worker already claimed it — the current worker moves on to the next test. No advisory lock table, no separate claim record; the row itself is the claim.

While the test is executing, the owning worker periodically updates `heartbeat_at` on its row (e.g. every few seconds). This is what distinguishes a live run from an abandoned one.

### Detecting stalls

Before a worker picks its next test, it scans for rows where `status = running` and `heartbeat_at` is older than some threshold (a few multiples of the heartbeat interval). Those rows represent workers that died, tests that hung, or runs that crashed hard enough to skip the normal completion path. The scanning worker flips them to `status = hung`, sets `completed_at = now()`, and leaves `worker_id` in place so it's clear who abandoned the run. After that, the `@unique(build_id, test_id)` constraint still holds — the test is not re-queued — but the row is no longer blocking anything.

The stall threshold is `config.toml`'s `stall_threshold_seconds` (default 30s). It must be larger than the longest legitimate test duration plus a margin; otherwise live work gets reclaimed out from under its owner. 30s is conservative for the kestrel suite where individual tests complete in well under a second — projects with slower tests should raise it in their config.

### Multiple simultaneous invocations

Two people typing `triage` at the same time (or one person plus some number of agents) is a first-class scenario. The mechanics:

- **Build step.** Both invocations run `cargo test --no-run`. Cargo's own build lock serializes them. Both then `sha256` the resulting binary, get the same hash, and `INSERT OR IGNORE` the same `build` row. No race.
- **Discovery.** If the build is new, both call `--list` and upsert `test` rows. `@unique(path)` + `INSERT OR IGNORE` absorbs the overlap; both end up with the same set.
- **Scheduling.** Each invocation expands its pattern to a set of `test_id`s and tries to insert `test_run` rows with `status = running`. A row that already exists — because another invocation already scheduled it, already completed it, or is currently running it — causes the insert to fail, and the losing invocation simply moves on. Scheduling is idempotent: `triage` and `triage declarations.*` running side-by-side just partition the overlapping work between them.
- **Execution.** Each invocation spawns its own worker pool (default 4). Workers from different invocations are indistinguishable to the claim mechanic; they all contend for the same `test_run` rows under the same uniqueness constraint. Total parallelism is the sum of the instances' `-j` values.
- **Reads are always safe.** `triage status`, `triage history`, `triage builds` are read-only queries against a WAL-mode SQLite — they run concurrently with active workers without contention.
- **Ctrl-C on one invocation.** That invocation's workers die with claims still held. Heartbeats stop updating. The *other* invocation's stall scanner finds the rows, flips them to `hung`, and moves on. No stranded tests, no special signal handling, no coordination between processes.

**What this guarantees.** No two workers — within one invocation or across invocations — ever run the same `(build, test)` pair. No test stays stuck in `running` when its owner dies. Every pending test is eventually picked up by *some* worker as long as at least one invocation is alive.

**What this does not guarantee.** Fair scheduling across instances. A fast invocation with a narrow pattern may finish its work before a slow invocation with a broad pattern even notices. That's fine — the broad one just discovers more already-claimed rows and skips them.

## Quarantine

Some tests are known to hang or crash in ways that wedge the whole suite (e.g. tests stuck in the macOS UNE kernel state). Rather than dropping them on the floor, triage records that they were intentionally skipped so "what got skipped on this build" stays answerable from a single table.

A test is quarantined by setting `test.quarantined = true` and giving a `skip_reason`. Quarantine is a property of the test, not the build — it persists across builds until explicitly cleared.

When triage schedules work for a build, it still inserts a `test_run` row for every quarantined test, but with `status = skipped` and `completed_at` set immediately. The executable is never invoked for those tests. This means:

- `SELECT ... FROM test_run WHERE build_id = ? AND status = 'skipped'` lists everything that was bypassed, without joining against `test`.
- A quarantined test that later gets un-quarantined just starts producing real `pending → running → passed/failed` rows on the next build; no migration needed.
- `skip_reason` lives on `test` (the durable cause), not on the per-build `test_run`, so updating the reason doesn't require rewriting history.

Clearing quarantine is a manual action — triage never flips `quarantined` back to false on its own.

## Future: Failure & Bug Tracking

Not implemented yet. This section captures the requirements for the root-cause-grouping layer that sits on top of `test_run`, so the schema can be designed once and migrated to when we're ready. Nothing below exists in the current database.

The layer is two concepts:

- **Failure** — a mechanically-derived span of `test_run` rows for one test that all share the same error signature. Failures open and close automatically; humans never create them directly.
- **Bug** — a human-curated grouping of failures with a description, an append-only notes log, and a status. A single underlying root cause typically manifests as many failures (different tests, same signature), so bugs are where cross-test clustering lives. Failures are added to bugs manually.

### Failure

A failure has identity `(test_id, signature_hash)`. That is: a failure is **per-test** — if the same error message shows up across ten tests, that is ten failures (and probably one bug linking them all).

**Signature.** Derived from the `failure_message` column on `test_run` by normalizing out the parts that churn without changing the underlying bug:

- line numbers after "line " tokens,
- hex and memory-address-looking substrings,
- temp paths (`/tmp/...`, `/var/folders/...`),
- any other pattern that's observed to change across otherwise-identical runs.

The normalized string is then hashed (sha256 or similar) to produce `signature_hash`. The canonical (normalized) message is stored alongside the hash so humans can read it.

**Lifecycle.** A failing `test_run` either extends the currently-open failure for its `(test_id, signature_hash)` or, if the most recent run for that test was passing or failed with a *different* signature, auto-opens a new failure. A failure closes when a later `test_run` for the same test produces a **different** signature — the new run opens a new failure, and the old one's `last_build` is finalized.

A failure does **not** close when the test passes. Passes create gaps in the failure's timeline but leave it open; if the same signature comes back in a later build, it extends the original failure rather than opening a new one. This makes flakes behave as one long-lived failure with holes instead of a pile of short-lived ones.

Quarantine does **not** close failures either — quarantine is an execution concern (don't run the test), not a classification concern (how do we group the failures that existed).

**Sketch:**

```
failure {
  id:             uuid;
  test_id:        uuid;            // fk: test.id
  signature_hash: string;          // hash of normalized failure message
  canonical_msg:  string;          // human-readable normalized form

  status:         enum;            // open | closed
  first_build:    uuid;            // fk: build.id
  last_build:     uuid?;           // fk: build.id — set when status flips to closed

  @unique(test_id, signature_hash, first_build)
}
```

### Bug

Bugs group failures across tests (many-to-many) and carry the human judgement triage can't derive mechanically.

**Many-to-many.** A failure can belong to several bugs, which matters during diagnosis (two bugs could intersect on the same set of tests until you're sure which is the root cause) and for split/merge. The junction row carries its own lifecycle — links don't just vanish.

**Append-only log.** Every meaningful event on a bug (note added, failure attached, failure detached, status change, split, merge) writes an entry to `bug_log`. Nothing overwrites; the history is readable top-to-bottom. This is the collaboration surface — an agent reviewing a bug sees exactly who did what, and a human leaves notes in the same stream.

**Empty bugs stay.** When all of a bug's failures detach (e.g. signatures moved elsewhere), the bug is not deleted — its status changes to `empty` so it's filterable out of the default view, but the description and log remain available for re-use if the same root cause resurfaces.

**Sketch:**

```
bug {
  id:           uuid;
  title:        string;
  description:  string;            // markdown-ish; mutable
  status:       enum;              // active | empty | archived
  created_at:   timestamptz;
  updated_at:   timestamptz;
}

bug_failure {
  bug_id:       uuid;              // fk: bug.id
  failure_id:   uuid;              // fk: failure.id
  attached_at:  timestamptz;
  detached_at:  timestamptz?;      // non-null = failure has left this bug but the link is kept for audit

  @unique(bug_id, failure_id)
}

bug_log {
  id:         uuid;
  bug_id:     uuid;                // fk: bug.id
  author:     string;              // worker_id or human identifier
  created_at: timestamptz;
  kind:       enum;                // note | attached | detached | status_change | split | merge
  body:       string;              // free-text payload; triage writes structured lines for non-note kinds
}
```

### Key operations

- **Auto-open / auto-extend.** After writing a failing `test_run`, triage computes the signature and either extends the open failure for that `(test_id, signature_hash)` or opens a new one, closing any previously-open failure for the same test whose signature no longer matches.
- **Attach failure to bug.** Inserts a `bug_failure` row and writes an `attached` log entry.
- **Detach failure from bug.** Sets `bug_failure.detached_at` (does not delete) and writes a `detached` log entry. Happens manually, or automatically when a failure closes: the failure itself is closed, but its links to bugs are only marked detached.
- **Split a bug.** Default is *move*: pick a subset of active `bug_failure` rows, set their `detached_at` on the source bug, insert fresh rows attaching them to the new bug. A separate *cross-link* verb is available when you want the same failure attached to both bugs simultaneously — that leaves the original rows active and adds new ones on the new bug. Both cases write `split` entries on both bugs' logs.
- **Merge bugs.** Pick a target bug, move every active `bug_failure` row from the sources onto the target (inserting new rows, leaving detached rows on the sources for audit), flip the sources to `status = archived`, and write `merge` entries on every involved bug's log.
- **Cross-test clustering hint.** After a build completes, surface any `signature_hash` that newly appears on ≥2 tests with no matching bug. This is a suggestion for an operator to create a bug with all of them attached — never an automatic link.

### Query surface

The queries this layer is meant to answer, for both agents and humans:

- **What's new this build:** failures where `first_build = <current>`.
- **What regressed:** failures opening on tests that had a passing `test_run` in the previous build on the same branch.
- **What's still broken:** `failure.status = open`.
- **Which bugs grew:** bugs with a new `bug_failure` (`attached_at`) in the current build.
- **Which bugs shrank:** bugs whose attached failures closed during the current build.
- **Unclustered failures:** open failures with no active `bug_failure` row.

### Prerequisites

This layer assumes `test_run` carries a `failure_message` column — it does now. Stdout and stderr are available via the derived `.triage/logs/<test_run.id>/{stdout,stderr}` paths, so no additional columns are needed for the signature pipeline.

## Open issues

Tracked here so they don't get lost:

- **Log retention.** `.triage/logs/` grows monotonically — every `test_run` row adds a subdirectory with stdout and stderr files. There's no automated cleanup yet. A future `triage gc` subcommand should prune log directories whose corresponding `test_run` rows are older than a retention window (e.g. 30 days) or whose `build` has been explicitly pruned. Not urgent for the initial kestrel use case but will bite any long-running deployment.
