#!/usr/bin/env python3
"""Move every Project item with a given source Status in the given milestone
to a target Status. Used by promote-to-beta.yml (Nightly→Beta) and
release.yml (Beta→Done).

Reads from environment variables:
  PROJECT_ID            Project node ID
  STATUS_FIELD_ID       Project Status field node ID
  SOURCE_OPTION_ID      Status option to match (defaults to NIGHTLY_OPTION_ID
                        if set, otherwise BETA_OPTION_ID — for backward compat
                        with the two callers).
  TARGET_OPTION_ID      Status option to set (defaults to BETA_OPTION_ID, then
                        DONE_OPTION_ID).
  MILESTONE             Milestone title (e.g. "0.16")
  REPO_OWNER, REPO_NAME

Authenticates via the `gh` CLI (uses GH_TOKEN env var, set by the workflow
to PROJECTS_TOKEN).
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


def resolve_options() -> tuple[str, str]:
    """Pick (source_option_id, target_option_id) from env, supporting either
    explicit SOURCE/TARGET vars or the older NIGHTLY/BETA/DONE convention.
    """
    src = os.environ.get("SOURCE_OPTION_ID")
    tgt = os.environ.get("TARGET_OPTION_ID")
    if src and tgt:
        return src, tgt

    nightly = os.environ.get("NIGHTLY_OPTION_ID", "")
    beta = os.environ.get("BETA_OPTION_ID", "")
    done = os.environ.get("DONE_OPTION_ID", "")

    if nightly and beta and not done:
        return nightly, beta
    if beta and done:
        return beta, done

    sys.exit("error: must set SOURCE_OPTION_ID + TARGET_OPTION_ID, "
             "or NIGHTLY_OPTION_ID + BETA_OPTION_ID, "
             "or BETA_OPTION_ID + DONE_OPTION_ID")


def main() -> None:
    project_id = os.environ["PROJECT_ID"]
    status_field_id = os.environ["STATUS_FIELD_ID"]
    milestone = os.environ["MILESTONE"]
    owner = os.environ["REPO_OWNER"]
    repo = os.environ["REPO_NAME"]
    source_id, target_id = resolve_options()

    # 1. Find the milestone number from the title.
    milestone_query = (
        "query($owner: String!, $repo: String!) { "
        "repository(owner: $owner, name: $repo) { "
        "milestones(first: 50, states: [OPEN, CLOSED]) { nodes { number title } } "
        "} }"
    )
    data = gh_graphql(milestone_query, owner=owner, repo=repo)
    nodes = data["data"]["repository"]["milestones"]["nodes"]
    matching = [n for n in nodes if n["title"] == milestone]
    if not matching:
        sys.exit(f"error: no milestone titled {milestone!r} in {owner}/{repo}")
    milestone_number = matching[0]["number"]

    # 2. Page through the project's items, filter by milestone + source status,
    #    update each to target status.
    page_query = (
        "query($project: ID!, $cursor: String) { "
        "node(id: $project) { ... on ProjectV2 { "
        "items(first: 100, after: $cursor) { "
        "pageInfo { hasNextPage endCursor } "
        "nodes { id "
        "  fieldValues(first: 20) { nodes { "
        "    ... on ProjectV2ItemFieldSingleSelectValue { "
        "      optionId field { ... on ProjectV2SingleSelectField { id } } } } } "
        "  content { ... on Issue { number milestone { number } } "
        "             ... on PullRequest { number milestone { number } } } "
        "} } } } }"
    )

    update_mutation = (
        "mutation($project: ID!, $item: ID!, $field: ID!, $option: String!) { "
        "updateProjectV2ItemFieldValue(input: { "
        "  projectId: $project, itemId: $item, fieldId: $field, "
        "  value: { singleSelectOptionId: $option } "
        "}) { projectV2Item { id } } }"
    )

    cursor: str | None = None
    moved = 0
    while True:
        if cursor is None:
            data = gh_graphql(page_query, project=project_id)
        else:
            data = gh_graphql(page_query, project=project_id, cursor=cursor)
        page = data["data"]["node"]["items"]

        for item in page["nodes"]:
            content = item.get("content") or {}
            ms = (content.get("milestone") or {}).get("number")
            if ms != milestone_number:
                continue

            current_status = None
            for fv in item["fieldValues"]["nodes"]:
                if not fv:
                    continue
                field = fv.get("field") or {}
                if field.get("id") == status_field_id:
                    current_status = fv.get("optionId")
                    break

            if current_status != source_id:
                continue

            gh_graphql(
                update_mutation,
                project=project_id,
                item=item["id"],
                field=status_field_id,
                option=target_id,
            )
            moved += 1

        if not page["pageInfo"]["hasNextPage"]:
            break
        cursor = page["pageInfo"]["endCursor"]

    print(f"moved {moved} item(s) in milestone {milestone}")


if __name__ == "__main__":
    main()
