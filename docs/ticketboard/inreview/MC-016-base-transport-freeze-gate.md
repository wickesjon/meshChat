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

Under the user-approved 2026-09-15 MC-013 sequencing correction, base ingress gates cover admission and clear/opaque pending-state behavior. Real signed/encrypted ingress replay evidence is pending MC-019/020/021 and must pass MC-022; the base freeze cannot label those cryptographic results passed. All base rate/memory/relay/SYNC checks remain required.

Acceptance reference updated under the user-approved MC-004 physical-gate replacement on 2026-09-11; see [the decision](../../decisions/MC-004-online-feasibility.md). MC-025/MC-027 retain physical radio acceptance.

The [base freeze record](../../testing/MC-016-base-freeze.md) defines the stable boundary, compatibility/versioning policy, corpus hashes, exact commands, measured outcomes and remaining owner gates. The normative design links this record without declaring crypto/full-wire security frozen. No runtime, dependency or vector bytes changed.

Runtime source: merged MC-015 `30ba33debac4530db60d6614df49506ab1c18bc8`, clean at measurement; MC-011/015 hard dependencies complete on main. Host Windows x86_64, Rust/Cargo1.85.1, Python3.14.4, cargo-deny0.20.2. Local core fmt/clippy/all-feature debug+release tests (82 each)/release build and dependency gates pass. Standalone simulator fmt/clippy/debug+release fixture tests/build pass with isolated target output. Both locks unchanged. Independent generators match170 logical+15 frame vectors. Twelve board and15 Python simulator regressions, definitions and budget worksheet pass.

Full relay evidence `.work/mc016/all-relay-metrics.json`:108 unique reports/216 executions across18 cases, seeds7/19/43 and both policies, every report clean source and identical rerun. All required lossless pairs deliver; max p95 18,599ms; dense forwarded ratio0.80 (720/900). Stress loss/outage reports missed pairs explicitly. Eighteen real production SYNC exchange cases rerun twice produce36 identical direction/capacity/fixture metrics; worst71.02s versus120s, all8 selected packets+ordered terminal, stored bytes/TTL preserved, with actual framing/ingress/scheduler and concurrent controls. The full8192-byte case remains the explicit1024-byte sizing worksheet (102.02s) plus production budget boundary arithmetic; signed556/encrypted387-byte structural fixtures remain Pending. The separate fixed JSON selection tests preserve mixed-channel and repeated Bloom omissions.

MC-004's approved online feasibility/permission evidence and explicit146-byte refusal/512-byte ceiling remain the contract; physical radio/driver/protected-storage certification is not claimed. The native HELLO/admission handshake is a specified driver obligation, not executed by the pre-admitted host harness. Actual integrated crypto/security and encrypted persistence remain with their owning downstream tickets. All base wire layouts and limits are explicit; no unresolved base ambiguity was found in this review. A discovered implementation defect would block this freeze and require scoped remediation rather than silently changing production code outside this ticket.

This ticket changes documentation only and has no native/interface/build/dependency effect; applicable host+documentation gates and required Terra review satisfy the local validation policy. No full hosted matrix, physical test or independent security assessment is substituted. Review is pending; completion is effective only after accepted review and squash merge.

## Review and merge

- Branch: `ticket/MC-016-base-transport-freeze-gate`.
- Review/PR: pending.
- Squash commit title: `MC-016: Base transport freeze gate`.
- Completion becomes effective only when the reviewed squash commit lands on main.
