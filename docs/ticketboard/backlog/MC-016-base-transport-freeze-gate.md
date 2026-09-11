---
id: "MC-016"
title: "Base transport freeze gate"
depends_on: ["MC-011","MC-015"]
kind: "gate"
branch: "ticket/MC-016-base-transport-freeze-gate"
---

# MC-016 — Base transport freeze gate

## Objective

Run the base codec, framing, ingress, relay and SYNC suite against the recorded MC-004 transport assumptions and online evidence.

## Dependencies

`MC-011`, `MC-015` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `tests/vectors/base/**`, `tests/simulator/**`, `docs/testing/**`, `docs/mesh-chat-design.md`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Run the base codec, framing, ingress, relay and SYNC suite against the recorded MC-004 transport assumptions and online evidence.
- Review all base type/flag layouts and freeze vectors plus a compatibility/versioning policy.
- Record test commands, seeds, versions, pass results and remaining crypto-only decisions.

## Exit criteria

- [ ] All MC-007 base gates and fuzz regressions pass with reproducible evidence.
- [ ] Approved MC-004 online feasibility and permission evidence exists; runtime capacity assumptions/refusal cases are explicit and no unresolved base-wire ambiguity remains. This does not certify physical radio support.
- [ ] Base freeze is documented without falsely declaring unfinished crypto envelopes frozen.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If a measured transport assumption fails, reopen its decision and dependent tickets before freezing.
- A deferred crypto layout is allowed only where explicitly isolated from the stable base contract.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Acceptance reference updated under the user-approved MC-004 physical-gate replacement on 2026-09-11; see [the decision](../../decisions/MC-004-online-feasibility.md). MC-025/MC-027 retain physical radio acceptance.

Not implemented. Record commands, versions, reproducible inputs and results here. For manual/hardware checks include device/OS, duration and report paths. No test or review is claimed yet.

## Review and merge

- Branch: `ticket/MC-016-base-transport-freeze-gate`.
- Review/PR: pending.
- Squash commit title: `MC-016: Base transport freeze gate`.
- Completion becomes effective only when the reviewed squash commit lands on main.
