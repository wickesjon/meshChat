---
id: "MC-026"
title: "iOS CoreBluetooth driver"
depends_on: ["MC-004","MC-016","MC-024"]
kind: "ios"
branch: "ticket/MC-026-ios-corebluetooth-driver"
---

# MC-026 — iOS CoreBluetooth driver

## Objective

Implement central/peripheral roles, write-readiness and notify-readiness flow control, restoration and per-direction capacities.

## Dependencies

`MC-004`, `MC-016`, `MC-024` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/ios/BLE/**`, `tests/bench/ios/**`, `docs/testing/**`, `src/core/src/native_transport.rs`, `src/core/src/relay.rs`, `tests/integration/transport/**`, `tests/integration/relay/**`, `tests/integration/ffi/**`, `.github/workflows/ci.yml`.

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

Implementation is in progress within the approved scope. Native automated evidence and review will be recorded here; physical acceptance remains MC-027.

## Review and merge

- Branch: `ticket/MC-026-ios-corebluetooth-driver`.
- Review/PR: [PR #29](https://github.com/wickesjon/meshChat/pull/29).
- Squash commit title: `MC-026: iOS CoreBluetooth driver`.
- Completion becomes effective only when the reviewed squash commit lands on main.

## Approved transport-boundary scope extension

Status: approved by the user on 2026-09-16. Drafted on the dedicated MC-026 branch from main `c6baf571b1200adefa0f7b01822b096b60d04b91` after MC-024 PR #28 merged. The approved paths and MC-024 dependency are now authoritative; all hard dependencies are complete on main.

### Evidence

Before approval, MC-026 permitted `src/ios/BLE/**`, `tests/bench/ios/**` and `docs/testing/**`. The production transport boundary from MC-023/024 lives in `src/core/src/native_transport.rs`. Its constructor hardcodes Android power/admission policy (six links), and its outstanding native-send handling applies a five-second callback timeout. The existing Rust scheduler already accepts a platform and exposes readiness, but those controls are private behind NativeTransport. Swift cannot select iOS's four-link cap or signal readiness through the current generated interface.

Design section 8.2 requires iOS write-without-response/readiness semantics, no per-write completion timeout, and a fifteen-second no-readiness/no-inbound liveness rule. Copying Android completion behavior or creating a second Swift protocol parser would violate those requirements.

### Approved permitted paths

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

### Scope decision

The user approved these six additional paths/path groups and the MC-024 hard dependency for MC-026. This changes implementation scope only; it does not weaken acceptance, security or device-certification requirements.

## Implementation candidate and local checks

The production Swift adapter and its serialized driver are implemented under `src/ios/BLE/Transport/`. The core iOS constructor selects the existing four-link policy; readiness and explicit queue-full outcomes preserve scheduler budgets, deadlines and fragments without applying Android callback timeouts. The existing SYNC transport wrapper is exposed to a trusted protocol owner and remains deferred on intake. No wire format, dependency, scheduler replacement or protected-key fallback was introduced.

The native adapter uses runtime directional transmit limits, validates the production INFO/characteristics, owns bounded records and subscriptions, waits for readiness callbacks, guards callback generations, and exposes stopped/limited/restoring states. Initial valid-activity eviction, five-minute blacklist, power cap shrink, lock/permission/radio stop, fresh restoration handshakes and foreground catch-up are integrated. Conservative manager-epoch replacement and application-entry obligations are documented in [the driver guide](../../testing/MC-026-ios-corebluetooth-driver.md). Real radio/OS restoration and key certification remain MC-027/044.

Local candidate checks pass: Rust/cargo 1.85.1 formatting and all-target/all-feature clippy with warnings denied; all 165 tests in debug and release; release build; cargo-deny 0.20.2 advisories/bans/licenses/sources (existing unmatched allowance warnings only). Host bindings and both Android release ABIs build with NDK 27.3.13750724. BLE `assembleDebug assembleRelease lintDebug` passes with all 24 JVM regressions, both ABI/ELF alignment checks, JDK 17.0.15+6, Gradle 8.13 and Kotlin 2.2.0. Logs are under `.work/mc026/`. Swift/native checks run on the pinned hosted Mac; their results and exact review revisions are still pending, not claimed passed.

### Review corrections

Terra medium reviewed `6a12edec66c0355ed88f47b1c1acb26535be73f6` and reported one P1: peripheral restoration removed services without republishing or clearing old subscriber/core state. Restoration now clears the affected role through the serialized driver and schedules a fresh peripheral manager, whose powered-on callback republishes services and advertising. A deterministic regression verifies cleanup before rebuilding, preservation of the other role, fresh admission/HELLO and rejection of old callbacks. Follow-up review is pending.

An additional regression reproduced lost terminal expiry events when a native submission crossed the object's absolute deadline. Completion and backpressure now return those effects without retrying the expired frame; the 14 native transport tests pass after the fix. Full revised-candidate validation remains pending.

Hosted run `35179676215` failed Swift host compilation because a nested test helper lacked explicit main-actor isolation. The helper is corrected; this failed run is not passing Swift or Xcode evidence. A fresh run must validate the correction and execute the previously skipped native builds.

Terra follow-up review PASS on `6698c70f9dc3a91bf11c5c7d34fcc15ec91d7a73`: restoration and terminal-expiry fixes accepted, no remaining review findings. Revised local debug/release suites pass (166 tests each), formatting/clippy/release build pass, regenerated Android bindings and both release ABIs build, BLE Debug/Release/lint and package ABI/ELF/ZIP alignment checks pass. All 24 JVM tests were explicitly rerun (`testDebugUnitTest --rerun-tasks`) against the revised library. The first release attempt encountered persisted PID-named test databases; archiving that ignored fixture directory and rerunning with fresh databases passed without source changes.

Hosted run `35180151353` on that revision passed Rust (including storage policy and cargo-deny), ticketboard, Swift 6 host FFI/production-driver regressions and the iOS skeleton. The BLE Xcode build exposed a Swift 6 concurrency error from carrying a Foundation notification into a main-actor closure. The callback now extracts its Sendable name before actor entry. A fresh Xcode run is required; skipped security/storage stages are not claimed passed. Android runner provisioning failed with `ECONNRESET` while downloading Java, before source checks; applicable local Android results above supply that evidence under the approved policy.
