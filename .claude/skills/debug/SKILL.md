# Debug Skill

Structured debugging protocol for Kestrel. Prevents speculative fix spirals.

## Step 0: Check known patterns

Read `/Users/dino/.claude/projects/-Users-dino-Documents-Projects-kestrel/memory/DEBUG.md` and check if the symptom matches a known pattern. If it does, apply the known fix directly and verify.

## Step 1: Reproduce

Get a deterministic reproduction before doing anything else. Do NOT skip this step.

- **For crashes/SIGSEGV**: Run under libgmalloc first:
  ```
  DYLD_INSERT_LIBRARIES=/usr/lib/libgmalloc.dylib cargo test -p kestrel-test-suite --release -- test_name
  ```
- **For intermittent bugs**: Run the failing test in a loop to confirm flakiness:
  ```
  for i in $(seq 1 20); do cargo test -p kestrel-test-suite --release -- test_name 2>&1 | tail -1; done
  ```
- **For path-dependent bugs**: Try running from a directory with a different path length

If you cannot reproduce after 5 minutes, say so and ask the user which diagnostic tool to try next. Do NOT guess at fixes.

## Step 2: Diagnose

With a reproduction in hand, identify the exact cause:

- Read crash reports: `~/Library/Logs/DiagnosticReports/`
- Get a stack trace from the crash
- Narrow down to the specific file, function, and line
- Form ONE hypothesis and state it clearly with supporting evidence

## Step 3: Fix

Apply a targeted fix for the confirmed root cause. Run the reproduction case to verify.

## Step 4: Verify

Run targeted tests first, then the broader suite:
```
cargo test -p kestrel-test-suite --release -- test_name
cargo test -p kestrel-test-suite --release
```

## Escalation rule

**After 3 failed fix attempts, STOP.** Do not try a 4th fix. Instead:
1. List what has been tried and why each failed
2. List what has been ruled out
3. State what evidence supports or contradicts the current hypothesis
4. Ask the user for guidance on how to proceed

## Recording outcomes

When the session ends, update DEBUG.md:

- **On failure** (bug not fixed): Add an entry to "Failed Approaches" with what was tried, why it failed, and the lesson learned
- **On success** (bug fixed): Add an entry to "Successful Fixes" with the root cause, fix, and how it was diagnosed
- Do NOT write to DEBUG.md during in-progress investigation
