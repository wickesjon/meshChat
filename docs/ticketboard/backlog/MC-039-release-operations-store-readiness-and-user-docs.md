---
id: "MC-039"
title: "Release operations, store readiness and user docs"
depends_on: ["MC-034","MC-035","MC-037","MC-038"]
kind: "release"
branch: "ticket/MC-039-release-operations-store-readiness-and-user-docs"
---

# MC-039 — Release operations, store readiness and user docs

## Objective

Prepare signed-build procedures, store privacy/permission disclosures, minimal content-free diagnostics policy and rollback/hotfix steps.

## Dependencies

`MC-034`, `MC-035`, `MC-037`, `MC-038` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `docs/releases/**`, `docs/organizer/**`, `docs/support/**`, `src/android/**`, `src/ios/**`, `src/share-site/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Prepare signed-build procedures, store privacy/permission disclosures, minimal content-free diagnostics policy and rollback/hotfix steps.
- Write organizer provisioning/rotation/expiry and beacon deployment guides, plus offline-sharing, battery and iOS support guidance.
- Validate production link/store configurations, licenses and release checklists; keep credentials out of the repository.

## Exit criteria

- [ ] Release builds are reproducible with provenance; signing secrets are handled outside tracked content through approved tooling.
- [ ] Organizer/support procedures are exercised and match shipping behavior.
- [ ] Submission packages, privacy declarations and production association checks are ready for authorized publication.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If store/domain/signing access is missing, record explicit external blockers and deliver reviewable configuration templates.
- Do not submit, publish or change production services without explicit authorization.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Not implemented. Record commands, versions, reproducible inputs and results here. For manual/hardware checks include device/OS, duration and report paths. No test or review is claimed yet.

## Review and merge

- Branch: `ticket/MC-039-release-operations-store-readiness-and-user-docs`.
- Review/PR: pending.
- Squash commit title: `MC-039: Release operations, store readiness and user docs`.
- Completion becomes effective only when the reviewed squash commit lands on main.
