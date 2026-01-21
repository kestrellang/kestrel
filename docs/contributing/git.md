# Git Workflow

This guide covers the branching strategy, PR workflow, and issue management for the Kestrel project.

## Branches

| Branch | Purpose | Branches From | Merges To |
|--------|---------|---------------|-----------|
| `main` | Releases only | - | - |
| `nightly` | Active development | `main` | `beta/*` |
| `feature/<issue>-<name>` | New features | `nightly` | `nightly` |
| `fix/<issue>-<name>` | Bug fixes | `nightly` | `nightly` |
| `beta/<version>` | Release preparation | `nightly` | `main` |
| `hotfix/<name>` | Urgent production fixes | `main` | `main` + `nightly` |

### Branch Details

**main**
- Contains only released versions
- All releases are tagged with `vX.Y.Z`
- Pre-1.0 releases use `v0.X.Y-preview`

**nightly**
- Primary development branch
- All features and fixes merge here first
- Should always be in a working state

**feature/\<issue\>-\<name\>**
- Branch off `nightly` for new features
- Example: `feature/42-generic-types`
- PR back into `nightly`

**fix/\<issue\>-\<name\>**
- Branch off `nightly` for bug fixes
- Example: `fix/57-parser-crash`
- PR back into `nightly`

**beta/\<version\>**
- Created when preparing a release
- Example: `beta/v0.14`
- Used for release stabilization
- Merges into `main` when ready

**hotfix/\<name\>**
- For urgent fixes to released versions
- Branch from `main`
- Bumps patch version (e.g., `v0.13.0` → `v0.13.1`)
- Must merge into both `main` AND `nightly`

## Workflow

### Feature/Fix Development

```
1. Create issue describing the work
2. Create branch from nightly:
   git checkout nightly
   git pull
   git checkout -b feature/123-my-feature

3. Make changes, commit
4. Push and create PR to nightly
5. Get review, CI must pass
6. Merge PR
```

### Release Process

```
1. Create beta branch from nightly:
   git checkout nightly
   git checkout -b beta/v0.14

2. Stabilize, fix any issues
3. When ready, merge to main:
   git checkout main
   git merge beta/v0.14

4. Tag the release:
   git tag v0.14.0
   git push origin v0.14.0
```

### Hotfix Process

```
1. Create hotfix branch from main:
   git checkout main
   git checkout -b hotfix/critical-bug

2. Fix the issue
3. Merge to main, tag with bumped patch version:
   git checkout main
   git merge hotfix/critical-bug
   git tag v0.13.1
   git push origin v0.13.1

4. Also merge to nightly:
   git checkout nightly
   git merge hotfix/critical-bug
```

## Pull Requests

### Requirements

- All PRs require at least one review
- CI must pass
- Branch name must reference issue number
- Commits should follow the commit message format (see [Workflows](workflows.md))

### PR Title Format

```
feature: description of feature
fix: description of bug fix
refactor: description of refactoring
docs: description of documentation change
test: description of test addition
```

## Issues

All work requires an issue first. This ensures:
- Discussion before implementation
- Clear requirements
- Trackable progress

### Creating Issues

Use the issue templates:
- **Bug Report**: For reporting bugs
- **Feature Request**: For proposing new features

### Automatic Branch Creation

When an issue is opened, a GitHub Action automatically:
1. Creates a branch off `nightly` (e.g., `feature/123-my-feature` or `fix/123-bug-name`)
2. Adds `.issues/123.md` with the issue description
3. Comments on the issue with checkout instructions

The `.issues/` directory serves as a permanent record of work done.

### Labels

**Type:**
| Label | Description |
|-------|-------------|
| `bug` | Something isn't working |
| `enhancement` | New feature or improvement |
| `breaking` | Breaking change |

**Status:**
| Label | Description |
|-------|-------------|
| `needs-triage` | Needs review |
| `accepted` | Approved for work |
| `wontfix` | Won't be addressed |

**Area:**
| Label | Description |
|-------|-------------|
| `type-system` | Type system changes |
| `syntactic-sugar` | Syntax conveniences |
| `literals` | Literal values |
| `operators` | Operator support |
| `protocols` | Protocol system |
| `structs` | Struct definitions |
| `extensions` | Extension methods |
| `enums` | Enum support |
| `codegen` | Code generation |
| `stdlib` | Standard library |
| `docs` | Documentation |

## Quick Reference

```bash
# Start a feature
git checkout nightly && git pull
git checkout -b feature/123-my-feature

# Start a fix
git checkout nightly && git pull
git checkout -b fix/456-bug-name

# Push tags
git push origin --tags

# Push a specific tag
git push origin v0.14.0
```
