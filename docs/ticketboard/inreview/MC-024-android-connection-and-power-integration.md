---
id: "MC-024"
title: "Android connection and power integration"
depends_on: ["MC-023","MC-014"]
kind: "android"
branch: "ticket/MC-024-android-connection-and-power-integration"
---

# MC-024 — Android connection and power integration

## Objective

Implement post-handshake duplicate-link resolution, bounded connection slots, novelty/RSSI selection and idle-peer eviction.

## Dependencies

`MC-023`, `MC-014` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/android/ble/**`, `src/android/power/**`, `tests/bench/android/**`, `src/core/src/native_transport.rs`, `tests/integration/transport/**`, `tests/integration/ffi/**`, `docs/testing/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Implement post-handshake duplicate-link resolution, bounded connection slots, novelty/RSSI selection and idle-peer eviction.
- Wire core Auto/Saver/Normal policy, isolated scanning backoff, charging and permission changes to Android lifecycle APIs.
- Exercise reconnect churn and injected service-termination callbacks with explicit degraded states; retain actual OEM termination scenarios for MC-025.

## Exit criteria

- [ ] Duplicate simultaneous connections settle deterministically without blocking asymmetric discovery.
- [ ] Slot-exhaustion and reconnection tests obey device limits and do not accumulate stale resources.
- [ ] Automated lifecycle/permission/service-stop and threshold tests pass with applicable native checks. Pixel/Samsung/Xiaomi permission, screen-off, OEM termination and measured connection-limit evidence is retained in MC-025.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If an OEM cannot sustain target links, use a measured lower connection target and record coverage consequences.
- If the OS kills service work, expose stopped/degraded operation and recovery instructions rather than implying continued relay.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Scheduling approved 2026-09-14 in [the validation policy](../../decisions/local-validation-policy.md#physical-acceptance-scheduling--approved-2026-09-14). Physical OEM acceptance transfers to MC-025; native build checks and bounded-state regression tests remain implementation gates.

Implementation and validation are in progress on the dedicated branch. No physical test or completed review is claimed. The current evidence below supersedes this ticket's original backlog state.

## Review and merge

- Branch: `ticket/MC-024-android-connection-and-power-integration`.
- Review/PR: pending.
- Squash commit title: `MC-024: Android connection and power integration`.
- Completion becomes effective only when the reviewed squash commit lands on main.

## Approved core-interface scope extension

Status: approved by the user on 2026-09-16. Implementation starts after both hard dependencies completed on main.

### Evidence of the gap

Before the extension, MC-024 permitted only `src/android/ble/**`, `src/android/power/**` and `tests/bench/android/**`, plus its own ticket and generated board. Its acceptance requires authenticated duplicate-link consolidation and the existing core Auto/Saver/Normal policy.

The completed MC-023 `NativeTransport` owns Friends, Ingress and Relay privately and currently fixes admission at six Normal-mode links. Rust already implements `Friends::duplicate_links_to_close` in `src/core/src/friends/proof.rs`, `power::Policy`, `Relay::set_power` and `Ingress::set_link_limit`, but none of those operations is exported by the transport owner. Kotlin cannot reach them through generated bindings. Recreating authenticated arbitration or resetting the core to switch modes would violate existing ownership/budget invariants.

### Approved narrow extension

Approved additional paths: `src/core/src/native_transport.rs`, `tests/integration/transport/**`, `tests/integration/ffi/**`, and `docs/testing/**` to MC-024, solely for this integration and evidence. Reuse the existing test registration and module; no manifest, dependency, lockfile or wire-contract change is proposed. Existing dependencies MC-023 and MC-014 already cover Friends through MC-023/MC-019.

Extend the production transport boundary to apply the existing core power policy and authenticated duplicate-link recommendations, and return bounded, explicitly qualified connection observations required by native selection. Core parsing/authentication remains in Rust. Raw byte arrival, HELLO claims, opaque traffic and pending verification must not be reported as valid CHAT/ANNOUNCE or authenticated peer novelty; expose qualification explicitly and test invalid traffic against idle eviction. Native code uses local discovery/RSSI, core observations and lifecycle state to schedule connections/scans; peer claims never grant identity or display authority. Do not export raw private keys or retain unlocked sessions.

Mode changes must close excess setup/ready links explicitly before lowering the cap, emit terminal effects, retain node/address budgets and pacing, and ignore old callback generations. Implement Auto/Normal/Saver integration; the later Beacon product remains MC-033. Preserve the existing two rotating Normal slots (min(2,L-1) for a lower cap), 60-second reevaluation, 20-second valid-traffic idle rule/five-minute blacklist, bounded discovery records and advisory RSSI diversity. Any missing normative threshold/definition that cannot be resolved from existing decisions remains a separate decision rather than an invented security claim.

### Acceptance and unchanged gates

Exercise both proved duplicate roles, unproved/sole asymmetric links, repeated mode transitions with exhausted credit, cap shrink including setup work, stale effects, bounded selection/churn and all Auto thresholds/hysteresis. Controlled Android tests cover scan backoff, charging, foregrounding, permissions and service termination with explicit stopped/degraded state.

Run Rust formatting/clippy/debug/release/security gates, real Kotlin and Swift binding regressions, Android build/lint/APK checks and affected Mac native checks. Use the ready-PR, separate Terra medium review, correction and squash-merge flow. Physical OEM/radio/battery acceptance remains MC-025; protected-key and independent assessment requirements remain unchanged.

### Scope decision

The user approved these four additional path groups for MC-024 while preserving the current product, wire and security requirements.

Drafted on the dedicated MC-024 branch from main `c3462db52d143a97e36a27524ccabd92b1610b46` after MC-023 PR #27 merged. Both hard dependencies are complete. The explicit scope decision is now approved; implementation is in progress within the extended paths.

## Implementation candidate

The approved bridge applies the existing core Auto/Normal/Saver policy, updates ingress/scheduler limits without resetting credits, returns cancellation effects for cap shrink, and consolidates duplicate connections through the existing proved-session ordering. Core observations expose first/latest qualifying activity and bounded, expiring advisory digest overlap. Full-key friend signatures use an additional budgeted strict activity check; pending content acceptance remains separate. No native parser, key retention, dependency or wire change was introduced.

The Android driver uses a bounded 64-record connection policy, unseen/novelty/RSSI selection, reserved exploration slots, five-minute initial-idle blacklist, bounded reconnect/scanning backoff and explicit degraded/stopped states. Existing server-epoch teardown semantics remain conservative. Battery/charging and actual foreground state drive the core policy; fine/background location restrictions pause scanning visibly. Provider lock/radio/permission loss and service stop close the generation. The application can read the applied ANNOUNCE cadence and battery tier; later feature owners supply actual announcement content.

Implementation choices, application-entry obligations, activity qualification, bounds, source references and remaining physical ownership are recorded in [the connection-policy guide](../../testing/MC-024-android-connection-policy.md).

Current local evidence: Rust/cargo 1.85.1, Python 3.14.4, JDK 17.0.15+6, Gradle 8.13 and Kotlin 2.2.0. All 160 core tests pass in debug and release; formatting and all-target/all-feature clippy with warnings denied pass. Dependency advisory/bans/licenses/sources checks pass with only existing unmatched allowance warnings. Host binding generation and Android arm64-v8a/x86_64 release builds pass with NDK 27.3.13750724. The first Android debug/release/lint candidate and its JVM suite passed; final policy/FFI regressions and Mac execution are still pending. An initial nullable battery-read compiler error and a Windows DLL-in-use rebuild were corrected/sequenced; neither failed invocation is counted as passed. Logs are in `.work/mc024/`.

Review and applicable native completion remain mandatory before merge. No physical radio, OEM, battery or protected-key certification is claimed.
