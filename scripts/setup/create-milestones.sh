#!/usr/bin/env bash
# Create the version milestones for the release train.
#
# Numeric milestones (0.16 → 0.23) get 3-week due dates starting
# 2026-05-31 (v0.16.0). Long-term milestones (Preview 3, Preview 4, RC,
# 2.0, 3.0) are open-ended.
#
# Idempotent: skips milestones that already exist.

set -euo pipefail

OWNER="${OWNER:-kestrellang}"
REPO="${REPO:-kestrel}"

# Title:date pairs. Date is the v0.X.0 release day (end of week 3).
NUMERIC=(
  "0.16:2026-05-31"
  "0.17:2026-06-21"
  "0.18:2026-07-12"
  "0.19:2026-08-02"
  "0.20:2026-08-23"
  "0.21:2026-09-13"
  "0.22:2026-10-04"
  "0.23:2026-10-25"
)

LONG_TERM=(
  "Preview 3"
  "Preview 4"
  "RC"
  "2.0"
  "3.0"
)

existing_titles() {
  gh api "repos/$OWNER/$REPO/milestones?state=all&per_page=100" --jq '.[].title'
}

EXISTING="$(existing_titles)"

create_milestone() {
  local title="$1"
  local due="${2:-}"
  if echo "$EXISTING" | grep -qxF "$title"; then
    echo "  skip (exists): $title"
    return
  fi

  if [ -n "$due" ]; then
    gh api "repos/$OWNER/$REPO/milestones" \
      -f title="$title" \
      -f due_on="${due}T23:59:59Z" \
      --silent
    echo "  created: $title (due $due)"
  else
    gh api "repos/$OWNER/$REPO/milestones" \
      -f title="$title" \
      --silent
    echo "  created: $title"
  fi
}

echo "==> Numeric milestones"
for entry in "${NUMERIC[@]}"; do
  title="${entry%%:*}"
  due="${entry##*:}"
  create_milestone "$title" "$due"
done

echo "==> Long-term milestones"
for title in "${LONG_TERM[@]}"; do
  create_milestone "$title"
done

echo "==> Done."
