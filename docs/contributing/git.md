# Git and Release Workflow

Branches, the release train, the GitHub Project board, issues, and PRs.

## Philosophy

Kestrel ships on a **3-week release train**. Every three weeks we cut a release, regardless of what's in flight. Big features that miss the train wait for the next one — the cadence keeps small work flowing instead of getting blocked behind multi-cycle plumbing.

Three things drive the workflow:

- **Branches** follow git-flow — `nightly` is the integration trunk, `main` only ever holds released versions.
- **The Project board** owns task state (Backlog → Done). It replaces the old per-task checklists in `ROADMAP.md`.
- **`ROADMAP.md`** keeps the narrative — phase descriptions, version themes, design rationale — but no longer tracks individual checkboxes.

## Branches

| Branch | Purpose | Branches from | Merges to |
|--------|---------|---------------|-----------|
| `main` | Released versions only. Advanced by merging `beta` at release. | — | — |
| `nightly` | Active development trunk. Always in a working state. | `main` | `beta` (via PR at cut) |
| `feature/NNNN-slug` | New feature for an issue. | `nightly` | `nightly` |
| `fix/NNNN-slug` | Bug fix for an issue. | `nightly` | `nightly` |
| `refactor/NNNN-slug` | Refactor for an issue. | `nightly` | `nightly` |
| `beta` | Single, permanent stabilization branch. `nightly` merges in at the start of each train. | `nightly` (via PR) | `main` (via PR at release) |
| `hotfix/NNNN-slug` | Patch for the most recent release. | tag `vX.Y.Z` | `main`, then forward to `nightly` and any active `beta` |

`NNNN` is the zero-padded issue number; the slug is the lowercased title. `issue-branch.yml` creates these automatically when an issue opens.

`beta`'s history is continuous — no force-pushes, ever. Branch protection on `beta` is as strict as on `main`.

## The release train

Each version is a **3-week cycle**. There is only ever one stabilization branch (`beta`), advanced by merging `nightly` into it at the start of each train.

```
Week 1–2:  nightly is open. Issue PRs merge in.
End of W2: open the nightly -> beta PR and merge it. nightly stays open for 0.(X+1).
Week 3:    bug-fix only on beta. Each fix cherry-picks back to nightly.
End of W3: merge the beta -> main PR (this releases the cycle), then tag v0.X.0
           to publish binaries. beta sits at the release until the next merge.
```

`beta` is never deleted — it's the same branch, advancing version by version. The week-2 merge is a fast-forward when the previous release tag is already on `nightly`'s history. It becomes a real merge when stabilization or hotfix commits have advanced one branch without the other.

Cherry-picks of beta fixes back to `nightly` mean the start-of-cycle merge usually resolves silently — git recognizes the identical patches. If it doesn't (someone amended a commit, or a fix on `nightly` touched the same code differently), you resolve a real conflict at that merge. That's a useful integration check, not a bug.

### What counts as a bug fix during week 3

A PR can land on `beta` only if it:

- Closes an issue with the `bug` label, **or**
- Fixes a regression introduced since the previous release tag.

No new features, no refactors, no public API changes. Anything else waits for the next train.

### Hotfixes

A hotfix patches the **most recent released version only**. Anything older is fixed forward in the next release.

`hotfix/NNNN-slug` branches from the tag (`vX.Y.Z`), gets PR'd to `main`, then `vX.Y.(Z+1)` is tagged from the merge commit. After the tag, merge `main` forward into `nightly`.

If `beta` already has an in-flight stabilization, merge `main` into `beta` too. `main` must be an ancestor of any branch that will later fast-forward it at release time.

If `main` has moved to a newer version since the bug shipped, don't backport. Fix forward in the next release.

## The Project board

One Project for the whole language. Multiple views, single source of state.

### Status (kanban columns)

| Status | Meaning |
|--------|---------|
| **Backlog** | Captured, not scheduled to a milestone. |
| **Up Next** | Has a milestone, work hasn't started. |
| **In Progress** | Branch exists, work happening. |
| **In Review** | PR open against `nightly` or `beta`. |
| **Nightly** | Merged to `nightly`, not yet promoted to `beta`. |
| **Beta** | Included in `beta`, not yet released to `main`. |
| **Done** | Released — the `beta` -> `main` PR merged. Drops off active views. |

The Nightly/Beta split matters because of the train: "merged" ≠ "promoted" ≠ "shipped." Without those states, work disappears from view before it reaches the release branch.

### Fields

| Field | Source | Values |
|-------|--------|--------|
| **Milestone** | Built-in GitHub Milestones | `0.16`, `0.17`, … |
| **Area** | Project field, mirrored by automation to a label | `parser`, `name-res`, `type-infer`, `mir`, `codegen`, `stdlib`, `lsp`, `tooling`, `docs` |
| **Size** | Project field | `S` (1–2 days), `M` (~1 week), `L` (multi-cycle) |

Type and priority live as labels, not fields — see [Labels](#labels).

### Views

- **Current Cycle** — kanban by Status, filtered to the current Milestone. Daily driver.
- **Triage** — issues with no Milestone and no Area. Inbox.
- **Roadmap** — table grouped by Milestone. Replaces `ROADMAP.md`'s checkbox view.
- **By Area** — grouped by Area. For context-switching.
- **Release Candidate** — filter on Status = Beta. Pre-release checklist.

### Automation

Project automation handles the routine transitions:

- Issue opened → Status = Backlog, label `triage`.
- Branch created (via `issue-branch.yml`) → Status = In Progress.
- PR marked Ready for Review → Status = In Review.
- PR merged to `nightly` → that PR's issues → Nightly.
- `nightly` → `beta` PR merged → every card in Nightly → Beta.
- `beta` → `main` PR merged → every card in Beta → Done (released).
- Hotfix PR merged to `main` → that PR's issues → Done.

Week-3 stabilization fixes skip Nightly at first: the PR lands on `beta`, moves to Beta, then the fix cherry-picks back to `nightly`.

Manual transitions: Backlog → Up Next (during triage), reverting on close-without-merge.

### GitHub Projects integration

GitHub Projects has useful built-in workflows, but branch-aware status changes need GitHub Actions. All board writes go through one workflow (`project-status.yml`) and one script (`scripts/setup/set_status.py`), which exposes two selectors: `--by-closing-issues` (the issues a PR closes) and `--by-status` (a whole status column). The integration contract is:

- `issue-branch.yml` creates the issue branch and draft PR, then sets Status = In Progress.
- `pull_request.ready_for_review` → the PR's closing issues → In Review.
- `pull_request.closed` with `merged == true`, dispatched by the head→base pair:
  - base `nightly` → the PR's closing issues → Nightly
  - base `beta`, head `nightly` → every Nightly card → Beta (cycle promotion)
  - base `beta`, other head → the PR's closing issues → Beta (week-3 stabilization fix)
  - base `main`, head `beta` → every Beta card → Done (release)
  - base `main`, other head → the PR's closing issues → Done (hotfix)
- Promotion and forward-merge PRs close no issues, so the closing-issue selector is a natural no-op for them; the head branch is what flags a bulk promotion. Bulk moves aren't milestone-filtered — because each promotion sweeps the whole column, Nightly/Beta only ever hold the current cycle's cards.

Use the Projects GraphQL API for custom field updates. The workflow needs a token that can read issues/PRs and update the Project, plus these repository variables or secrets:

| Name | Purpose |
|------|---------|
| `PROJECT_ID` | Node ID of the GitHub Project. |
| `PROJECT_STATUS_FIELD_ID` | Node ID of the Project's Status field. |
| `PROJECT_STATUS_IN_PROGRESS_ID` | Option ID for In Progress. |
| `PROJECT_STATUS_IN_REVIEW_ID` | Option ID for In Review. |
| `PROJECT_STATUS_NIGHTLY_ID` | Option ID for Nightly. |
| `PROJECT_STATUS_BETA_ID` | Option ID for Beta. |
| `PROJECT_STATUS_DONE_ID` | Option ID for Done. |

Do not rely on branch creation alone as the source of truth. Branches prove work started; the head and base branches of merged PRs prove where the change actually landed.

## Issue lifecycle

1. **Open issue.** `issue-branch.yml` creates `feature/NNNN-slug` (or `fix/`, `refactor/` based on label) off `nightly` and opens a draft PR. Issue lands in **Triage**.
2. **Triage** (see [Triage cadence](#triage-cadence)). Assign Area + Size + Milestone. Status moves to Up Next.
3. **Work.** Push to the branch. Status moves to In Progress.
4. **Review.** Mark PR Ready. Status moves to In Review.
5. **Merge.** Normal work lands on `nightly` and moves to Nightly. Week-3 stabilization fixes land on `beta`, move to Beta, and cherry-pick back to `nightly`.
6. **Promote to `beta`.** Merging the `nightly` → `beta` PR moves every Nightly card to Beta.
7. **Release.** Merging the `beta` → `main` PR moves every Beta card to Done; then tag `vX.Y.0` to publish binaries.

## Triage cadence

Triage happens **in batch the day after a release tag**, paired with the natural "what's next?" moment of cutting the new milestone. Incoming issues sit in the Triage view with the `triage` label until then.

For each issue: assign Area, Size, Milestone (current, next, later, or none → stays in Backlog), remove the `triage` label, move to Up Next.

Hotfix-worthy bugs are the exception — they get triaged on arrival.

## Epics and big features

Multi-cycle work (existentials, class runtime, LLVM backend) doesn't fit a 3-week milestone. Use **epic + children**:

- **Epic issue** describes the whole feature. Lives across milestones, stays In Progress until every child closes.
- **Child issues** decompose by pipeline stage. Each child is a single-milestone unit. Parser support lands in 0.17, type inference in 0.18, codegen in 0.19, etc.

This forces the work into shippable slices, which is the whole point of the train.

The epic body uses a task list referencing the children:

```markdown
- [ ] #142 — parser support for `any P` syntax
- [ ] #143 — name resolution for existentials
- [ ] #144 — HIR + inference
- [ ] #145 — MIR boxing + vtable layout
- [ ] #146 — codegen
- [ ] #147 — stdlib `any` adoption
```

GitHub auto-checks the boxes as children close.

## Labels

Kept minimal. Type + Area + a small set of workflow signals.

**Type** (one per issue, set by template):

| Label | Use |
|-------|-----|
| `bug` | Something is wrong. |
| `feature` | New functionality. |
| `refactor` | Internal restructuring, no behavior change. |
| `chore` | Tooling, docs, dependencies. |

**Area** (mirrors the Project field):

`parser`, `name-res`, `type-infer`, `mir`, `codegen`, `stdlib`, `lsp`, `tooling`, `docs`.

**Workflow** (signals, not Project Status):

| Label | Applied | Removed |
|-------|---------|---------|
| `triage` | On issue open | When triaged |
| `breaking` | Manual | — |
| `wontfix` | Manual on close | — |

Don't add a label per Project Status — the board already shows that.

## Pull requests

### Targeting

| Source | Target |
|--------|--------|
| `feature/*`, `fix/*`, `refactor/*` | `nightly` |
| Stabilization fix during week 3 | `beta` |
| `hotfix/*` | `main` |
| `beta` | `main` (at release time, by maintainer) |

### Requirements (all branches)

- Linked to an issue in the current or next Milestone (`Closes #NNN` in the body).
- `/triage` (full test suite) green.
- `cargo fmt` clean, no `cargo clippy` warnings.
- Manual review and merge — no auto-merge.

### Additional requirements for `beta` and `hotfix/*`

- `bug` label on the linked issue, **or** explicit regression note in the PR body.
- After a `beta` fix merges, cherry-pick it to `nightly`.
- After a hotfix merges to `main`, merge `main` forward to `nightly` and any active `beta`.

### PR title format

```
feature: short description
fix: short description
refactor: short description
chore: short description
```

The prefix matches the branch type and the issue's Type label.

## Quick reference

```bash
# Pick up an issue branch (created by issue-branch.yml)
git fetch origin
git checkout feature/0142-existential-parser

# Advance beta from nightly (end of week 2): open the PR, then merge it on
# GitHub with "Create a merge commit" to preserve the cycle boundary.
gh workflow run promote-to-beta.yml -f milestone=0.16
#   (equivalently: gh pr create --base beta --head nightly --title "Promote 0.16 to beta")

# Release (end of week 3): open and merge the beta -> main PR on GitHub.
# The merge moves Beta cards to Done; the manual tag publishes binaries.
gh pr create --base main --head beta --title "Release 0.16"
#   ... merge it (fast-forward) on GitHub, then:
git checkout main && git pull
git tag v0.16.0
git push origin v0.16.0

# Cherry-pick a beta fix back to nightly
git checkout nightly && git pull
git cherry-pick <commit-from-beta>
git push origin nightly

# Hotfix to the most recent release
git checkout -b hotfix/0201-segfault-on-empty-array v0.16.0
# ... fix, PR to main, tag v0.16.1
git checkout nightly && git pull
git merge --no-ff origin/main -m "merge hotfix v0.16.1 into nightly"
git push origin nightly

# If beta is already stabilizing, carry the hotfix there too
git checkout beta && git pull
git merge --no-ff origin/main -m "merge hotfix v0.16.1 into beta"
git push origin beta
```
