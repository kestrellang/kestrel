# Workflow setup scripts

These scripts wire up the GitHub-side state described in
[`docs/contributing/git.md`](../../docs/contributing/git.md): the GitHub
Project board, labels, milestones, branch protection, and the bulk
migration of ROADMAP items into issues.

Run them in the order below. Each script is idempotent — re-running won't
duplicate state.

## Prerequisites

1. **`gh` CLI** authenticated as a user with admin on `kestrellang/kestrel`.
2. **Fine-grained PAT** with these scopes:
   - `repo` (read/write)
   - `read:org`
   - `read:project`, `write:project`
   - `admin:repo_hook` (for branch protection)
3. PAT exported as `GH_TOKEN` for the local terminal, **and** added as
   the repo secret `PROJECTS_TOKEN` (used by Actions workflows that
   touch the Project).
4. `python3` ≥ 3.10 in `PATH` (used by `set_status.py`).

## Order

```bash
# 1. Create the beta branch from nightly's tip (one-time).
git push origin nightly:refs/heads/beta

# 2. Sync labels from .github/labels.yml. Either push a commit that
#    touches labels.yml, or run the workflow manually:
gh workflow run sync-labels.yml

# 3. Create the GitHub Project and capture its IDs as repo variables.
bash scripts/setup/configure-project.sh

# 4. Create milestones (0.16 → 0.23 with 3-week due dates, plus the
#    five open-ended long-term milestones).
bash scripts/setup/create-milestones.sh

# 5. Bulk-migrate ROADMAP unchecked items into issues (~80).
bash scripts/setup/migrate-roadmap.sh

# 6. Apply branch protection to main, nightly, beta.
bash scripts/setup/configure-branch-protection.sh
```

## What each script does

| Script | What |
|--------|------|
| `configure-project.sh` | Creates Project "Kestrel Language", adds custom fields (Area, Size), configures Status options (Backlog, Up Next, In Progress, In Review, Nightly, Beta, Done), and writes every node ID back as repo variables (`PROJECT_ID`, `PROJECT_STATUS_FIELD_ID`, `PROJECT_STATUS_*_ID`, `PROJECT_AREA_FIELD_ID`, `PROJECT_SIZE_FIELD_ID`). Idempotent — skips creation if the project already exists. |
| `create-milestones.sh` | Creates 8 numeric milestones (`0.16` → `0.23`) with 3-week due dates starting `2026-05-31`, plus 5 open-ended milestones (`Preview 3`, `Preview 4`, `RC`, `2.0`, `3.0`). Skips any that already exist. |
| `migrate-roadmap.sh` | Creates one issue per unchecked ROADMAP item (~80 total) with the right type label, `triage` label, and milestone. Idempotent by issue title — re-running won't create duplicates. |
| `configure-branch-protection.sh` | Applies protection rules: `main` (no direct pushes, fast-forward only, PR required), `nightly` (PR required, `ci` status check, linear history), `beta` (PR required, `ci` status check, merge commits allowed for the cycle merge). |
| `set_status.py` | Library script invoked by `project-status.yml` to move Project cards — either the issues a PR closes (`--by-closing-issues`) or a whole status column on promotion (`--by-status`). Not run directly. |

## Re-running

All scripts are safe to re-run. They check existing state before writing.
If you need a clean slate, delete the Project (web UI) and the
`PROJECT_*` repo variables (`gh variable list` / `gh variable delete`)
before re-running `configure-project.sh`.
