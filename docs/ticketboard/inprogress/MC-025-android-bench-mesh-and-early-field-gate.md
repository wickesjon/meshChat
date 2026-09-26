---
id: "MC-025"
title: "Android integrated bench mesh and field gate"
depends_on: ["MC-024","MC-015","MC-029","MC-030","MC-031","MC-032","MC-033","MC-042","MC-022"]
kind: "gate"
branch: "ticket/MC-025-android-bench-mesh-and-early-field-gate"
---

# MC-025 — Android integrated bench mesh and field gate

## Objective

Run two-, five- and ten-device mixed-OEM trials with controlled topology, late joining, churn and an identity-rotating flooder.

## Dependencies

`MC-024`, `MC-015`, `MC-029`, `MC-030`, `MC-031`, `MC-032`, `MC-033`, `MC-042`, `MC-022` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `tests/bench/android/**`, `tests/bench/beacon/**`, `docs/testing/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

### Two-device exploratory exchange — 2026-09-26

The user supplied a Samsung SM-X510 Android 16 tablet as device B. It was installed with the same fixed Debug candidate as device A (`0f5e0c4`, APK `61c4a84a7ec02490cb83c983422610edb07ab1e6631ccddadc0ecf5000bd0ce7`). After explicit foreground Connect, both devices reported one direct connection. One synthetic public-channel message in each direction was confirmed in the opposite device's rendered history: 2/2 expected pairs in this limited smoke workload. The [acceptance packet](../../testing/MC-025-android-physical-acceptance.md#phonetablet-exchange-smoke--2026-09-26) records device/OS details, method, limitations and ignored local evidence paths. This does not satisfy the required two-phone framing/capacity test or larger mixed-OEM cohorts.

Open finding: phone-side GATT server close/register cycles recur, with framework null-callback warnings during initial observation. A later bounded log snapshot still showed repeated registration while messages could travel. Diagnose and resolve this before claiming duplicate-link convergence, stability, load or power acceptance; do not weaken epoch-based stale-callback protection. No production changes or formal performance claims accompany this evidence checkpoint. All exit criteria remain unchecked. Validate documentation links, board/default/unit checks and whitespace before committing this checkpoint; no PR/final review/merge is implied.

### First physical preflight and Connect failure — 2026-09-26

Device A is a user-provided Samsung Galaxy S24 Ultra (SM-S928U1), Android 16/API 36, build S928U1UES6DZH3, security patch 2026-08-05, arm64 with 4096-byte pages. ADB authorization, installation and onboarding succeeded. Installed Debug APK SHA-256 `2f167a92e6aec3081182028f57a8df373715d1828fa340d6f2001579654294b0` matches the MC-035 handoff artifact. Bluetooth, location and notification permissions were granted. Initial inventory measured 57% battery, USB charging and 30.5°C battery temperature; this is not an MC-007 energy run.

User reported Connect immediately showed “Your data is locked.” App-scoped logs showed GATT server registration, scanning and advertising succeeded, followed by transport shutdown about one second later. Android's current-user lock summary reported unlocked during diagnosis. The root cause and focused retest are recorded below; full physical acceptance remains incomplete. The installed profile, keys and history were retained. Local diagnostics and serial mapping remain ignored under `.work/mc025/2026-09-26-device-a-preflight/`.

Necessary repository-local scope extension under the standing 2026-09-17 approval: `src/android/ui/src/main/kotlin/org/meshchat/ui/MeshModel.kt`, `src/android/ble/src/main/java/org/meshchat/transport/**`, `src/android/app/build.gradle.kts`, and `tests/integration/android-ui/**` may be changed only as needed to diagnose/fix this Connect failure and add focused regression coverage. Tests should use the existing test source sets where possible. Validate affected Android Debug/Release builds, lint/JVM and native regression checks, APK compatibility/alignment, board checks and whitespace; record exact candidate hashes and actual physical retest. Preserve protected-lock behavior and emulator-only runner guards. No wire/crypto changes or reduced physical criteria are authorized by this extension.

Diagnosis: a JDWP constructor breakpoint captured `MessagingException.Unavailable` from `NativeTransport.friendCards`, reached through `MeshModel.startRadio`'s immediate refresh and the `coreWork` path where `radio` was still null. The service starts on the main thread before its queued ready callback assigns the radio to the model. During encrypted-store opening, native radio ticks can advance the transport clock beyond the timestamp captured by that unguarded read; the core correctly refuses backward time. Fix: construct a model-owned monitor before startup, pass it through the service session to `AndroidGattRadio`, and hold it for `coreWork` even before the ready callback. The existing radio lock and monotonic-clock refusal remain in force; no storage, key, wire or cryptographic code changes.

The explicit `ConnectStartupTest` uses the installed synthetic profile and real radio/protection adapters, preserves identity/data/permissions, and exercises three Connect/refresh/stop cycles. It requires `physicalConnect=true`; existing emulator runners are unchanged. The baseline APK failed its first startup assertion on the Galaxy in 9.874 seconds (`connect-before.log`). The updated APK passed the three-cycle regression in 76.843 seconds (`connect-after.log`). Normal app launch/Connect afterward also showed “Searching for nearby people…” with the foreground transport service running (`normal-launch-after.txt`). Device preflight and this focused regression do not satisfy the full OEM, mesh, lock-lifecycle, battery or endurance criteria.

Updated Debug APK SHA-256: `61c4a84a7ec02490cb83c983422610edb07ab1e6631ccddadc0ecf5000bd0ce7`. Focused test APK: `705bca39aaff601ae15b5c4bec5ba5889745cbccc0a94e34e70fef2e5ae088be`. The source is recorded in this branch's `MC-025: fix Android protected startup serialization` checkpoint, with no final review, PR, main merge or ticket completion implied.

Validation uses JDK 17.0.15+6, Gradle 8.13, Kotlin 2.2.0, AGP 8.11.1, Android build tools 35.0.0, Python 3.14.4, Rust/Cargo 1.85.1 and ADB 37.0.1. Local environment/cache configuration is retained at `.work/mc025/android-env.ps1`; logs are under the device-preflight directory above. Applicable commands/results:

- App: `src/android/gradlew.bat -p src/android --offline --no-daemon --max-workers=2 :app:assembleDebug :app:assembleRelease :app:assembleDebugAndroidTest :app:lintDebug :app:testDebugUnitTest` — PASS, all 22 JVM tests including the actual public crypto-vector bridge run. `final-app-checks.log`.
- BLE: `src/android/gradlew.bat -p src/android/ble --offline --no-daemon --max-workers=2 assembleDebug assembleRelease lintDebug` — PASS, including all 26 JVM tests, native-set/ELF checks and Kotlin public-vector bridge tests. `fixed-ble-retry.log`.
- `python -B tests/integration/storage/run_policy.py` — PASS, all 14 shared storage-policy regressions. These use a SQLite double and do not claim device encryption certification. `storage-policy.log`.
- `python -B tests/integration/android-ui/check_apk.py` on Debug/Release APKs and build-tools `zipalign -c -P 16 4` on both — PASS for both ABIs/native sets and 16 KiB ELF/ZIP alignment.
- Physical: targeted `adb -s <device-A> shell am instrument -w -r -e physicalConnect true -e class org.meshchat.ui.ConnectStartupTest org.meshchat.app.test/androidx.test.runner.AndroidJUnitRunner` — baseline FAIL, fixed PASS as above; no reset or blanket permission grants.
- `python -B tests/ticketboard/validate.py`, 12 ticketboard unit tests, changed-document local links and `git diff --check` — PASS.

Initial invocation incorrectly treated BLE as an app subproject; corrected to its standalone Gradle project. Initial BLE and app crypto-vector checks hit Windows Gradle transform-cache rename errors before vector execution; these are failed/unavailable runs, not passes. Sequential BLE and app retries completed actual vectors successfully. An initial storage-policy invocation lacked Cargo on PATH; it passed after loading the repository environment. No cache/security settings or validation assertions were bypassed. Production changes are Android-only synchronization: shared Rust/FFI, packaged native libraries, iOS, wire/crypto construction and storage adapters are unchanged, so fresh iOS builds, full Rust/security assessment and unrelated destructive emulator suites are not required for this narrow fix. The focused real-device regression and existing affected native/JVM tests provide the relevant local evidence; full physical/security/review gates remain mandatory for completion.

### Preparation checkpoint — 2026-09-17

All hard dependencies are complete on main; this branch starts from integrated candidate `e20a05e4c55df857de2dc8168258138bf2092fbc` (MC-035 squash). Prepare `docs/testing/MC-025-android-physical-acceptance.md` within the existing scope: a requirements-to-scenario matrix, preflight/evidence template, unchanged MC-007 measurement rules and explicit blockers. No production changes or acceptance relaxation is planned. Validate board generation/default, board unit tests, whitespace and local documentation links. Procedure preparation is not executed physical acceptance.

- Run two-, five- and ten-device mixed-OEM trials with controlled topology, late joining, churn and an identity-rotating flooder.
- Measure actual GATT transmissions, reachable delivery, latency and battery under the MC-007 workload.
- Compare radio outcomes with simulator predictions and record a small field trial on the integrated Android candidate before MC-034 beta acceptance.

## Exit criteria

- [ ] Deferred MC-023 two-device whole/fragmented bidirectional exchange, runtime capacity and measured backpressure pass; notify subscription, busy/error/stalled callbacks, MTU changes and disconnect recovery are covered on actual devices.
- [ ] Deferred MC-024 Pixel/Samsung/Xiaomi permission, screen-off, OEM service termination, power-threshold, duplicate-link and measured slot/reconnection-limit results pass with explicit degraded states.
- [ ] Deferred MC-033 powered-phone endurance runs six wall-clock hours with bounded memory and no manual recovery; actual power removal/downgrade preserves core state. Sparse-gap beacon/no-beacon trials record coverage, relay work and phone battery/load changes without assuming offload.
- [ ] Recorded bench results meet the approved targets or have resolved deviations with updated evidence.
- [ ] SYNC and bridge tests succeed within the supported hop/capacity model.
- [ ] Battery and flood results include device/OS, duration, baseline and instrumentation limitations.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If field behavior differs from the simulator, fix the model or implementation and rerun the affected scenarios.
- Do not replace missing hardware evidence with simulated numbers or widen acceptance silently.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Scheduling approved 2026-09-14 in [the validation policy](../../decisions/local-validation-policy.md#physical-acceptance-scheduling--approved-2026-09-14). Near completion means the listed Android feature implementations and full-wire gate are complete, before MC-034 acceptance or distribution. This physical gate is deferred, not satisfied; use synthetic messages/identities until MC-043 permits sensitive-data use. Original MC-007 workload/thresholds and all original two/five/ten-device scenarios remain required.

Historical inventory on 2026-09-17 returned no devices after the isolated development emulator was stopped. On 2026-09-26 a Galaxy S24 Ultra and later a Samsung SM-X510 tablet became available, enabling the startup preflight and exploratory exchange above. Two real Android phones, Pixel/Samsung/Xiaomi coverage and five-/ten-device cohorts remain required for the full gate; the tablet smoke does not waive that matrix. The [acceptance packet](../../testing/MC-025-android-physical-acceptance.md) distinguishes these checks from the unexecuted matrix. Candidate-specific workload/measurement instrumentation must also be verified before formal execution; the existing emulator runners deliberately reject physical serials and must not be reused by removing that guard. No completed ticket or final review is claimed.

## Review and merge

- Branch: `ticket/MC-025-android-bench-mesh-and-early-field-gate`.
- Review/PR: pending.
- Squash commit title: `MC-025: Android integrated bench mesh and field gate`.
- Completion becomes effective only when the reviewed squash commit lands on main.

Preparation validation on 2026-09-17: ticketboard regeneration/default validation, all 12 board unit tests, acceptance-packet local-link checks and `git diff --check` pass. These validate documentation only. No PR is published and no final Terra or physical acceptance review is claimed while the gate is incomplete.
