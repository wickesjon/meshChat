---
id: "MC-007"
title: "Budgets and measurable mesh acceptance"
depends_on: ["MC-004","MC-006"]
kind: "decision"
branch: "ticket/MC-007-budgets-and-measurable-mesh-acceptance"
---

# MC-007 — Budgets and measurable mesh acceptance

## Objective

Specify frame/byte/token accounting units, burst windows, per-link and global crypto budgets, queue byte limits and priority fairness.

## Dependencies

`MC-004`, `MC-006` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `docs/mesh-chat-design.md`, `docs/decisions/**`, `tests/simulator/scenarios/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Specify frame/byte/token accounting units, burst windows, per-link and global crypto budgets, queue byte limits and priority fairness.
- Reconcile SYNC eligible-set selection, item/byte limits, time target, mixed-channel relevance and deterministic Bloom false positives; cover maximum-size signed and encrypted traffic.
- Define TTL-reachable chain tests, GATT-send metrics, unsuppressed baselines, fixed traffic/loss/topology seeds and measured battery/latency thresholds.

## Exit criteria

- [x] A budget worksheet demonstrates that the chosen SYNC test workload fits all simultaneous budgets including overhead and control traffic.
- [x] Each simulator gate names denominator, sample duration, seeds, loss/churn parameters and pass threshold.
- [x] Bounded-memory tables cover credentials, senders, orphans, reassembly, caches and outbound queues.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If the recovery target cannot fit safe budgets, narrow the byte-bounded eligible workload or explicitly extend time; do not silently relax ingress protection.
- If suppression gains are smaller than hoped, preserve reachable delivery and report the measured cost rather than forcing an unattainable relay ratio.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Started from main `3267f060b91542305d62ff4ff5325a8982f49d32`, with MC-004 and MC-006 complete. [The budget contract](../../decisions/MC-007-budgets-and-acceptance.md), matching normative design updates and [scenario definitions/checks](../../../tests/simulator/scenarios/README.md) specify actual GATT frame/byte accounting, link/node/reconnect/crypto admission, finite state, fair queues and measured acceptance procedures.

Evidence state is **specified**, with arithmetic and scenario-definition checks executed using Python 3.14. `budget_worksheet.py` checks two simultaneous paced SYNC walks, including HELLO/proof capacity reservations, reverse requests, page/terminal markers, signed ANNOUNCE and reactions. Eight 1024-byte sizing envelopes consume 104 frames/13,060 bytes per direction and complete at 102.02 seconds against the 120-second target, within link/node/frame/byte/forwarding and reserved crypto budgets. Maximum unsigned/friend/organizer sizes (338/443/556 bytes) also pass, completing at 48.02/57.02/71.02 seconds. Future encrypted/proof fixtures and exact crypto work remain MC-008 dependencies; no fake valid proof or cryptographic result is claimed.

`validate_definitions.py` verifies five graph definitions, seeds 7/19/43, link ceilings and scheduled TTL-reachable denominators: chain 99 reachable/nine beyond TTL; cycle/barbell 84, bridge 96 and dense 60 reachable. The manifest specifies 360-second live/loss/churn runs, 120-second SYNC and 600-second adversarial cases. MC-012/013/015/016 must execute the actual core scenarios; these definition checks are not measured delivery results. Physical battery/radio gates and independent security assessments remain separate.

The approved correction already withdrew the unsupported 95-of-100-in-30-seconds and absolute 0.3-relays/node/message targets. This decision selects an eight-item/8192-byte/120-second workload without relaxing ingress. It follows the ticket's explicit fallback to report actual suppression cost, including zero, while preserving reachable delivery: three-fragment transfers paced at one frame/second can outlast the 400ms hold-off, so a universal positive saving cannot be inferred. No physical evidence or security gate is waived. Persistent schema/disk limits remain MC-018; live protocol memory is bounded here.

Validation: both scenario/worksheet commands pass, ticketboard write/default validation, all 12 ticketboard unit tests and `git diff --check` pass. Separate Terra medium review [5184122599](https://github.com/wickesjon/meshChat/pull/9#pullrequestreview-5184122599) at `2bb55b97f874a6361465af39eba6baa95395eb00` requested concrete mixed-channel and deterministic-Bloom fixtures. Versioned inputs now fix subscriptions, held IDs, all Bloom bytes, selected/omitted IDs, pages and display outcomes; the validator recomputes the filter and verifies every expected selection, including the repeated false-positive omission of ID 525. Follow-up review remains required.

CI run [34656577104](https://github.com/wickesjon/meshChat/actions/runs/34656577104) could not start any of its four jobs because account-level Actions runner availability is blocked. No CI steps executed. Local checks are not a substitute for the required CI gate; this ticket remains inreview and cannot merge until runner availability is restored and final CI passes.

## Review and merge

- Branch: `ticket/MC-007-budgets-and-measurable-mesh-acceptance`.
- Review/PR: [PR #9](https://github.com/wickesjon/meshChat/pull/9); actual separate Terra medium reviews are recorded there. This is not an independent cryptographic/security assessment.
- Squash commit title: `MC-007: Budgets and measurable mesh acceptance`.
- Completion becomes effective only when the reviewed squash commit lands on main.
