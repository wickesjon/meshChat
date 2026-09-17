---
id: "MC-028"
title: "Android shell and channel chat"
depends_on: ["MC-018","MC-023","MC-011","MC-024","MC-022"]
kind: "android"
branch: "ticket/MC-028-android-shell-and-channel-chat"
---

# MC-028 — Android shell and channel chat

## Objective

Implement onboarding, Channels/Messages/Friends navigation, Afterhours/light tokens and permission/mesh states.

## Dependencies

`MC-018`, `MC-023`, `MC-011`, `MC-024`, `MC-022` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/android/ui/**`, `tests/integration/android-ui/**`, `src/android/app/**`, `src/android/build.gradle.kts`, `src/android/settings.gradle.kts`, `src/android/security/src/main/java/org/meshchat/storage/EncryptedStorage.kt`, `src/core/src/native_channels.rs`, `src/core/src/lib.rs`, `src/core/src/native_transport.rs`, `src/core/Cargo.toml`, `tests/integration/ffi/**`, `.github/workflows/ci.yml`, `docs/testing/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Implement onboarding, Channels/Messages/Friends navigation, Afterhours/light tokens and permission/mesh states.
- Build public/private channel lists, chat, byte-aware composer, reactions, glyphs and anonymous Confessions behavior.
- Render safe native text with separate trust chrome and local-arrival ordering; show enqueue failure and best-effort send meaning.

## Exit criteria

- [ ] Channel join/send/receive/react flows work with the real core and retain history after restart.
- [ ] Anonymous posts use disposable identity and neutral avatar; UI never calls plaintext channels encrypted.
- [ ] Accessibility, byte-boundary, unsafe-text, disconnected and rate-limited states pass UI checks.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If radio access is unavailable during UI work, use deterministic core-driven fixtures while keeping hardware validation pending.
- If a message cannot enter the bounded queue, show not-sent/retry rather than a false sent state.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Implemented on the dedicated branch; final native/package/emulator checks and independent review are pending. See [the reproducible Android UI guide](../../testing/android-channel-ui.md) for feature ownership, bounded-state limits and commands.

- Production Compose app reuses the protected identity/SQLCipher/GATT owners. New Rust channel exports reuse frozen codec/text/channel policies, separate unverified display from trust and persist local-arrival history/reactions.
- Eight Rust feature regressions include a real three-node relay, disposable Confessions identities, Unicode/byte limits, rate limits, failed storage retry, and orphan cap/expiry/disconnect behavior. Actual generated Kotlin and Swift traces extend the native FFI gates.
- Emulator acceptance uses explicit synthetic fixtures and the real protected app; actual RF/hardware certification remains MC-025/043. This triggers the documented deterministic-fixture fallback while native checks remain required.
- Compose BOM 2025.05.01 introduces graphics-path 1.0.1. Its unaligned RELRO was detected before packaging; the pinned matching source is rebuilt with both 16 KB flags under the approved UI path, preserving its Java/resources/licenses. Both locally rebuilt ABIs pass ELF alignment; production package checks remain mandatory.
- Local app and emulator-test Kotlin compilation passed with JDK 17.0.15+6/Gradle 8.13/Kotlin 2.2.0. An ignored compile-only init script uses the existing original SQLCipher API jar because aligned SQLCipher is built on Linux. It cannot package or execute tests; this is compilation evidence only, not packaged SQLCipher evidence.
- PR and exact final reviewed/validated revisions will be recorded after publishing; no review or hardware pass is claimed yet.

## Review and merge

- Branch: `ticket/MC-028-android-shell-and-channel-chat`.
- Review/PR: pending.
- Squash commit title: `MC-028: Android shell and channel chat`.
- Completion becomes effective only when the reviewed squash commit lands on main.

## Approved application-integration scope extension

Status: approved by the user on 2026-09-16. Implementation begins within the following scope; native and security acceptance remain unchanged.

### Why the current paths block the ticket

The permitted UI and UI-test directories are not included by the Android app build. Its launcher is still the foundation TextView/Core skeleton. The production transport/service and protected identity/storage adapters live in separate BLE/security projects, and the app neither packages their sources/SQLCipher nor declares the radio service and permissions. Rust channel names/glyphs, text validation/sanitization and logical message codecs are public Rust APIs but are not exported through UniFFI. A UI-only implementation would either duplicate security-sensitive rules in Kotlin or fail the required real-core and persistent-history acceptance.

### Approved additional paths and limits

- `src/android/app/**`, `src/android/build.gradle.kts`, `src/android/settings.gradle.kts`: app entry, native UI/source integration, pinned UI dependencies, radio permissions/service, protected-storage packaging and test wiring. Reuse existing production identity/storage/transport sources; exclude probe activities and temporary security-probe bindings.
- `src/android/security/src/main/java/org/meshchat/storage/EncryptedStorage.kt`: narrow protected-operation entry for constructing the transport/application owner with the current identity generation. Preserve operation-scoped database/key access, lock/reset serialization, fail-closed behavior and existing encryption/backup rules. No raw key, session or database escape.
- `src/core/src/native_channels.rs`, `src/core/src/lib.rs`, `src/core/src/native_transport.rs`, `src/core/Cargo.toml`: bounded public-channel application boundary and exports around existing channel/text/codec/store/transport owners, plus module/test registration. Cover channel creation/join, public CHAT/reaction composition and intake, disposable Confessions identifiers, safe presentation and persistent history. Keep wire formats, budgets, trust classifications and existing cryptography unchanged; do not implement DM/organizer/Supporter features assigned to later tickets.
- `tests/integration/ffi/**`, `.github/workflows/ci.yml`, `docs/testing/**`: real-core native-boundary tests, packaged-app/emulator integration and durable evidence. Existing `tests/integration/android-ui/**` owns new feature and UI cases. Any Swift consumer check required by changed exports remains mandatory.

Retain the existing UI/test paths. Add hard dependencies MC-024 (final Android connection/lifecycle owner) and MC-022 (full-wire freeze) alongside current MC-018/023/011. Both added dependencies are already complete on main. No unrelated refactor, dependency upgrade, deployment or protocol/security change is authorized by this proposal.

### Implementation and acceptance

Implement native onboarding/navigation and the specified themes, private word-triple/public channel lists, byte-aware chat composer, reactions, safe text, separate trust chrome, anonymous neutral-avatar behavior and honest permission/mesh/send states. Messages/Friends navigation may lead to their later-ticket entry states; their full features remain MC-029. Public and word-triple channels remain explicitly plaintext. Use the existing protected provider/storage and transport owner; never replace them with a plaintext preference store or simulated transport to satisfy production acceptance.

Require real-core join/send/receive/react traces, encrypted history across process restart, disconnected/queue-full/rate-limited outcomes, duplicate and unsafe-text tests, UTF-8 byte boundaries and Confessions unlinkability/presentation tests. Run Android Debug/Release/lint/package alignment, Kotlin and applicable emulator/UI/security checks, Rust and both native FFI consumers, ticketboard validation and separate Terra review. Physical radio and hardware protection remain MC-025/043; no certification or release claim is added.


Proposal prepared on dedicated branch ticket/MC-028-android-shell-and-channel-chat from main af7680a49dae969470cf8b1e70fda2d4aea59b7d, after MC-026 PR #29 merged. The user approved this extension; the Scope and depends_on above now include it. All hard dependencies are complete on main. The ticket is in progress on its dedicated branch.
