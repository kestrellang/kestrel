---
name: run-tests
description: Run the kestrel-test-suite2 file_tests harness single-threaded in the background, with output streamed to a temp file you can grep, tail, or analyze. Includes a hang-watchdog because compiled test binaries can wedge in macOS ReportCrash and become unkillable. TRIGGER for ANY invocation of the Kestrel test suite — full suite, targeted subsets, or single tests. Includes phrasings like "run the tests", "run the test suite", "run kestrel-test-suite2", "run the <name> tests", "run tests for <feature>", "re-run the failing test", "check if tests pass", "verify this works", "did that break anything", "does it still work", "make sure nothing regressed". ALSO TRIGGER before reaching for `cargo test -p kestrel-test-suite2` or executing `target/*/deps/file_tests-*` directly — this skill replaces both. SKIP only for non-kestrel-test-suite2 crates (e.g., `cargo test -p kestrel-hir`).
---

# Run kestrel-test-suite2

Run the lib2 file-based test harness directly (not via `cargo test`) so output is streamed line-by-line to a temp file you can monitor and inspect after the run completes.

## Why not `cargo test`?

`cargo test -p kestrel-test-suite2 --release` works, but it buffers output and doesn't expose the test binary you need for filtering / disassembly / re-running individual tests. Running the test binary directly gives:

- Live, line-buffered output to a temp file
- Ability to pass `--skip` and positive filters without recompiling
- A handle for spawning a hang-watchdog (see below)

## Steps

### 1. Pick a unique output path (REQUIRED for multi-agent safety)

Other agents may be running this skill concurrently. Using the shared `$OUT` would clobber their output and corrupt their watchdog. Pick a per-run path at the start and use it for the rest of the session:

```bash
OUT=$(mktemp /tmp/kts2.XXXXXX.out)
echo "OUT=$OUT"
```

Where XXXXXX is a randomly generated string based on the timestamp

Remember the path — every later step (`nohup`, watchdog `tail`, `grep` inspection) must reference `$OUT` (or the literal path), never `$OUT`.

Also check for active runs from other agents before launching a full suite:
```bash
pgrep -af "file_tests-|cargo test -p kestrel-test-suite2"
```
If another agent is running the full suite, prefer a targeted substring filter or wait.

### 2. Build the test binary (skip if no codegen changes)

```bash
cargo test -p kestrel-test-suite2 --release --no-run 2>&1 | tail -3
```

Note the path printed: `Executable tests/file_tests.rs (target/release/deps/file_tests-<hash>)`. That hash is stable for a given Cargo workspace state.

### 3. Run the binary in the background, output to $OUT

The harness uses `datatest-stable`, which requires the working directory to contain `testdata/`. So `cd` into `lib2/kestrel-test-suite/` first.

```bash
nohup /Users/dino/Documents/Projects/kestrel/target/release/deps/file_tests-<hash> \
  --test-threads=1 \
  > "$OUT" 2>&1 &
echo "PID=$!"
```

Replace `<hash>` with the value from step 1, or expand the path with a glob:
```bash
BIN=$(ls /Users/dino/Documents/Projects/kestrel/target/release/deps/file_tests-* | head -1)
```

The full suite is ~2800 tests. Single-threaded, expect ~25 minutes total in clean conditions.

### 4. Arm the hang watchdog (REQUIRED)

Compiled `.ks` test binaries that crash trigger macOS ReportCrash, which can wedge in kernel U-state. The parent runner blocks in `waitpid()` and never returns. `kill -9` cannot reap a process in uninterruptible sleep — it just sits there forever, taking the whole suite hostage.

Use Monitor with an until-loop that kills any child running >30s. This unblocks the parent so it can move on to the next test. The watchdog is scoped to `PARENT_PID` via `pgrep -P`, so it only touches *your* children — another agent's test run is unaffected.

```
Monitor command:
  while ps -p <PARENT_PID> > /dev/null 2>&1; do
    child=$(pgrep -P <PARENT_PID> 2>/dev/null | head -1)
    if [ -n "$child" ]; then
      etime=$(ps -p $child -o etimes= 2>/dev/null | tr -d ' ')
      if [ -n "$etime" ] && [ "$etime" -gt 30 ]; then
        lastline=$(tail -1 "$OUT")
        echo "HANG: child $child running ${etime}s — last test: $lastline"
        kill -9 $child 2>/dev/null
      fi
    fi
    sleep 5
  done
  echo "TEST_DONE"
  # Parse the summary line ("test result: FAILED. N passed; M failed; ...").
  # Do NOT grep `... FAILED$` — datatest-stable line-wraps long test names,
  # so the FAILED marker often lands on its own line and would be undercounted.
  summary=$(awk '/^test result:/ { print; exit }' "$OUT")
  echo "$summary"
```

Set `timeout_ms` to ~3600000 (1h) and `persistent: false`. Never use `pkill -9 -f file_tests-` — it will kill other agents' runs too.

### 5. Inspect output

While running or after:

```bash
# Final counts (authoritative — use after the run ends)
awk '/^test result:/ { print }' $OUT   # → "test result: FAILED. N passed; M failed; ..."

# Progress counts mid-run (passed count is reliable; failed is NOT — see note below)
grep -cE "\.\.\. ok$" $OUT
tail -3 $OUT                            # current test

# All failing test paths (robust — datatest-stable wraps long names, so the
# FAILED marker may land on its own line; parse the trailing "failures:" block instead)
sed -n '/^failures:$/,/^test result:/p' $OUT | grep "^    run_ks_test::" | sed 's/^    //'

# Full failure messages with diagnostics
sed -n '/^failures:$/,/^test result:/p' $OUT
```

**Do not** use `grep -cE "\.\.\. FAILED$"` for counts: datatest-stable line-wraps
long test names, so the `FAILED` marker often lands on its own line and gets
missed. The summary line (`test result: FAILED. N passed; M failed`) is the
only authoritative count — it appears at the very end of the output.

## Filtering — running a subset

```bash
# Substring filter (positional arg — matches test name substring)
$BIN --test-threads=1 closure_capture > $OUT 2>&1

# Skip a known-hanging test
$BIN --test-threads=1 --skip function_as_value > $OUT 2>&1

# Both
$BIN --test-threads=1 closures --skip function_as_value > $OUT 2>&1
```

## Single test with output

When iterating on one test, foreground it with `--nocapture` and a tight shell-level `timeout` to avoid the zombie problem:

```bash
cd /Users/dino/Documents/Projects/kestrel/lib2/kestrel-test-suite
timeout 30 $BIN --test-threads=1 --nocapture closure_capture_single 2>&1 | tail -20
```

## Preserving the compiled .ks binary for analysis

The runner deletes the compiled `.ks` binary after each test. To keep it for objdump / lldb / re-running:

1. Edit `lib2/kestrel-test-suite/src/runner.rs` to skip cleanup when `KESTREL_KEEP_TEST_BIN` is set:
   ```rust
   if std::env::var("KESTREL_KEEP_TEST_BIN").is_err() {
       let _ = std::fs::remove_dir_all(&temp_dir);
   } else {
       eprintln!("KEPT: {}", exe_path.display());
   }
   ```
2. Rebuild: `cargo test -p kestrel-test-suite2 --release --no-run`
3. Run with the env var: `KESTREL_KEEP_TEST_BIN=1 $BIN --test-threads=1 --nocapture <filter>`
4. Find the binary: `ls -lat /var/folders/*/T/kestrel2_test_*/test 2>/dev/null | head -1`
5. **Revert the runner.rs change** when done — it's a debugging aid, not for commit.

## Troubleshooting

- **"error while iterating directory ... testdata"**: you didn't `cd` into `lib2/kestrel-test-suite/` before running the binary. The harness uses datatest-stable which resolves `testdata/` relative to CWD.
- **Output file empty after several seconds**: the test binary is in linker / startup. Tests typically begin streaming within 5s; if not, check `ps` to confirm the binary is actually running.
- **Parent runner stuck after monitor reports HANG**: the kill didn't reap the child (kernel U-state). The monitor's job isn't to reap — it's to send SIGKILL so the kernel releases the parent's `waitpid`. If even that fails, the child is in a syscall blocked on something the kernel won't interrupt; the only recovery is to wait it out or reboot. This is rare but happens with codegen bugs that crash repeatedly in the same way.
