---
id: "MC-038"
title: "Mixed-platform scale and battery field validation"
depends_on: ["MC-034","MC-035","MC-036"]
kind: "gate"
branch: "ticket/MC-038-mixed-platform-scale-and-battery-field-validation"
---

# MC-038 — Mixed-platform scale and battery field validation

## Objective

Run a consented 30-50-device gathering with the predeclared MC-007 workload, density/topology, duration and device matrix.

## Dependencies

`MC-034`, `MC-035`, `MC-036` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `tests/field/**`, `docs/testing/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Run a consented 30-50-device gathering with the predeclared MC-007 workload, density/topology, duration and device matrix.
- Measure delivery within hop reach, latency distributions, churn, iOS degradation, battery and beacon/no-beacon differences.
- Record anonymized aggregate results and calibrate coverage claims against observed evidence.

## Exit criteria

- [ ] Predeclared field targets pass with device counts, duration, instrumentation and uncertainty documented.
- [ ] At least one sparse bridge and one dense mixed-platform case are included.
- [ ] Published product copy is limited to supported measured behavior; failed scenarios have resolved follow-up tickets.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If recruitment or hardware is insufficient, retain smaller trials as preliminary and keep the scale gate blocked.
- If targets fail, fix/retest or obtain explicit scope/claim changes; do not change thresholds after seeing results without disclosure.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Not implemented. Record commands, versions, reproducible inputs and results here. For manual/hardware checks include device/OS, duration and report paths. No test or review is claimed yet.

## Review and merge

- Branch: `ticket/MC-038-mixed-platform-scale-and-battery-field-validation`.
- Review/PR: pending.
- Squash commit title: `MC-038: Mixed-platform scale and battery field validation`.
- Completion becomes effective only when the reviewed squash commit lands on main.
