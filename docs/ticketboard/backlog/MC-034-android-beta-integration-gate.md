---
id: "MC-034"
title: "Android beta integration gate"
depends_on: ["MC-025","MC-029","MC-030","MC-031","MC-032","MC-033","MC-042","MC-022"]
kind: "gate"
branch: "ticket/MC-034-android-beta-integration-gate"
---

# MC-034 — Android beta integration gate

## Objective

Exercise every Android v1 feature against production core/radio/storage integrations.

## Dependencies

`MC-025`, `MC-029`, `MC-030`, `MC-031`, `MC-032`, `MC-033`, `MC-042`, `MC-022` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `tests/integration/android/**`, `tests/bench/android/**`, `docs/releases/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Exercise every Android v1 feature against production core/radio/storage integrations.
- Run a mid-app field check and verify offline onboarding, privacy/trust states, reset/delete and degraded lifecycle behavior.
- Record beta build provenance, known limitations and reproducible end-to-end evidence.

## Exit criteria

- [ ] All Android v1 feature acceptance tests pass on the supported device matrix.
- [ ] Crypto/full-wire gate has passed; no simulated-only implementation is represented as production complete.
- [ ] A reviewable beta candidate and limitation list are ready; distributing it requires explicit authorization.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If a feature fails, keep the beta gate blocked or obtain an explicit approved beta scope reduction.
- Never bypass encryption, trust or storage controls to make the beta appear complete.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Not implemented. Record commands, versions, reproducible inputs and results here. For manual/hardware checks include device/OS, duration and report paths. No test or review is claimed yet.

## Review and merge

- Branch: `ticket/MC-034-android-beta-integration-gate`.
- Review/PR: pending.
- Squash commit title: `MC-034: Android beta integration gate`.
- Completion becomes effective only when the reviewed squash commit lands on main.
