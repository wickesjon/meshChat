---
id: "MC-001"
title: "Project rules and executable ticketboard"
depends_on: []
kind: "docs"
branch: "ticket/MC-001-closeout"
---

# MC-001 — Project rules and executable ticketboard

## Objective

Preserve the original documents in the initial main commit; perform this approved planning change on ticket/MC-001-project-planning.

## Dependencies

None. See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `AGENTS.md`, `.gitignore`, `src/README.md`, `tests/README.md`, `docs/**`, `tests/ticketboard/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Preserve the original documents in the initial main commit; perform this approved planning change on ticket/MC-001-project-planning.
- Add project boundaries, squash-merge rules, corrected design requirements, the replacement plan, and this ticketboard.
- Implement a dependency validator and derive the index/diagram from ticket metadata.

## Exit criteria

- [x] Every approved review finding maps to a design correction or a blocking decision ticket.
- [x] All ticket IDs, links, dependencies, workflow locations, and the generated diagram validate; the graph is acyclic.
- [x] git diff --check passes; the user accepted the planning change by merging PR #1.
- [x] Relevant checks pass and implementation is merged to main via PR #1. The user used a regular merge; the exception is recorded below without rewriting history. Subsequent tickets retain the squash-merge requirement.

## Potential fallbacks

- If Git metadata is inaccessible, request a user-run branch setup and keep changes unapplied until the branch exists.
- If review exposes a new design decision, add a scoped blocking ticket; do not invent a resolved specification.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

- Baseline: original documents preserved in user-created commit `dfc1a3f` on main.
- Branch confirmed: `ticket/MC-001-project-planning`.
- `python -B tests/ticketboard/validate.py`: passed; 42 tickets, 99 dependencies, acyclic graph and current generated files.
- `python -B -m unittest discover -s tests/ticketboard -v`: 12 tests passed (cycle/missing/self/duplicate dependencies, branch/metadata/section checks, edge direction, bounded paths and generation).
- `git diff --check`: passed before publication and at closeout.
- Fallback used: sandbox Git metadata writes remained denied despite granted permissions; the user ran the baseline/branch setup in their own terminal before any planning edits.
- User approved the proposed scope on 2026-09-11 and merged [PR #1](https://github.com/wickesjon/meshChat/pull/1) at 2026-09-11T13:53:19Z.
- Published planning commit: `b225054c7d1d3e80b59bb3e36a50198694eb6402`.
- Main merge commit: `024b9aaac73cc7b903472065efd5a711ce46121b`. Its two parents confirm a regular merge, not a squash. Preserve this user-created history as a recorded one-time deviation; this is not a change to the ongoing merge rule.
- Local synchronization: all 54 working-file blob hashes matched merged origin/main after line-ending normalization; local main/index were aligned without changing working-file contents, and clean status was verified before the closeout branch.

## Review and merge

- Implementation branch: `ticket/MC-001-project-planning`.
- Closeout branch: `ticket/MC-001-closeout`.
- Review/PR: [PR #1, merged](https://github.com/wickesjon/meshChat/pull/1). The closeout branch only records the completed work and refreshes the board.
- Squash commit title: `MC-001: Project rules and executable ticketboard`.
- Implementation completion is evidenced by PR #1 on main, with the merge-method deviation above. The board status change becomes visible on main when the closeout PR lands; future ticket completion requires squash merge.
