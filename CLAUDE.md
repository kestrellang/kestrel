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

## Testing
- Full suite takes ~5 minutes. Use `--release` for full runs: `cargo test -p kestrel-test-suite --release`
- During iteration, target specific tests: `cargo test -p kestrel-test-suite --release -- test_name`
- Only run the full suite before commits, not after every edit