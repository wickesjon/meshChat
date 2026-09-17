---
id: "MC-029"
title: "Android friends and encrypted messaging UI"
depends_on: ["MC-028","MC-020"]
kind: "android"
branch: "ticket/MC-029-android-friends-and-encrypted-messaging-ui"
---

# MC-029 — Android friends and encrypted messaging UI

## Objective

Build friend QR/scanner, explicit fingerprint/petname confirmation, friend management and approved nearby/last-seen behavior.

## Dependencies

`MC-028`, `MC-020` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/android/ui/**`, `tests/integration/android-ui/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Build friend QR/scanner, explicit fingerprint/petname confirmation, friend management and approved nearby/last-seen behavior.
- Build DM list/threads and encrypted reactions using authenticated core events and stored conversation identity.
- Implement changed-key blocking, missing-key states and clear confidentiality/metadata information.

## Exit criteria

- [ ] Adding friends, encrypted send/receive, reactions, restart and friend removal work end to end.
- [ ] Deep-link and scanned-key flows require explicit trust confirmation; spoofed nicknames cannot gain a verified badge.
- [ ] Key changes block sending until re-pairing; no UI action causes plaintext DM fallback.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If camera access is denied, offer the validated explicit link/code flow with its source-trust warning.
- If live presence proof is unavailable, show last authenticated observation using MC-008 semantics.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Not implemented. Hard dependencies are complete on main `dd9e006eaf6844ff128ea0a4c7000e26c9216391` after MC-028 PR #30. The proposal below is prepared on this ticket's dedicated branch. Implementation is blocked on the explicit scope decision; the current permitted paths remain unchanged pending approval. No feature test or review pass is claimed.

## Review and merge

- Branch: `ticket/MC-029-android-friends-and-encrypted-messaging-ui`.
- Review/PR: pending.
- Squash commit title: `MC-029: Android friends and encrypted messaging UI`.
- Completion becomes effective only when the reviewed squash commit lands on main.


## Proposed friend and encrypted-message integration extension

Status: **pending user approval**, prepared 2026-09-17 UTC. This proposal does not authorize implementation by itself.

### Concrete blocker

Current scope permits only Android UI and its tests. MC-019's `Friends` and MC-020's `Dms` are Rust APIs without application-facing UniFFI exports. `NativeTransport` owns live ingress budgets, link/session state and friend verification; the application must not construct a second independent ingress owner for incoming DMs. The Android identity adapter deliberately keeps `IdentityKeySession` private and operation-scoped. The manifest/build also lacks camera/scanner and friend-link entry integration. UI-only changes cannot deliver the real-core encrypted messaging, QR/deep-link and persistence criteria.

### Additional paths requested and limits

Retain existing UI/test paths and add only:

- `src/android/app/**`: friend-link entry and unconfirmed proposal state, camera permission and offline QR/camera dependencies, source/test wiring. Request camera access only from the explicit scanner action; preserve the validated link/code fallback when denied. Installation sites, store deployment and domain associations remain MC-031.
- `src/android/security/src/main/java/org/meshchat/identity/IdentityProvider.kt` and `src/android/security/src/main/java/org/meshchat/storage/EncryptedStorage.kt`: narrow protected adapters for friend confirmation/removal, signing/session proof, DM authentication/composition and encrypted history. Preserve identity/reset serialization and unlock checks; invalidate sessions and close databases before returning. Never expose seeds, shared secrets, live sessions or database handles to the UI or callbacks that outlive a protected operation.
- `src/android/ble/src/main/java/org/meshchat/transport/**`: only protected proof/authentication/send hooks and core-effect delivery needed to connect the adapters to the existing serialized driver. Preserve connection/power/lifecycle policy, capacities and queue limits. Revalidate pinned tuple/generation at actual egress; removal/replacement must invalidate stale queued sends.
- `src/core/src/native_messaging.rs` (new), `src/core/src/native_transport.rs`, `src/core/src/native_channels.rs`, `src/core/src/lib.rs`, `src/core/Cargo.toml`: bounded UniFFI/application integration around existing `Friends`, `Dms`, link grammar, text and encrypted-store owners; module/test registration. Use one authoritative live ingress/crypto budget and verifier state. Export confirmation proposals, public fingerprints/petnames, authenticated history/reactions and supported presence observations. A separate authenticated channel presentation path may consume successfully verified signed content; the unsigned boundary must continue rejecting pending/misclassified signed bytes. No wire, cipher, signature transcript, replay window, budget, persistence schema or trust-policy changes.
- `tests/integration/ffi/**`, `tests/integration/ble/**`, `.github/workflows/ci.yml`, `docs/testing/**`: changed native-boundary/driver regression coverage, Android UI/emulator acceptance and applicable Swift/build checks, with durable evidence. Existing `tests/integration/android-ui/**` owns new feature tests. No physical gate is removed.

Permit this ticket's implementation/exit wording to be clarified below. Dependencies remain MC-028 and MC-020; their completed closure already includes the required driver, storage, friend and full-wire gates.

### Required behavior and security clarification

Implement friend QR display/scan and validated friend-link input with explicit fingerprint/petname confirmation, friend management, encrypted DM list/threads/reactions and encrypted persistence after process restart. Inbound links/QRs create only unconfirmed proposals. Trust chrome derives from successful core verification and the full pinned Ed25519/X25519 tuple, never nicknames or claimed sender IDs. Preserve confidentiality/metadata disclosures, honest queue/delivery-unknown states, and no plaintext fallback. Presence follows the existing session-bound 60-second freshness rule and last-authenticated-response age; RSSI is not identity or distance proof.

Align the ticket's broad "Key changes block sending until re-pairing" wording with normative design section 7.4/MC-008: **a network claim cannot replace a pin or disable the existing friend**. Only a user-selected replacement flow suspends the selected old context and requires fresh QR/explicit old-versus-new tuple confirmation before sending to the new identity. Preserve history under the old identity, invalidate stale queued sends, and never silently merge conversations. This aligns the ticket with the approved design, without changing security policy or permitting automatic replacement.

### Acceptance before completion

Require actual core/transport traces for encrypted send/receive/reactions, invalid-before-valid same-ID intake, authenticated replay, unknown/missing keys, removal/replacement during queued sends, denied camera, malformed links and explicit confirmation, protected lock/reset, and restart with retained pins/history. Exercise synthetic QR decoding/scanning and actual friend-link entry. Unsigned/copycat nicknames must never gain verified UI chrome. Run generated Kotlin and Swift consumers after API changes, Android Debug/Release/lint/package alignment and synthetic emulator UI/storage checks, Rust security regressions and required dependency checks. Use the approved local-validation policy, separate Terra medium final review, and squash merge. Emulator checks do not certify physical camera/radio or hardware protection; integrated physical and beta gates MC-025/043/034 and separately required independent security assessments remain mandatory.

Do not begin production implementation until this extension is approved. No new product feature, protocol revision, machine setting or deployment authorization is included.
