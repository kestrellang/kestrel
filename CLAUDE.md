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
- Full suite takes ~5 minutes. Use `--release` for full runs: `cargo test -p kestrel-test-suite --release`
- During iteration, target specific tests: `cargo test -p kestrel-test-suite --release -- test_name`
- Only run the full suite before commits, not after every edit

### Fluid Memory

Whenever you get stuck, and figure something out, add a memory for it so you won't get stuck in that way again1

# Code Quality

Prioritize early returns, avoid code smells such as deep nesting and long functions
Try to reuse existing code rather than rewriting new functions
Code should be terse but include comments explaining what it does and why