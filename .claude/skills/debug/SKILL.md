---
name: debug
description: Structured protocol for debugging the lib2 Kestrel compiler — failing tests, diagnostic mismatches, inference cascades, MIR/codegen bugs, SIGSEGV or stack corruption, and intermittent flakes. Prevents speculative fix spirals by enforcing a reproduce → diagnose → one-hypothesis → fix → verify loop, with a hard stop after 3 failed attempts. Use when a triage run fails, when a `.ks` behaves unexpectedly, when the compiler crashes, or when you need to trace why a specific transformation produced the wrong output. Covers the `kestrel dump` CLI, `debug_trace!`, LLDB, and the project-wide "no `eprintln!`, no test-cajoling" rules.
---

# Debugging the lib2 Kestrel Compiler

Structured debugging protocol. Read this top-to-bottom before touching code —
the order matters. Skipping steps produces the speculative-fix spirals this
protocol exists to prevent.

## Step 0 — Check known patterns

Read `/Users/dino/.claude/projects/-Users-dino-Documents-Projects-kestrel/memory/DEBUG.md`
and see if the current symptom matches a known pattern. If it does, apply the
known fix directly and go to **Verify**. Don't re-derive a solved problem.

High-leverage patterns you'll often find there:
- COW `makeUnique()`/`grow()` ordering causing String/Array buffer overflows.
- Stack-slot initialization in Cranelift codegen (zero-init is defensive).
- Path-length-dependent crashes on macOS — a tell for uninitialized memory.

## Step 1 — Reproduce deterministically

Get a reproduction before anything else. **Do not skip this step.** No
reproduction means no diagnosis, and any "fix" you write is speculation.

### For a failing testdata file

Work directly with the failing `.ks` under
`lib2/kestrel-test-suite/testdata/...`:

```
# via the /triage skill, with a tight pattern
# never invoke `cargo test -p kestrel-test-suite2` or `file_tests-*` directly
```

If you need smaller scope, copy the failing file into `temp/repro.ks` and
shrink it — delete declarations, simplify types, drop imports — until the
reproduction is the minimum that still fails. Simpler = faster to diagnose.

### For a user bug report / freestanding example

Write the minimum `.ks` file that reproduces the symptom:

```
mkdir -p temp
# write temp/repro.ks with the smallest code that triggers the issue
```

### For crashes / SIGSEGV

Run under libgmalloc to catch use-after-free and heap corruption at the point
of the bug, not later:

```
DYLD_INSERT_LIBRARIES=/usr/lib/libgmalloc.dylib /path/to/built/repro
```

Crash reports are at `~/Library/Logs/DiagnosticReports/` — the stack trace
there is often more useful than anything you'll get from a panic message.

### For intermittent / flaky bugs

Pin it first. Run the failing test in a loop (via `/triage` with a tight
pattern, or directly against the repro binary) enough times to get a
deterministic hit rate. If it only fails in one directory, suspect
path-length-dependent stack layout — that usually means uninitialized memory
or a buffer overflow, not a "flaky test."

### Budget

If you can't reproduce in ~5 minutes, **stop and ask the user** which diagnostic
tool to try next. Don't keep guessing.

## Step 2 — Diagnose with the right tool

Once you have a reproduction, narrow down **where** the problem is before
asking **what**. The lib2 pipeline is:

```
Source → Tokens → CST → AST (ECS) → Name Res → HIR → Type Infer → MIR → Codegen
```

### `kestrel dump` — show compiler-internal state at a stage

The lib2 binary exposes dump subcommands (see `src/main.rs` for the
authoritative list):

```
kestrel dump tokens repro.ks        # lexer output
kestrel dump cst repro.ks           # concrete syntax tree
kestrel dump mir repro.ks           # MIR after HIR lowering + all passes
kestrel dump cranelift repro.ks     # Cranelift IR (CLIF), pre-optimization
kestrel dump diagnostics repro.ks   # all accumulated diagnostics
```

Dumps go to stdout; diagnostics to stderr. Capture with
`kestrel dump mir repro.ks > out.txt`.

> **Not available today**: `ast`, `hir`, `types`, `asm` dumps. The `TODO` in
> `src/main.rs` tracks them. If you reach for one and it's missing, the
> alternative is `debug_trace!` at the relevant crate boundary.

Run the repro through adjacent stages and compare — the stage where output
first goes wrong localizes the bug.

### `debug_trace!` — targeted tracing inside a stage

When the problem is inside a stage (e.g., "inference picks the wrong
overload"), add `debug_trace!` calls at the decision points in the relevant
crate, then run with:

```
VERBOSE_DEBUG_OUTPUT=1 <repro command>
```

`debug_trace!` output is gated on that env var. Project rule from `CLAUDE.md`:
**don't use `eprintln!` / `println!` or any other flags for debugging**. Add
`debug_trace!` to the compiler source instead. Clean them up later (Step 6),
or leave them if they'd help the next reader — but only on purpose.

### LLDB — for crashes and deep stepping

```
cargo build
lldb target/debug/kestrel -- build repro.ks
# or `-- dump mir repro.ks` if the crash is during a specific dump stage
```

Useful starting breakpoints in lib2:

| Issue | Breakpoint prefix |
|-------|------------------|
| AST building | `kestrel_ast_builder::builders` |
| HIR lowering | `kestrel_hir_lower` |
| Inference / solver | `kestrel_type_infer::solver` / `::resolve` |
| Analyzer diagnostic | `kestrel_analyze::decl` / `::body` |
| MIR lowering | `kestrel_mir_lower` |
| Cranelift codegen | `kestrel_codegen2_cranelift` |

(Run `image lookup -rn <pattern>` inside lldb if the exact symbol is unclear.)

### Codebase exploration

If the repro is clear but you don't know which code owns the behavior, use
the `kestrel-pipeline` skill to map the offending construct to its file:line
across stages — that's what it's for. For deeper "how does this path work?"
questions, spawn an Explore subagent with a targeted question.

## Step 3 — One hypothesis

State **one** hypothesis in writing before touching code. Include:

- The exact file/function/line you believe is wrong.
- The evidence that supports the hypothesis (dump output, trace log, lldb
  backtrace).
- What would disprove it.

If you can't name a specific line, you're not ready to fix — go back to Step 2.

## Step 4 — Fix

Apply the minimum change that addresses the confirmed root cause. Don't
bundle refactors or cosmetic fixes — those go in a separate commit.

**Project rule (firm, from `CLAUDE.md`):**
- Never modify a test "to make it pass" unless it uses invalid syntax, and
  even then only with explicit user go-ahead. Tests document intended
  behavior — if the test is right and your fix makes it fail, your fix is
  wrong.
- Never add `#[ignore]`. If a test is broken, fix it or surface it.
- Never revert or throw away in-progress changes to "start clean." Stash
  first; ask before discarding.

## Step 5 — Verify

- Run the **targeted** repro / testdata via `/triage` — must pass.
- Run a **broader** triage pattern scoped to the feature area — must pass.
- Full triage run before commit. Do not claim a run is green without reading
  the pass/fail output from triage — a started-and-ignored run is not a
  verification.

Multi-agent note: other agents may be running the test suite at the same
time. Keep triage patterns scoped; don't assume exclusive access.

## Escalation rule — hard stop at 3

**After 3 failed fix attempts against the same hypothesis, STOP.** Do not
try a 4th. Instead, write up for the user:

1. What each attempt tried and how it failed.
2. What the failures ruled out.
3. The evidence for and against the current hypothesis.
4. What additional diagnostic you'd like to run, or what decision you need
   from them.

This is in `CLAUDE.md` and in user feedback memory. Honor it — the bugs that
drove this rule were all cases where a 4th-through-8th speculative fix made
things worse.

## Step 6 — Record outcomes

When the debugging session ends, update
`/Users/dino/.claude/projects/-Users-dino-Documents-Projects-kestrel/memory/DEBUG.md`:

- **Bug fixed:** add an entry to *Successful Fixes* with the root cause, the
  fix, and the tool/technique that diagnosed it. One paragraph each.
- **Bug not fixed (escalated):** add an entry to *Failed Approaches* with
  what was tried, why each attempt failed, and the lesson learned.

Don't write to DEBUG.md mid-investigation. The record is for what you
actually learned, not what you suspect.

Also: clean up `temp/repro.ks` (keep only if it's a good regression seed),
remove any `debug_trace!` calls you added purely for this session (keep the
ones that are load-bearing for future readers — mark them as such in the
surrounding comment if it's not obvious), and check whether a new
`.ks` belongs under `testdata/` as a permanent regression test. If it does,
delegate to the `write-tests` skill for format/placement.

## Anti-patterns (don't do these)

- Patching symptoms without a stated root cause.
- Using `eprintln!` / `println!` as a debug channel. Use `debug_trace!`.
- Running `cargo test -p kestrel-test-suite2` or `file_tests-*` directly.
  Always `/triage`.
- Changing a test to match buggy behavior. See Step 4.
- Ignoring the 3-attempt escalation rule. Thrashing costs more than asking.
- Writing to DEBUG.md with speculation. Record outcomes, not in-progress guesses.

## When to use a different skill

- **`change`** — the root cause is understood and the fix is a scoped
  behavioral change; skip the debug protocol and go straight to the change
  workflow.
- **`kestrel-pipeline`** — just need "which file handles X?" lookups.
- **`write-tests`** — promoting a repro into a permanent regression test.
- **`triage`** — operational questions about the test harness itself.
