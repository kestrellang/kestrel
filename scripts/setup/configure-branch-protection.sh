#!/usr/bin/env bash
# Apply branch protection rules to main, nightly, beta.
#
# main:    no direct pushes, fast-forward only, PR required.
# nightly: PR required, `ci` status check, linear history.
# beta:    PR required, `ci` status check, merge commits allowed
#          (the cycle nightly→beta merge is non-fast-forward by design).
#
# Idempotent: PUTting protection overwrites the existing config, so
# re-running is safe.

set -euo pipefail

OWNER="${OWNER:-kestrellang}"
REPO="${REPO:-kestrel}"

protect_main() {
  echo "==> Protecting main"
  gh api -X PUT "repos/$OWNER/$REPO/branches/main/protection" \
    --input - <<'JSON' >/dev/null
{
  "required_status_checks": {
    "strict": true,
    "contexts": ["ci"]
  },
  "enforce_admins": false,
  "required_pull_request_reviews": {
    "required_approving_review_count": 0,
    "dismiss_stale_reviews": true
  },
  "restrictions": null,
  "required_linear_history": true,
  "allow_force_pushes": false,
  "allow_deletions": false,
  "required_conversation_resolution": true,
  "lock_branch": false,
  "allow_fork_syncing": false
}
JSON
}

protect_nightly() {
  echo "==> Protecting nightly"
  gh api -X PUT "repos/$OWNER/$REPO/branches/nightly/protection" \
    --input - <<'JSON' >/dev/null
{
  "required_status_checks": {
    "strict": true,
    "contexts": ["ci"]
  },
  "enforce_admins": false,
  "required_pull_request_reviews": {
    "required_approving_review_count": 0,
    "dismiss_stale_reviews": true
  },
  "restrictions": null,
  "required_linear_history": true,
  "allow_force_pushes": false,
  "allow_deletions": false,
  "required_conversation_resolution": false,
  "lock_branch": false,
  "allow_fork_syncing": false
}
JSON
}

protect_beta() {
  echo "==> Protecting beta"
  # Note: required_linear_history MUST be false here — the week-2
  # nightly→beta merge is a real merge commit by design.
  gh api -X PUT "repos/$OWNER/$REPO/branches/beta/protection" \
    --input - <<'JSON' >/dev/null
{
  "required_status_checks": {
    "strict": true,
    "contexts": ["ci"]
  },
  "enforce_admins": false,
  "required_pull_request_reviews": {
    "required_approving_review_count": 0,
    "dismiss_stale_reviews": true
  },
  "restrictions": null,
  "required_linear_history": false,
  "allow_force_pushes": false,
  "allow_deletions": false,
  "required_conversation_resolution": false,
  "lock_branch": false,
  "allow_fork_syncing": false
}
JSON
}

protect_main
protect_nightly

# beta only exists after `git push origin nightly:refs/heads/beta`; skip
# protection if it doesn't exist yet.
if gh api "repos/$OWNER/$REPO/branches/beta" >/dev/null 2>&1; then
  protect_beta
else
  echo "==> Skipping beta (branch does not exist yet — push it first)"
fi

echo "==> Done."
