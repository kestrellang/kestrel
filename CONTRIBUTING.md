# Contributing to Kestrel

Thanks for your interest in contributing to Kestrel!

## Quick Start

```bash
# Clone and setup
git clone https://github.com/kestrellang/kestrel.git
cd kestrel

# Run the .ks test suite via the triage CLI — do NOT use `cargo test` for it
triage                 # full suite
triage <pattern>       # targeted subset

# Unit tests for an individual crate are still plain cargo
cargo test -p kestrel-type-infer

# Check formatting and lints
cargo fmt --check
cargo clippy
```

The `.ks` test suite (`kestrel-test-suite`) must be run through the `triage` CLI, not `cargo test` — triage records results in `.triage/triage.db`, supports background runs (`triage --async`), and is safe alongside other agents working in the same tree. See `.claude/skills/triage/SKILL.md` and [`docs/contributing/index.md`](docs/contributing/index.md) for details.

## Documentation

Detailed contributing guides are in [`docs/contributing/`](docs/contributing/):

- [**Architecture**](docs/contributing/architecture.md) - How the compiler works
- [**Quick Reference**](docs/contributing/quick-reference.md) - File locations and imports
- [**Patterns**](docs/contributing/patterns.md) - Code style and conventions
- [**Workflows**](docs/contributing/workflows.md) - Step-by-step guides
- [**Git**](docs/contributing/git.md) - Branching, PRs, and issues

## Workflow Summary

1. **Create an issue** describing your bug fix or feature
2. A branch and draft PR are created automatically
3. Check out the branch and make your changes
4. Push commits - CI runs fmt, clippy, and tests
5. Mark PR ready for review when done
6. PR merges to `nightly` after approval

## Before Committing

```bash
cargo fmt
cargo clippy
triage          # full .ks suite — required before commits, not after every edit
```
