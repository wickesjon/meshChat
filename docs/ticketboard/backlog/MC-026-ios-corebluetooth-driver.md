---
id: "MC-026"
title: "iOS CoreBluetooth driver"
depends_on: ["MC-004","MC-016"]
kind: "ios"
branch: "ticket/MC-026-ios-corebluetooth-driver"
---

# MC-026 — iOS CoreBluetooth driver

## Objective

Implement central/peripheral roles, write-readiness and notify-readiness flow control, restoration and per-direction capacities.

## Dependencies

`MC-004`, `MC-016` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/ios/BLE/**`, `tests/bench/ios/**`, `docs/testing/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Implement central/peripheral roles, write-readiness and notify-readiness flow control, restoration and per-direction capacities.
- Support the asymmetric discovery strategy and established-link recovery with bounded reconnect state.
- Connect native lifecycle events to the shared core without assuming background execution is continuous.

## Exit criteria

- [ ] Automated native-adapter tests cover all base frame kinds in both roles, readiness callbacks and directional runtime limits; relevant Mac/Xcode builds and regressions pass. MC-027 retains physical exchange acceptance.
- [ ] Injected background, suspension, disconnect, restoration and foreground catch-up transitions produce the expected bounded state in automated tests; real OS execution and timing remain MC-027 acceptance.
- [ ] Unsupported discovery/force-quit cases are distinguished from regressions.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If background participation is intermittent, retain foreground catch-up and honest degraded status.
- If an achievable discovery pairing fails, block interop and revise the measured transport contract rather than claiming parity.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Scheduling approved 2026-09-14 in [the validation policy](../../decisions/local-validation-policy.md#physical-acceptance-scheduling--approved-2026-09-14). All physical driver/lifecycle/discovery scenarios transfer to MC-027 after feature integration; missing Mac/Xcode is still a native build blocker.

Not implemented. Record commands, versions, reproducible inputs and results here. For manual/hardware checks include device/OS, duration and report paths. No test or review is claimed yet.

## Review and merge

- Branch: `ticket/MC-026-ios-corebluetooth-driver`.
- Review/PR: pending.
- Squash commit title: `MC-026: iOS CoreBluetooth driver`.
- Completion becomes effective only when the reviewed squash commit lands on main.

## Proposed transport-boundary scope extension

Status: proposed, awaiting explicit user approval. Drafted on the dedicated MC-026 branch from main `c6baf571b1200adefa0f7b01822b096b60d04b91` after MC-024 PR #28 merged. No MC-026 implementation has started; the current permitted paths and dependencies remain authoritative until approval.

### Evidence

MC-026 currently permits `src/ios/BLE/**`, `tests/bench/ios/**` and `docs/testing/**`. The production transport boundary from MC-023/024 lives in `src/core/src/native_transport.rs`. Its constructor hardcodes Android power/admission policy (six links), and its outstanding native-send handling applies a five-second callback timeout. The existing Rust scheduler already accepts a platform and exposes readiness, but those controls are private behind NativeTransport. Swift cannot select iOS's four-link cap or signal readiness through the current generated interface.

Design section 8.2 requires iOS write-without-response/readiness semantics, no per-write completion timeout, and a fifteen-second no-readiness/no-inbound liveness rule. Copying Android completion behavior or creating a second Swift protocol parser would violate those requirements.

### Proposed permitted paths

Add `src/core/src/native_transport.rs`, `src/core/src/relay.rs`, `tests/integration/transport/**`, `tests/integration/relay/**`, `tests/integration/ffi/**`, and `.github/workflows/ci.yml` solely for the platform/readiness boundary, bounded scheduler backpressure handling, regressions and execution of the new native-adapter tests. Existing MC-026 paths cover the Swift adapter, Xcode project, native-adapter tests and evidence. Add MC-024 to authoritative hard dependencies, retaining MC-004 and MC-016; MC-024 transitively supplies MC-023 and the protected identity/friends owners. No implementation begins until that dependency is complete on main.

Relay currently distinguishes only success and failure; two failures terminate an object. A CoreBluetooth notification refusal means wait for readiness, so the narrow scheduler scope permits distinguishing that backpressure from actual failure while retaining object deadlines and charging every actual submission. CI scope permits compiling/running the new production-adapter tests on the existing pinned Mac runner. Do not authorize arbitrary core refactoring, new dependencies, manifests, wire/security changes, a new scheduler or unrelated workflow changes.

### Intended integration

- Select the existing iOS platform policy without changing Android defaults or its five-second completion handling.
- Feed actual CoreBluetooth directional capacities, connection generations, readiness and bounded monotonic lifecycle events into the shared transport owner.
- Distinguish OS acceptance from peer delivery. A write-without-response is submitted only when canSendWriteWithoutResponse is true; there is no didWriteValueFor wait. A refused updateValue is held boundedly until notification readiness, without falsely completing the frame or multiplying retries. Readiness, object deadlines, pacing and credits remain authoritative.
- Enforce iOS liveness independently of Android per-frame callbacks. Preserve admitted/setup caps, terminal cleanup, reassembly expiry, budget continuity, protected identity lifecycle and stale-callback rejection through background, suspension, restoration and foreground catch-up. Restoration never fabricates continuous execution or resurrects an unlocked identity session.
- Retain the asymmetric discovery constraints and explicit stopped/degraded state. Hardware certification and real restoration timing remain MC-027.

### Required validation

Rust debug/release, formatting/clippy/security gates; actual Swift and Kotlin boundary regressions; deterministic iOS adapter traces with both roles, all base kinds, measured-capacity refusal, central/peripheral backpressure, stale callbacks and suspension/restoration cleanup; Mac/Xcode device and simulator Debug/Release compilation; Android regression checks for the shared boundary. Retain all independent security and physical gates. Publish a ready PR, obtain a separate Terra medium review, fix findings, and squash merge only after applicable checks pass.

### Decision requested

Approve these six additional paths/path groups and the MC-024 hard dependency for MC-026. This changes implementation scope only; it does not weaken acceptance, security or device-certification requirements.
