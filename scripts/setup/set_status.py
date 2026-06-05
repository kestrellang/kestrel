#!/usr/bin/env python3
"""Set the Project Status field of issues. Single write primitive for the
board, with two selection modes used by project-status.yml:

  --by-closing-issues   move the issues a PR closes (PR_NUMBER) -> TARGET
  --by-status           move every item in SOURCE_OPTION_ID -> TARGET_OPTION_ID

Selection differs per merge: feature/hotfix merges move what the PR closes;
nightly->beta and beta->main promotions sweep a whole status column (a
promotion PR closes no issues, so it has nothing to select by-closing).

Reads from environment variables:
  PROJECT_ID            Project node ID                       (both modes)
  STATUS_FIELD_ID       Project Status field node ID          (both modes)
  PR_NUMBER             PR whose closing issues to move        (closing mode)
  REPO_OWNER, REPO_NAME                                        (closing mode)
  TARGET                Status option to set                   (closing mode)
  SOURCE_OPTION_ID      Status option to match                 (status mode)
  TARGET_OPTION_ID      Status option to set                   (status mode)

Authenticates via the `gh` CLI (GH_TOKEN env var, set by the workflow to
PROJECTS_TOKEN).
"""

from __future__ import annotations

import json
import os
import subprocess
import sys


def gh_graphql(query: str, **variables: object) -> dict:
    args = ["gh", "api", "graphql", "-f", f"query={query}"]
    for key, value in variables.items():
        args.extend(["-F", f"{key}={value}"])
    result = subprocess.run(args, capture_output=True, text=True, check=True)
    return json.loads(result.stdout)


ADD_ITEM = (
    "mutation($project: ID!, $content: ID!) { "
    "addProjectV2ItemById(input: {projectId: $project, contentId: $content}) "
    "{ item { id } } }"
)

SET_STATUS = (
    "mutation($project: ID!, $item: ID!, $field: ID!, $option: String!) { "
    "updateProjectV2ItemFieldValue(input: { "
    "  projectId: $project, itemId: $item, fieldId: $field, "
    "  value: { singleSelectOptionId: $option } "
    "}) { projectV2Item { id } } }"
)


def set_status(project: str, field: str, item: str, option: str) -> None:
    gh_graphql(SET_STATUS, project=project, item=item, field=field, option=option)


def by_closing_issues() -> None:
    """Move every issue this PR closes to TARGET. Branch-merge PRs (promotions,
    forward-merges) close nothing, so this is a natural no-op for them."""
    project = os.environ["PROJECT_ID"]
    field = os.environ["STATUS_FIELD_ID"]
    target = os.environ["TARGET"]

    query = (
        "query($owner: String!, $repo: String!, $number: Int!) { "
        "repository(owner: $owner, name: $repo) { pullRequest(number: $number) { "
        "closingIssuesReferences(first: 20) { nodes { id } } } } }"
    )
    data = gh_graphql(
        query,
        owner=os.environ["REPO_OWNER"],
        repo=os.environ["REPO_NAME"],
        number=os.environ["PR_NUMBER"],
    )
    nodes = data["data"]["repository"]["pullRequest"]["closingIssuesReferences"]["nodes"]

    for node in nodes:
        # addProjectV2ItemById is idempotent — returns the existing item if the
        # issue is already on the board.
        item = gh_graphql(ADD_ITEM, project=project, content=node["id"])
        item_id = item["data"]["addProjectV2ItemById"]["item"]["id"]
        set_status(project, field, item_id, target)

    print(f"moved {len(nodes)} closing issue(s) -> {target}")


def by_status() -> None:
    """Move every item currently in SOURCE_OPTION_ID to TARGET_OPTION_ID.

    No milestone filter: promotions sweep the whole column, so Nightly/Beta
    only ever hold the current cycle's cards.
    """
    project = os.environ["PROJECT_ID"]
    field = os.environ["STATUS_FIELD_ID"]
    source = os.environ["SOURCE_OPTION_ID"]
    target = os.environ["TARGET_OPTION_ID"]

    page_query = (
        "query($project: ID!, $cursor: String) { "
        "node(id: $project) { ... on ProjectV2 { "
        "items(first: 100, after: $cursor) { "
        "pageInfo { hasNextPage endCursor } "
        "nodes { id fieldValues(first: 20) { nodes { "
        "  ... on ProjectV2ItemFieldSingleSelectValue { "
        "    optionId field { ... on ProjectV2SingleSelectField { id } } } } } "
        "} } } } }"
    )

    cursor: str | None = None
    moved = 0
    while True:
        if cursor is None:
            data = gh_graphql(page_query, project=project)
        else:
            data = gh_graphql(page_query, project=project, cursor=cursor)
        page = data["data"]["node"]["items"]

        for item in page["nodes"]:
            current = None
            for fv in item["fieldValues"]["nodes"]:
                if fv and (fv.get("field") or {}).get("id") == field:
                    current = fv.get("optionId")
                    break
            if current == source:
                set_status(project, field, item["id"], target)
                moved += 1

        if not page["pageInfo"]["hasNextPage"]:
            break
        cursor = page["pageInfo"]["endCursor"]

    print(f"moved {moved} item(s) {source} -> {target}")


def main() -> None:
    mode = sys.argv[1] if len(sys.argv) > 1 else ""
    if mode == "--by-closing-issues":
        by_closing_issues()
    elif mode == "--by-status":
        by_status()
    else:
        sys.exit("usage: set_status.py --by-closing-issues | --by-status")


if __name__ == "__main__":
    main()
