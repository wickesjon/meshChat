---
id: "MC-040"
title: "v1 release acceptance gate"
depends_on: ["MC-039"]
kind: "gate"
branch: "ticket/MC-040-v1-release-acceptance-gate"
---

# MC-040 — v1 release acceptance gate

## Objective

Audit completion and merge evidence for every required v1 ticket, approved scope decisions and both wire freezes.

## Dependencies

`MC-039` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `docs/releases/**`, `docs/ticketboard/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Audit completion and merge evidence for every required v1 ticket, approved scope decisions and both wire freezes.
- Verify security review, field evidence, store acceptance and operational readiness without confusing submission readiness with approval.
- Record the release decision, immutable build identifiers and remaining explicitly deferred work.

## Exit criteria

- [ ] All required tickets are complete on main with satisfied dependencies and evidence.
- [ ] Both platform store outcomes and authorized distribution evidence are recorded; external blockers are resolved.
- [ ] No release-critical risk remains hidden behind a fallback; the v1 acceptance record is reviewable.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If any prerequisite lacks evidence, keep release blocked and identify the exact owner/action.
- A reduced release needs explicit scope approval and updated acceptance records; never mark full v1 done for an Android-only beta.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Not implemented. Record commands, versions, reproducible inputs and results here. For manual/hardware checks include device/OS, duration and report paths. No test or review is claimed yet.

## Review and merge

- Branch: `ticket/MC-040-v1-release-acceptance-gate`.
- Review/PR: pending.
- Squash commit title: `MC-040: v1 release acceptance gate`.
- Completion becomes effective only when the reviewed squash commit lands on main.
