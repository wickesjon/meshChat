---
id: "MC-013"
title: "Ingress budgets, authentication states and dedup"
depends_on: ["MC-010","MC-007","MC-012"]
kind: "core"
branch: "ticket/MC-013-ingress-budgets-authentication-states-and-dedup"
---

# MC-013 — Ingress budgets, authentication states and dedup

## Objective

Charge bytes/frames before allocation and apply global/per-link expensive-work limits at each actual operation.

## Dependencies

`MC-010`, `MC-007`, `MC-012` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/core/**`, `tests/integration/ingress/**`, `tests/simulator/**`, `tests/fuzz/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Charge bytes/frames before allocation and apply global/per-link expensive-work limits at each actual operation.
- Separate bounded attempt tracking from accepted-message dedup so invalid copies cannot suppress later authenticated packets; preserve bounded rejection of repeated garbage.
- Implement per-sender/class and unknown-type limits, control-packet admission and reconnect-resistant abuse accounting within documented limits.

## Exit criteria

- [ ] Invalid-first/valid-second and identical-replay tests pass for signed and encrypted test fixtures.
- [ ] Flooding rotated identities cannot exceed the sum of the attacker's actual admitted link budgets; multi-link attacks are measured separately.
- [ ] Over-budget frames never enter display, reassembly or crypto paths; memory and work remain within specified bounds.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If authentication is pending, use the explicit unverified/pending state without adding a trusted dedup entry.
- If budget pressure prevents verification, drop or defer within a bounded queue; never label unverified traffic authenticated.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Not implemented. Dependencies MC-010/007/012 are complete on main; branch starts from `c3ef8dba9b9bd0ce4400622d84c374ac2b51a92c`. No production change, passing crypto fixture or completion is claimed.

### Sequencing blocker and proposed scope decision — 2026-09-15

The first exit criterion requires signed/encrypted invalid-first/valid-second and identical-replay tests. The existing crypto vectors are explicitly transcript/size fixtures only, with no valid ciphertext or authenticated application implementation. The [MC-008 dependency approval](../../decisions/MC-008-dependency-scope-proposal.md) reserves HPKE integration/exceptions for MC-020 (MC-017 provider integration as needed), and MC-020 depends on MC-016. MC-016 depends on MC-015, which depends on MC-014, which depends on MC-013. Adding MC-020 as an MC-013 hard prerequisite creates a cycle; the ticket validator reproduced that on an in-memory graph without changing authoritative dependencies.

The pending-authentication fallback permits safe implementation states, but it does not explicitly waive or transfer the real cryptographic replay exit criterion. Completing MC-013 based only on a test verifier or structural codec fixtures would overstate its evidence. Implementing HPKE here would also exceed the approved owner/dependency scope. AGENTS.md requires an explicit scope/acceptance decision rather than either shortcut.

**Proposed adjustment, pending user approval:**

1. Replace only MC-013's first exit criterion with: “Malformed-first/structurally-valid-second and identical-replay tests pass for clear and opaque signed/encrypted fixtures at the admission/state-machine boundary. Signed/encrypted inputs remain explicitly pending/unverified and never enter trusted dedup or authenticated display before a production verifier succeeds. These are ingress/state tests, not cryptographic validation.” Keep its rate, memory, multi-link abuse, review and merge criteria unchanged.
2. Add explicit end-to-end ingress replay criteria to MC-019 (friend signatures), MC-020 (encrypted CHAT/REACTION) and MC-021 (organizer signatures/credentials): real invalid-first/cryptographically-valid-second inputs sharing an ID must permit the later valid acceptance once budget is available; identical authenticated replay must not repeat effects or refresh trust. Failures, eviction, simultaneous arrivals and budget-available recovery must use the actual production verifier and ingress implementation, with no test-verifier substitute.
3. Make MC-022's full-wire freeze explicitly require those integrated ingress/crypto results from MC-019/020/021. Their existing dependencies already provide the necessary ordering, so no cyclic prerequisite is added. MC-016 retains base admission/state/relay/SYNC gates and explicitly records the cryptographic portion as pending until MC-022.
4. Update the active implementation plan to state the same evidence split. Do not change wire/security requirements, dependency exceptions, the independent assessment requirement, or any beta/release/physical gates. No test is marked passed by this scheduling correction.

Requested scope addition for this coordinated planning correction: the MC-016/019/020/021/022 ticket files and `docs/ticketboard/implementation-plan.md`, alongside MC-013's own ticket. No production code, dependency integration or normative crypto construction change is proposed by this correction. The actual edit remains unapplied until the user decides. Alternative: retain the existing criterion and leave MC-013/descendants blocked; independent ready tickets can still proceed.

Validation of this blocker record: unchanged authoritative DAG remains valid (46 tickets, 127 dependencies); the proposed MC-020→MC-013 edge is cyclic in the isolated validator check. No fallback has been declared completed.

## Review and merge

- Branch: `ticket/MC-013-ingress-budgets-authentication-states-and-dedup`.
- Review/PR: pending.
- Squash commit title: `MC-013: Ingress budgets, authentication states and dedup`.
- Completion becomes effective only when the reviewed squash commit lands on main.
