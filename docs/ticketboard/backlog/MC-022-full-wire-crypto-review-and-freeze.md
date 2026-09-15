---
id: "MC-022"
title: "Full-wire crypto review and freeze"
depends_on: ["MC-020","MC-021"]
kind: "gate"
branch: "ticket/MC-022-full-wire-crypto-review-and-freeze"
---

# MC-022 — Full-wire crypto review and freeze

## Objective

Run primitive reference vectors and negative cases against the selected construction and canonical encodings.

## Dependencies

`MC-020`, `MC-021` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `tests/vectors/crypto/**`, `tests/integration/**`, `docs/testing/**`, `docs/mesh-chat-design.md`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Run primitive reference vectors and negative cases against the selected construction and canonical encodings.
- Verify vectors through Kotlin and Swift bindings and an independent reference where available; distinguish shared-core parity from independent cryptographic review.
- Obtain independent crypto review of the construction and transcripts before declaring the full wire stable; record findings and disposition.

## Exit criteria

- [ ] MC-019/020/021 provide real-verifier ingress invalid-first/valid-second and authenticated-replay evidence, including failure, eviction, concurrency and budget-available recovery; pending-state fixtures alone cannot satisfy this full-wire gate.
- [ ] All required forgery, replay, binding, key lifecycle and credential-recovery checks pass.
- [ ] Independent review findings affecting the protocol are resolved, with evidence linked.
- [ ] Every v1 wire/QR format is frozen and versioned; no undocumented field remains.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If independent review is unavailable, keep the full-wire freeze and public crypto release blocked.
- If review requires a layout change, update vectors/spec and reopen dependent integrations before release.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Not implemented. Record commands, versions, reproducible inputs and results here. For manual/hardware checks include device/OS, duration and report paths. No test or review is claimed yet.

## Review and merge

- Branch: `ticket/MC-022-full-wire-crypto-review-and-freeze`.
- Review/PR: pending.
- Squash commit title: `MC-022: Full-wire crypto review and freeze`.
- Completion becomes effective only when the reviewed squash commit lands on main.
