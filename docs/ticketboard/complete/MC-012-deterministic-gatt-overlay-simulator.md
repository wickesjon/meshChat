---
id: "MC-012"
title: "Deterministic GATT-overlay simulator"
depends_on: ["MC-003","MC-007","MC-009","MC-010"]
kind: "core"
branch: "ticket/MC-012-deterministic-gatt-overlay-simulator"
---

# MC-012 — Deterministic GATT-overlay simulator

## Objective

Build an event-driven simulator using the actual sans-IO core, directed link capacities, bounded queues and per-egress transmissions.

## Dependencies

`MC-003`, `MC-007`, `MC-009`, `MC-010` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `tests/simulator/**`, `tests/integration/simulator/**`, `docs/testing/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Build an event-driven simulator using the actual sans-IO core, directed link capacities, bounded queues and per-egress transmissions.
- Support loss, mobility, churn, partitions, mixed versions, identity rotation, clock skew and power tiers.
- Commit reproducible dense, sparse, cut-vertex and late-join scenario definitions and machine-readable metrics.

## Exit criteria

- [x] A seed reproduces event traces and metrics; link budgets and fragmentation costs match the transport contract.
- [x] Scenarios count each actual egress transmission and distinguish origin sends, relays, fragments and delivery.
- [x] The same harness can run an unsuppressed reference policy without changing production protocol behavior.
- [x] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If runtime grows too large, keep a small deterministic PR suite and schedule a larger fixed-seed sweep.
- Do not substitute a broadcast-radio model for point-to-point GATT to improve delivery numbers.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Implementation independently reviewed; completion is pending the authorized squash merge. MC-009/010 are added as hard dependencies because the runner directly exercises their codec and framing APIs; both are complete on main. The simulator has its own test-only Cargo workspace under its permitted paths. A synthetic fixture policy drives the existing core primitives; production ingress, relay scheduling and SYNC gates remain MC-013/014/015/016, as the MC-007 manifest already specifies. No production protocol acceptance or device evidence is claimed by fixture-policy runs.

### Implementation and validation

- Added a locked standalone test workspace and JSON-line adapter invoking the actual core link lifecycle, time/power boundary, encoder, parser, TTL forwarding and reassembly. Added deterministic Python event scheduling, directed capacities, bounded fixture queues/work budgets and per-egress measurements. The production dependency graph and source are unchanged; the auxiliary lockfile introduces no crate version/checksum absent from the root lockfile.
- Added named late-join, mobility/reconnect, version/identity/clock/power, directional-capacity, capacity-refusal and backpressure cases. MC-007's original acceptance manifest remains specified; its production gates are not relabeled passed.
- See [simulator method and evidence](../../testing/MC-012-simulator.md). Every report identifies synthetic policy versus real primitives, exact source/binary/input digests, denied origins, static recipient denominators, misses, per-egress frames/bytes and resource peaks. No hardware/security/session acceptance is claimed.
- Windows x86_64, Rust/cargo 1.85.1, Python 3.14.4: simulator fmt, clippy (all targets, warnings denied), locked debug/release builds and 15 integration checks pass in both profiles. Full 17-case × three-seed × two-policy suite is replayed once per combination (204 executions/profile), requiring identical full metrics and trace hashes. Reports/traces are generated under ignored `.work/mc012/`.
- All six lossless coverage-fixture topologies have full scheduled TTL-reachable delivery and zero beyond-TTL deliveries. Loss, late join, version/capacity refusal and backpressure retain failed pairs. Exact C146 CHAT cost is three values of 146/146/100 bytes; C512 is one 342-byte value. These test-driver results do not complete MC-014.
- Relevant checks are local under the approved policy. The existing hosted jobs do not discover the independent simulator workspace; changing `.github/**` is outside this ticket. Native Android/iOS source, dependencies and artifacts are unchanged from the fully passing MC-011 revision; simulator host builds/tests and the dependency gate are the relevant checks.
- cargo-deny 0.20.2 advisory/license/version/source gates pass with a freshly fetched RustSec database. The initial bans check correctly rejected the unversioned local path dependency; adding the exact current core version fixed it without relaxing policy. Only unused allowances from the broader production policy produce warnings. MC-007 definition/worksheet checks pass unchanged. Ticketboard default validator, its 12 tests, and `git diff --check` pass.
- Separate Terra medium review [5207059712](https://github.com/wickesjon/meshChat/pull/16#pullrequestreview-5207059712) independently reviewed `ac4e1cbe7d9c360f683d73dc70c1092c08a8120b`, found no blocking issues, and reran targeted Debug/Release integration checks, MC-007 checks, ticketboard validation, diff checks and a chain replay. Full clean-source Debug/Release reports have identical metrics/traces across profiles (102 reports/profile, 408 replayed executions total); only executable digests differ. Final completion metadata is reviewed separately before merge. No fallback was triggered and no acceptance requirement was changed.

## Review and merge

- Branch: `ticket/MC-012-deterministic-gatt-overlay-simulator`.
- Review/PR: [PR #16](https://github.com/wickesjon/meshChat/pull/16); Terra review [5207059712](https://github.com/wickesjon/meshChat/pull/16#pullrequestreview-5207059712).
- Squash commit title: `MC-012: Deterministic GATT-overlay simulator`.
- Completion becomes effective only when the reviewed squash commit lands on main.
