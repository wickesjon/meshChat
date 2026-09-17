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

- [ ] Only adopted, valid credential chains produce staff badges and pinned authority.
- [ ] Root/staff expiry, invalid provisioning and credential recovery are reflected correctly without hidden trust changes.
- [ ] A provisioned staff device posts a signed update received through ordinary non-adopting relays.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If provisioning validation fails, preserve current trust and discard the candidate secret safely.
- If credentials expire offline, disable authoritative posting until explicitly reprovisioned; do not extend expiry.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Implemented: canonical native organizer wrapper sharing ingress/scheduler and protected storage; explicit event/staff confirmation; protected one-credential Android staff vault with generation binding and failure recovery; private scanner without result-intent/private saved state; secure-window/obscured-touch guards; verified/expired/unsigned event presentation, signed pin controls, bounded public credential recovery and per-fragment authority checks. Public wire/trust contracts are unchanged. Authenticated local history provenance now includes the adoption revision so removal/re-adoption cannot reactivate old badges/pins. Pre-UI records without a revision remain visible without authority. See [Android workflow](../../organizer/android-workflow.md).

Initial component tests: native transport signed update through a non-adopting bridge and fail-closed adoption/import/egress regression pass; all 23 existing organizer tests pass after the provenance change. Strict full-workspace clippy passed before the final rate-gate/test additions. The first Android compile passed; lint found a native scanner string literal, now moved to resources. Full Rust/Android checks and emulator/Swift consumer validation are in progress. No final review or physical/security certification is claimed.

## Review and merge

- Branch: `ticket/MC-030-android-organizer-ui-and-staff-provisioning`.
- Review/PR: pending.
- Squash commit title: `MC-030: Android organizer UI and staff provisioning`.
- Completion becomes effective only when the reviewed squash commit lands on main.
