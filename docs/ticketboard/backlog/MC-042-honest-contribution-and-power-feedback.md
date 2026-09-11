---
id: "MC-042"
title: "Honest contribution and power feedback"
depends_on: ["MC-028","MC-024"]
kind: "product"
branch: "ticket/MC-042-honest-contribution-and-power-feedback"
---

# MC-042 — Honest contribution and power feedback

## Objective

Implement local counters and power feedback with measured-versus-estimated labels and defined reset/retention behavior.

## Dependencies

`MC-028`, `MC-024` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/core/**`, `src/android/ui/**`, `src/ios/UI/**`, `tests/integration/stats/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Implement local counters and power feedback with measured-versus-estimated labels and defined reset/retention behavior.
- Map contribution titles/share cards from design §16 only to locally observable evidence; replace unverifiable unique-person or bridge claims with accurate counters.
- Keep stats private by default and share/export user-initiated; exclude raw identities, message text and channel words from aggregate diagnostics.

## Exit criteria

- [ ] Counter tests distinguish packets, logical messages and egress sends and cannot imply successful delivery.
- [ ] Battery estimates are labeled and unsupported OS attribution is never presented as measurement.
- [ ] Share cards contain only user-approved local aggregates; no background telemetry or leaderboard is introduced.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If bridge/person-count inference cannot be supported, use literal relay/message counters and record the reduced wording.
- If reliable battery attribution is unavailable, show a labeled estimate or omit the number rather than fabricating measurements.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Not implemented. Record commands, versions, reproducible inputs and results here. For manual/hardware checks include device/OS, duration and report paths. No test or review is claimed yet.

## Review and merge

- Branch: `ticket/MC-042-honest-contribution-and-power-feedback`.
- Review/PR: pending.
- Squash commit title: `MC-042: Honest contribution and power feedback`.
- Completion becomes effective only when the reviewed squash commit lands on main.
