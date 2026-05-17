#!/usr/bin/env bash
# Create the GitHub Project (or reuse an existing one), configure its
# Status options + Area + Size custom fields, and write every node ID
# back as a repo variable.
#
# Idempotent — skips anything that already exists.
#
# Prerequisites: gh CLI authenticated with read:project + write:project,
# and admin on the repo. See scripts/setup/README.md.

set -euo pipefail

OWNER="${OWNER:-kestrellang}"
REPO="${REPO:-kestrel}"
PROJECT_TITLE="${PROJECT_TITLE:-Kestrel Language}"

STATUS_OPTIONS=(
  "Backlog"
  "Up Next"
  "In Progress"
  "In Review"
  "Nightly"
  "Beta"
  "Done"
)

AREA_OPTIONS=(
  parser name-res type-infer mir codegen stdlib lsp tooling docs
)

SIZE_OPTIONS=(S M L)

set_repo_var() {
  local name="$1"
  local value="$2"
  gh variable set "$name" --repo "$OWNER/$REPO" --body "$value" >/dev/null
  echo "  set var $name"
}

echo "==> Resolving project '$PROJECT_TITLE' under owner '$OWNER'"
PROJECT_NUMBER=$(gh project list --owner "$OWNER" --format json \
  --jq ".projects[] | select(.title == \"$PROJECT_TITLE\") | .number" \
  | head -1)

if [ -z "$PROJECT_NUMBER" ]; then
  echo "==> Creating project"
  PROJECT_NUMBER=$(gh project create --owner "$OWNER" --title "$PROJECT_TITLE" --format json --jq .number)
fi
echo "  project number: $PROJECT_NUMBER"

PROJECT_ID=$(gh project view "$PROJECT_NUMBER" --owner "$OWNER" --format json --jq .id)
echo "  project node id: $PROJECT_ID"
set_repo_var PROJECT_ID "$PROJECT_ID"

echo "==> Listing existing fields"
FIELDS_JSON=$(gh project field-list "$PROJECT_NUMBER" --owner "$OWNER" --format json)

field_id_for_name() {
  local name="$1"
  echo "$FIELDS_JSON" | python3 -c "
import json, sys
data = json.load(sys.stdin)
fields = data.get('fields', data) if isinstance(data, dict) else data
for f in fields:
    if f.get('name') == sys.argv[1]:
        print(f.get('id', ''))
        break
" "$name"
}

reload_fields() {
  FIELDS_JSON=$(gh project field-list "$PROJECT_NUMBER" --owner "$OWNER" --format json)
}

# ---- Status field ----
STATUS_FIELD_ID=$(field_id_for_name "Status")
if [ -z "$STATUS_FIELD_ID" ]; then
  echo "==> ERROR: built-in Status field not found on the project."
  echo "    Open the project once in the web UI to initialize defaults, then re-run."
  exit 1
fi
set_repo_var PROJECT_STATUS_FIELD_ID "$STATUS_FIELD_ID"

echo "==> Reconciling Status options"
EXISTING_STATUS=$(echo "$FIELDS_JSON" | python3 -c "
import json, sys
data = json.load(sys.stdin)
fields = data.get('fields', data) if isinstance(data, dict) else data
for f in fields:
    if f.get('name') == 'Status':
        for o in f.get('options', []):
            print(f\"{o['id']}\\t{o['name']}\")
        break
")

# Add any missing status options.
for opt in "${STATUS_OPTIONS[@]}"; do
  if ! echo "$EXISTING_STATUS" | awk -F'\t' -v n="$opt" '$2==n {found=1} END {exit !found}'; then
    echo "  adding status option: $opt"
    gh api graphql -f query='
      mutation($field: ID!, $name: String!) {
        updateProjectV2SingleSelectField(input: {
          fieldId: $field,
          options: [{ name: $name, color: GRAY, description: "" }]
        }) { projectV2Field { ... on ProjectV2SingleSelectField { id } } }
      }' \
      -F field="$STATUS_FIELD_ID" -F name="$opt" >/dev/null || \
      echo "  (note: option add may need manual setup via the web UI)"
  fi
done

# Re-fetch to pick up any newly added options, then export each ID.
reload_fields
for opt in "${STATUS_OPTIONS[@]}"; do
  ID=$(echo "$FIELDS_JSON" | python3 -c "
import json, sys
data = json.load(sys.stdin)
fields = data.get('fields', data) if isinstance(data, dict) else data
for f in fields:
    if f.get('name') == 'Status':
        for o in f.get('options', []):
            if o['name'] == sys.argv[1]:
                print(o['id'])
                break
        break
" "$opt")

  VAR_NAME="PROJECT_STATUS_$(echo "$opt" | tr '[:lower:] ' '[:upper:]_')_ID"
  if [ -n "$ID" ]; then
    set_repo_var "$VAR_NAME" "$ID"
  else
    echo "  WARNING: no id for status '$opt' — set $VAR_NAME manually"
  fi
done

# ---- Area field ----
AREA_FIELD_ID=$(field_id_for_name "Area")
if [ -z "$AREA_FIELD_ID" ]; then
  echo "==> Creating Area single-select field"
  AREA_FIELD_ID=$(gh api graphql -f query='
    mutation($project: ID!, $name: String!, $options: [ProjectV2SingleSelectFieldOptionInput!]!) {
      createProjectV2Field(input: {
        projectId: $project,
        dataType: SINGLE_SELECT,
        name: $name,
        singleSelectOptions: $options
      }) { projectV2Field { ... on ProjectV2SingleSelectField { id } } }
    }' \
    -F project="$PROJECT_ID" \
    -F name="Area" \
    -f options="$(python3 -c "
import json
opts = '$(IFS=,; echo "${AREA_OPTIONS[*]}")'.split(',')
print(json.dumps([{'name': o, 'color': 'GREEN', 'description': ''} for o in opts]))
")" \
    --jq '.data.createProjectV2Field.projectV2Field.id' 2>/dev/null || true)
  reload_fields
  AREA_FIELD_ID=$(field_id_for_name "Area")
fi
[ -n "$AREA_FIELD_ID" ] && set_repo_var PROJECT_AREA_FIELD_ID "$AREA_FIELD_ID"

# ---- Size field ----
SIZE_FIELD_ID=$(field_id_for_name "Size")
if [ -z "$SIZE_FIELD_ID" ]; then
  echo "==> Creating Size single-select field"
  gh api graphql -f query='
    mutation($project: ID!, $name: String!, $options: [ProjectV2SingleSelectFieldOptionInput!]!) {
      createProjectV2Field(input: {
        projectId: $project,
        dataType: SINGLE_SELECT,
        name: $name,
        singleSelectOptions: $options
      }) { projectV2Field { ... on ProjectV2SingleSelectField { id } } }
    }' \
    -F project="$PROJECT_ID" \
    -F name="Size" \
    -f options='[{"name":"S","color":"GREEN","description":"1–2 days"},{"name":"M","color":"YELLOW","description":"~1 week"},{"name":"L","color":"ORANGE","description":"Multi-cycle"}]' \
    >/dev/null 2>&1 || true
  reload_fields
  SIZE_FIELD_ID=$(field_id_for_name "Size")
fi
[ -n "$SIZE_FIELD_ID" ] && set_repo_var PROJECT_SIZE_FIELD_ID "$SIZE_FIELD_ID"

echo
echo "==> Done."
echo "    Project: https://github.com/orgs/$OWNER/projects/$PROJECT_NUMBER"
echo "    Configure views (Current Cycle / Triage / Roadmap / By Area / Release Candidate) via the web UI."
