# Contributing to Kestrel

Thanks for your interest in contributing to Kestrel!

## Quick Start

```bash
# Clone and setup
git clone https://github.com/jkpdino/kestrel.git
cd kestrel
./scripts/setup-hooks.sh

# Run tests
cargo test

# Check formatting and lints
cargo fmt --check
cargo clippy
```

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
cargo test
```

Or use the pre-commit hooks (installed via `./scripts/setup-hooks.sh`).
