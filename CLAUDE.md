Information on the project structure, workflows, quick references, and patterns can be found in /docs/contributing/

/
docs/
contributing/
architecture.md
index.md
patterns.md
quick-reference.md
workflows.md

- never change a test unless it uses invalid syntax. dont change a test to make it pass. Dont ignore one either
  NEVER throw away changes. always stash changes before checking out another branch.
- When hitting a roadblock, stop and ask for guidance before reverting changes
- After 3 failed attempts at the same class of fix, STOP. List what was tried, what was ruled out, and ask for guidance before continuing.

## Debugging
- Verbose debug tracing is available via `VERBOSE_DEBUG_OUTPUT=1`. This enables `debug_trace!` output in the binder and semantic tree crates (member resolution, method calls, where clause checks, type substitutions).

## Testing
- **ALWAYS use the `/run-tests` skill** for anything that runs `kestrel-test-suite2` — full suite, targeted subsets, or single tests. Do not invoke `cargo test -p kestrel-test-suite2` or the `file_tests-*` binary directly; the skill handles backgrounding, the hang-watchdog (mandatory on macOS), and a per-run grep-able output file (via `mktemp /tmp/kts2.XXXXXX.out`, so multiple agents don't clobber each other).
- After a run completes, **read the output** — check pass/fail counts and grep the `$OUT` file for failures before reporting results. A started run with ignored output is not a test run.
- Only run the full suite before commits, not after every edit. For iteration, pass a positional substring filter to the skill.
- Do not run `kestrel-test-suite` (the lib1 suite) — lib2 is the active target.

### Fluid Memory

Whenever you get stuck, and figure something out, add a memory for it so you won't get stuck in that way again1

# Code Quality

Prioritize early returns, avoid code smells such as deep nesting and long functions
Try to reuse existing code rather than rewriting new functions
Code should be terse but include comments explaining what it does and why
All solutions should be clean, comprehensive, and stick to hECS patterns
Always use a single source of truth, don't spread it over multiple locations
Make sure to consider incremental compilation.

# Lib 2 rewrite

We are rewriting it in lib2, focus on that

# Directory-scoped guidance (`AGENTS.md`)

Before editing a file, look for an `AGENTS.md` in its directory or any parent
directory up to the repo root. These files document patterns and principles
that apply inside the containing subtree — design conventions, diagnostic-ID
allocations, analyzer structure, HIR invariants, etc. Nearer files are more
specific and take precedence; the repo-root `CLAUDE.md` is the fallback.

## Proactively capture patterns

While working, watch for reusable patterns, invariants, or design decisions —
things like "we always do X here because Y," "never do Z in this subtree,"
"when adding a new W, update these three places." Examples: a naming rule
picked up from a code review, a new diagnostic-ID allocation, a non-obvious
ordering constraint between passes, a decision to prefer one approach over
another after weighing tradeoffs.

When you notice one, **ask the user** whether to record it in the nearest
applicable `AGENTS.md` (or create a new one scoped to the smallest subtree
where the pattern applies). A one-line ask is enough: "Noticed we're doing X
here — want me to add it to `path/AGENTS.md`?" Don't silently add entries,
and don't hoard patterns for a session-ending summary — capture them as they
come up, while the context is fresh.