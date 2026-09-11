---
id: "MC-001"
title: "Project rules and executable ticketboard"
depends_on: []
kind: "docs"
branch: "ticket/MC-001-project-planning"
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
- [ ] git diff --check passes; review the documentation diff before squash merging.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If Git metadata is inaccessible, request a user-run branch setup and keep changes unapplied until the branch exists.
- If review exposes a new design decision, add a scoped blocking ticket; do not invent a resolved specification.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

- Baseline: original documents preserved in user-created commit `dfc1a3f` on main.
- Branch confirmed: `ticket/MC-001-project-planning`.
- `python -B tests/ticketboard/validate.py`: passed; 42 tickets, 99 dependencies, acyclic graph and current generated files.
- `python -B -m unittest discover -s tests/ticketboard -v`: 12 tests passed (cycle/missing/self/duplicate dependencies, branch/metadata/section checks, edge direction, bounded paths and generation).
- `git diff --check`: passed; final reviewer approval and squash merge remain pending.
- Fallback used: sandbox Git metadata writes remained denied despite granted permissions; the user ran the baseline/branch setup in their own terminal before any planning edits.
- User approved the proposed scope on 2026-09-11; final diff review remains pending.

## Review and merge

- Branch: `ticket/MC-001-project-planning`.
- Review/PR: pending.
- Squash commit title: `MC-001: Project rules and executable ticketboard`.
- Completion becomes effective only when the reviewed squash commit lands on main.
