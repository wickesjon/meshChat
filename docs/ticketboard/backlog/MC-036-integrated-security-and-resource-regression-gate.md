---
id: "MC-036"
title: "Integrated security and resource regression gate"
depends_on: ["MC-022","MC-027","MC-033"]
kind: "security"
branch: "ticket/MC-036-integrated-security-and-resource-regression-gate"
---

# MC-036 — Integrated security and resource regression gate

## Objective

Aggregate feature-owned fuzz and negative suites; run invalid crypto, fragment/session floods, slot exhaustion, sender rotation and cache pollution against bench devices.

## Dependencies

`MC-022`, `MC-027`, `MC-033` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `tests/adversarial/**`, `tests/fuzz/**`, `tests/integration/security/**`, `.github/**`, `docs/security/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Aggregate feature-owned fuzz and negative suites; run invalid crypto, fragment/session floods, slot exhaustion, sender rotation and cache pollution against bench devices.
- Measure CPU, peak memory and per-link/global work caps; test backup leakage and sensitive logging across core/native layers.
- Map every design security finding to implementation and evidence, separating accepted risks from remediated defects.

## Exit criteria

- [ ] Every v1 security finding has an evidence-backed status and linked regression or documented manual procedure.
- [ ] Flood throughput cannot cause unbounded resources or verification work; malformed inputs cannot cross trust boundaries.
- [ ] No sensitive material appears in controlled logs/backups; critical/high unresolved defects block release.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If device instrumentation is limited, record what was measured and obtain the missing evidence separately.
- A grep-only check is insufficient for logging safety; add behavioral capture tests where source checks cannot prove it.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Not implemented. Record commands, versions, reproducible inputs and results here. For manual/hardware checks include device/OS, duration and report paths. No test or review is claimed yet.

## Review and merge

- Branch: `ticket/MC-036-integrated-security-and-resource-regression-gate`.
- Review/PR: pending.
- Squash commit title: `MC-036: Integrated security and resource regression gate`.
- Completion becomes effective only when the reviewed squash commit lands on main.
