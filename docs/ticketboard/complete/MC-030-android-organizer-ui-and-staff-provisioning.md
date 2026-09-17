---
id: "MC-030"
title: "Android organizer UI and staff provisioning"
depends_on: ["MC-028","MC-021","MC-041"]
kind: "android"
branch: "ticket/MC-030-android-organizer-ui-and-staff-provisioning"
---

# MC-030 — Android organizer UI and staff provisioning

## Objective

Implement event discovery prompts, canonical root adoption and staff-only credential import confirmations.

## Dependencies

`MC-028`, `MC-021`, `MC-041` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/android/ui/**`, `tests/integration/android-ui/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Scope extension approved — 2026-09-17

All hard dependencies are now complete on main: MC-041 was squash merged as `7e2219d53c4572d5c149e211950f9f4680b63979` after final Terra medium review of `ce3cc0ced5e10644db2e256bf199116fcad6d634` in PR #35. This dedicated branch starts from that main revision. Production implementation started only after the scope approval recorded below.

The existing canonical `Organizer` is Rust-only. Android currently has no production organizer binding, no protected staff-key owner, and no organizer authentication path in its shared native transport. MC-041's security-probe adapter is test-only and cannot supply a shipping UI. MainActivity currently routes only public channel/friend scanner input, so private provisioning also requires explicit app-owned scanner/lifecycle protection. Implementing UI alone would not satisfy this ticket's acceptance criteria.

Approved extension, limited to MC-030 organizer integration:

- `src/core/src/native_organizer.rs`, `native_transport.rs`, `native_messaging.rs`, `organizer.rs`, `storage.rs`, `lib.rs`, and `src/core/Cargo.toml`: export the existing canonical organizer operations, connect verified ingress/egress and bounded credential recovery to the existing transport, expose public authority/expiry state, and register regression tests. Preserve canonical wire formats, trust checks and storage security requirements.
- `src/android/security/src/main/java/org/meshchat/identity/**` and `src/android/security/src/main/java/org/meshchat/storage/**`: protected staff credential/key ownership with explicit import confirmation, identity-generation binding, lock/background/reset/forget invalidation and fail-closed recovery. Use existing protected-storage contracts; no plaintext key persistence or root private key on phones.
- `src/android/ble/src/main/java/org/meshchat/transport/GattDriver.kt` and `MeshTransportService.kt`, only if needed to connect organizer operations to the existing serialized transport/lifecycle owner.
- `src/android/app/src/main/java/org/meshchat/app/**`, `src/android/app/src/main/AndroidManifest.xml`, and `src/android/app/build.gradle.kts`: app/scanner routing, sensitive provisioning window and obscured-touch protection, lifecycle clearing and test registration. No private URI sharing/export or persistence in activity state/logs.
- `tests/integration/ffi/**` and `.github/workflows/ci.yml`: generated Kotlin/Swift consumer regressions and registration of this ticket's Android/core checks. Existing `tests/integration/android-ui/**` remains the primary regression location.
- `docs/organizer/**`: operator instructions for Android adoption/provisioning/recovery and the distinction between implemented safeguards and later physical acceptance.

Validation will include canonical invalid/expired/mismatched-chain cases, explicit adoption/replacement and failed-import preservation, protected key lifecycle/recovery tests, badge/pin authority revalidation, bounded recovery, two staff identities posting through a non-adopting transport, Android build/lint/emulator UI checks and shared Swift/native checks. Required Terra review follows the ready PR, then fixes/review and squash merge. No product, wire/trust, physical-device or independent-security requirement is waived.

The user explicitly approved this extension with “yes” on 2026-09-17 before implementation. The user subsequently approved necessary future scope extensions within the repository. `AGENTS.md` records that standing decision in this change. Additional paths will still be documented with purpose and validation; product/security requirements and acceptance gates remain unchanged.

Additional path under standing repository-local approval: `src/android/app/src/main/res/values/strings.xml` supplies the new native scanner label required by Android lint. Validation: app lint/build and scanner UI checks.

## Implementation details

- Implement event discovery prompts, canonical root adoption and staff-only credential import confirmations.
- Build verified updates, bounded unverified/pending drawer, signed pin controls and credential-expiry states.
- Protect sensitive provisioning screens from accidental sharing/logging and use platform-supported obscured-touch protections.

## Exit criteria

- [x] Only adopted, valid credential chains produce staff badges and pinned authority.
- [x] Root/staff expiry, invalid provisioning and credential recovery are reflected correctly without hidden trust changes.
- [x] A provisioned staff device posts a signed update received through ordinary non-adopting relays.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If provisioning validation fails, preserve current trust and discard the candidate secret safely.
- If credentials expire offline, disable authoritative posting until explicitly reprovisioned; do not extend expiry.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Implemented: canonical native organizer wrapper sharing ingress/scheduler and protected storage; explicit event/staff confirmation; protected one-credential Android staff vault with generation binding and failure recovery; private scanner without result-intent/private saved state; secure-window/obscured-touch guards; verified/expired/unsigned event presentation, signed pin controls, bounded public credential recovery and per-fragment authority checks. Public wire/trust contracts are unchanged. Authenticated local history provenance now includes the adoption revision so removal/re-adoption cannot reactivate old badges/pins. Pre-UI records without a revision remain visible without authority. See [Android workflow](../../organizer/android-workflow.md).

Corrected production source revision: `6241134743e274e29791cacf8e6f255221004afe`. Terra medium follow-up reviewed that exact revision and reported PASS with no actionable source blocker. A later test-only assertion explicitly checks refused background private access, alongside the existing import/submission/stale-resume checks; the final Android run includes it. Subsequent evidence/document changes do not change native production inputs.

Validation on 2026-09-17:

- Windows Rust/Cargo 1.85.1: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`; `cargo test --workspace --all-features --locked` and the same with `--release` pass 202 tests each; `cargo build --workspace --all-features --locked --release` passes. Four native organizer tests include two staff identities through framed non-adopting relay delivery, invalid/unadopted import and canceled egress, authority/pin expiry, removal/re-adoption, same-ID distinct content and inert discovery. Existing 23 canonical organizer regressions also pass. `python -B tests/integration/storage/run_policy.py` passes 14 checks. Cargo-deny 0.20.2 advisory/license/source/version gates pass with existing unused-license-allowance warnings only.
- `python -B src/core/build_bindings.py android` and its `--security-probe` variant regenerate host Kotlin/Swift and arm64-v8a/x86_64 libraries. NDK 27.3.13750724; JDK 17.0.15+6, Gradle 8.13, AGP 8.11.1, Kotlin 2.2.0, compile SDK 36/build tools 35.0.0. App and security Gradle Debug/Release/test APK builds, lint and host tests pass: 21 app tests, including three staff-vault tests; one security organizer-import test covers its 11 cases. `python -B tests/integration/identity/run_kotlin.py` passes provision/reopen/sign/agree/lock/key-loss/reset plus seven injected recovery faults using test-only wrapping/storage.
- App and security debug/release APKs pass their `check_apk.py` / `check_android_apk.py` native-library-set and 16-KiB ELF LOAD/RELRO alignment checks, and `zipalign -c -P 16 4`. Reused SQLCipher 4.17.0 and Compose graphics aligned artifacts are unchanged from prior tickets; no dependency upgrade is included.
- Explicit isolated `emulator-5580`: Android API 29 x86_64 image revision 8, emulator 37.1.11, WHPX, adb 37.0.1, runtime page size 4 KiB. Channel baseline and organizer import/reopen/forget runners pass on the corrected APK. Real AndroidKeyStore/SQLCipher operations protect synthetic keys/history; native frames are delivered in process, not by physical BLE. Verified/reopened/unverified screenshots contain only public synthetic content and were inspected. This is functional evidence, not hardware or 16-KiB-runtime certification.

Additional checks: the existing friend UI runner passes all pair/reopen/replacement phases on the corrected production APK. MC-005 protected-key emulator create/reopen/key-loss phases pass. MC-018 initially refused its `create` precondition because the reused synthetic security app still held a database/envelope from prior runs; after clearing only `org.meshchat.securityprobe` on the explicitly checked emulator, all create/reopen/checks/key-loss/reset phases pass. No production source change was required for that fixture reset. Both corrected sharing phases now pass, including native ACTION_VIEW confirmation, exact offline QR, PNG export and sensitive clipboard marking. The changed assertion was rebuilt with `:app:assembleDebugAndroidTest :app:lintDebug`; both pass. All three billing purchase/reopen-refund/link-cap phases pass; test store responses are synthetic and do not certify live purchases. All Beacon enable/reopen-exit/enable/real-touch phases pass, including its actual two-second hold target. Synthetic power inputs do not certify physical endurance or battery draw.

- [Corrected Mac job 105345933079](https://github.com/wickesjon/meshChat/actions/runs/35263739811/job/105345933079) passes on `6241134743e274e29791cacf8e6f255221004afe`: macOS 15.7.9 arm64, Xcode 16.4 (16F6), Swift 6.1.2. Regenerated Rust/Swift libraries, the new production organizer FFI trace, existing FFI/supporter/security/import/SQLCipher regressions, and unsigned skeleton/BLE/security Debug/Release device/simulator builds pass. The later changes are Android test assertions and documentation only; native production inputs and Swift checks are identical to that passed revision. Hosted Rust and ticketboard also pass. The complete hosted Android job is supplemental under the approved local-validation policy; its current build/lint results are not substituted for unexecuted phases. Logs/generated fixtures remain under ignored `.work/mc030/`; no private URI, key, QR image or message content is committed. Physical radio/OEM/protection evidence remains MC-025/027/043/044 and integrated independent assessment remains separately required.

## Review and merge

- Branch: `ticket/MC-030-android-organizer-ui-and-staff-provisioning`.
- Review/PR: [PR #36](https://github.com/wickesjon/meshChat/pull/36).
- Squash commit title: `MC-030: Android organizer UI and staff provisioning`.
- Completion becomes effective only when the reviewed squash commit lands on main.

### Initial review and corrective validation

Terra medium reviewed `76889c6ff48c687b85d2724b3af0e98fd1a5ce67` and found one P1: queued model background cancellation could lag native staff-fragment submission. The correction closes a synchronized foreground gate immediately at lifecycle notification and checks it at actual native submission. Resume uses a revision so stale queued foreground work cannot reopen the gate after a newer background notification. A concurrent JVM regression covers in-flight submission, immediate closure before model cleanup, refused private access/import, and stale resume. The author also corrected event row identity/dedup to use the immutable content digest, preserving distinct same-ID variants; a native regression covers those rows and untrusted EVENT_INFO discovery without adoption.

Initial full Rust/Android checks and all three organizer emulator phases passed. Synthetic verified/reopened/unverified screenshots were inspected. Hosted iOS job 105342013580 in run 35262674458 passed all Swift/native and storage checks. The corrected source changed the exported event row record, so both native consumers were regenerated and rerun as recorded above; follow-up Terra review passed the corrected source. The initial results were not substituted for those checks.

Additional path under standing repository-local approval: `tests/integration/sharing/android/SharingUiTest.kt` updates the existing invalid-input message assertion to include newly supported public event links. The corrected app reached this assertion but the old literal timed out; rejection behavior itself is unchanged. The test APK/lint rebuild and both sharing phases subsequently passed.


### Final completion staging

All implementation exit criteria and applicable local/native checks are satisfied. The complete folder is staged for final exact-revision/evidence review and squash merge; the combined review/merge checkbox remains unchecked in this pre-merge snapshot. Completion becomes effective only on main. Final reviewed revision and actual Terra outcome are recorded in PR #36, avoiding a self-referential commit hash here. The supplemental hosted Android job on the corrected source has passed app/package/BLE checks and is still running security checks; it is not claimed fully passed. Matching local app/security native, host, emulator and packaging checks passed under the approved local-validation policy. Required Mac, Rust and ticketboard jobs on the corrected native revision passed. Final metadata and Android test-only changes do not alter those native production inputs.

Board regeneration/default validation, 12 ticketboard tests, and whitespace checks pass on this staged completion before final review. No fallback weakened trust: invalid provisioning preserves the prior credential; unavailable physical certification stays with its explicit later gates.
