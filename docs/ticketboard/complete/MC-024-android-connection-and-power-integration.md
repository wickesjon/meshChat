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

- [x] Duplicate simultaneous connections settle deterministically without blocking asymmetric discovery.
- [x] Slot-exhaustion and reconnection tests obey device limits and do not accumulate stale resources.
- [x] Automated lifecycle/permission/service-stop and threshold tests pass with applicable native checks. Pixel/Samsung/Xiaomi permission, screen-off, OEM termination and measured connection-limit evidence is retained in MC-025.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If an OEM cannot sustain target links, use a measured lower connection target and record coverage consequences.
- If the OS kills service work, expose stopped/degraded operation and recovery instructions rather than implying continued relay.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Scheduling approved 2026-09-14 in [the validation policy](../../decisions/local-validation-policy.md#physical-acceptance-scheduling--approved-2026-09-14). Physical OEM acceptance transfers to MC-025; native build checks and bounded-state regression tests remain implementation gates.

Implementation and applicable local/native validation are complete on the dedicated branch. Separate Terra production review passed; final metadata review/merge remains pending. No physical test is claimed. The current evidence below supersedes this ticket's original backlog state.

## Review and merge

- Branch: `ticket/MC-024-android-connection-and-power-integration`.
- Review/PR: [PR #28](https://github.com/wickesjon/meshChat/pull/28).
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

Local evidence on implementation revision `417e5d0245494ba6f1fc98bb58e5356180f16b55`: Rust/cargo 1.85.1, Python 3.14.4, JDK 17.0.15+6, Gradle 8.13 and Kotlin 2.2.0. All 160 core tests pass in debug and release; formatting and all-target/all-feature clippy with warnings denied pass. Dependency advisory/bans/licenses/sources checks pass with only existing unmatched allowance warnings. Host binding generation and Android arm64-v8a/x86_64 release builds pass with NDK 27.3.13750724. Final BLE `assembleDebug assembleRelease lintDebug` passes, including 24 JVM tests (eight connection-policy, nine driver, three native transport, two notification queue, one foundation and one wire-vector test). Debug/release APK ABI and ELF checks pass, and both APKs pass `zipalign -c -P 16 4`. An initial nullable battery-read compiler error and a Windows DLL-in-use rebuild were corrected/sequenced; neither failed invocation is counted as passed. Logs are in `.work/mc024/`.

Separate `gpt-5.6-terra` medium review passed exact revision `417e5d0245494ba6f1fc98bb58e5356180f16b55` against main `c3462db52d143a97e36a27524ccabd92b1610b46`, with no actionable findings. The reviewer checked scope, caps/budget continuity, proved duplicate arbitration, activity qualification, bounded selection/backoff and lifecycle cleanup, and independently ran the board validator and diff check. Hosted run [35176509195](https://github.com/wickesjon/meshChat/actions/runs/35176509195) passed Rust/security and ticketboard checks but caught a Swift test compilation typo (`isEmpty()` instead of the Boolean property `isEmpty`). Correction `b7ef9d6428d5893abf7a9e0ab8c79459df8da794` changes only that assertion. Terra follow-up passed that exact revision with no findings; the original failed invocation is not passing evidence. Native rerun [35176934340](https://github.com/wickesjon/meshChat/actions/runs/35176934340) remains pending.

### Native rerun and evidence applicability

The full Mac job in run [35176934340](https://github.com/wickesjon/meshChat/actions/runs/35176934340) passes on corrected source `b7ef9d6428d5893abf7a9e0ab8c79459df8da794`, using pinned Xcode 16.4, Swift 6.1.2 and Rust 1.85.1. This includes the corrected Swift transport trace (signed activity, invalid-signature rejection, power policy, Auto hysteresis and observations), shared device/simulator libraries, unsigned app/BLE/security builds, CryptoKit interoperability, SQLCipher lifecycle/public replay/DM direction separation and native crypto-vector error parity. Rust debug/release/build/storage/dependency gates and ticketboard also pass on this source. Hosted Android app packaging and BLE build/lint pass; its security job is still running.

Only the Swift assertion syntax changed after local Android/Rust validation; production code, Kotlin tests, native libraries and dependency inputs are identical. The final ticket/evidence/board changes do not alter those runtime inputs. Applicable completed checks remain valid under the local-validation policy; a pending or cancelled run is never counted as passing. Final metadata review and merge remain pending. No physical radio, OEM, battery or protected-key certification is claimed.

The local Android build/lint, real binding/driver/policy tests and both ABI/alignment checks cover every changed Android consumer. MC-024 does not modify the Android security probe, protected provider, storage implementation, security test fixtures, dependency versions or CI; their completed synthetic emulator evidence from MC-023 run 35171360291 remains applicable. The new transport operations are exercised by the real Kotlin/Swift/Rust transport tests. The current full hosted Android job is supplemental to that combination; any observed relevant failure must still be resolved before merge. Final review must confirm this applicability rationale.

The ticket is staged in `complete/` only for the reviewed squash commit. Board regeneration/default validation, the 12 validator unit tests and diff check are required on that metadata revision. Completion becomes effective on main; the final merge checkbox is deliberately not asserted before the merge. Final exact-head review and any later hosted results are recorded in PR #28.
