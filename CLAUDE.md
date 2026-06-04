Information on the project structure, workflows, quick references, and patterns can be found in /docs/contributing/

## Agent Rules

- Never change a test in order to cajole it to pass, unless I tell you explicitly to, or it uses invalid kestrel syntax. Don't add #[ignore] to tests
- Tests should document the state of the compiler, they don't need to all pass. If a behavior is not working yet, you should add a test to ensure it gets fixed.
- If you hit a roadblock, stop and ask for guidance. Don't revert your changes, throw away changes, or anything. After 3 failed attempts at the same class of fix, STOP. List what was tried, what was ruled out, and ask for guidance before continuing.
- There will be multiple agents working at the same time in this codebase


## Debugging
- Verbose debug tracing is available via `VERBOSE_DEBUG_OUTPUT=1`. This enables `debug_trace!` output in the binder and semantic tree crates (member resolution, method calls, where clause checks, type substitutions). Don't use eprintln!, println!, or any other flags for debugging. When debugging something add `debug_trace!` to the compiler source code.

## Testing
- **Only run `kestrel-test-suite` through the `/triage` skill** — full suite, targeted subsets, or single tests. Do not invoke `cargo test -p kestrel-test-suite` or the `file_tests-*` binary directly; the triage skill records results in `.triage/triage.db`, supports background runs, and is safe alongside other agents.
- After a run completes, **read the triage results** — check pass/fail counts and failure messages before reporting results. A started run with ignored output is not a test run.
- Only run the full suite before commits, not after every edit. For iteration, pass a targeted pattern to triage.

### Fluid Memory

Whenever you get stuck, and figure something out, add a memory for it so you won't get stuck in that way again1

# Code Quality

Prioritize early returns, avoid code smells such as deep nesting and long functions
Try to reuse existing code rather than rewriting new functions
Code should be terse but include comments explaining what it does and why
All solutions should be clean, comprehensive, and stick to hECS patterns
Always use a single source of truth, don't spread it over multiple locations
Make sure to consider incremental compilation.
Avoid fragile solutions, use holistic solutions that won't break down the line
When you fix a mistake, think about how you can avoid the same mistake being made later

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

## Debugging

Any time you have to trace something through compiler internals, use the kestrel-debug crate