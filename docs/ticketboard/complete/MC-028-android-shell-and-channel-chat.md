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

- [x] Channel join/send/receive/react flows work with the real core and retain history after restart.
- [x] Anonymous posts use disposable identity and neutral avatar; UI never calls plaintext channels encrypted.
- [x] Accessibility, byte-boundary, unsafe-text, disconnected and rate-limited states pass UI checks.
- [x] Relevant checks pass and evidence is recorded. Completion is staged for final Terra review and squash merge; it becomes effective only on main.

## Potential fallbacks

- If radio access is unavailable during UI work, use deterministic core-driven fixtures while keeping hardware validation pending.
- If a message cannot enter the bounded queue, show not-sent/retry rather than a false sent state.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Implemented and tested on the dedicated branch; final completion metadata and test-harness fixes require the final Terra review before squash merge. See [the reproducible Android UI guide](../../testing/android-channel-ui.md) for feature ownership, bounded-state limits and commands.

- Production Compose app reuses the protected identity/SQLCipher/GATT owners. New Rust channel exports reuse frozen codec/text/channel policies, separate unverified display from trust and persist local-arrival history/reactions.
- Eight Rust feature regressions include a real three-node relay, disposable Confessions identities, Unicode/byte limits, rate limits, failed storage retry, and orphan cap/expiry/disconnect behavior. Actual generated Kotlin and Swift traces extend the native FFI gates.
- Emulator acceptance uses explicit synthetic fixtures and the real protected app; actual RF/hardware certification remains MC-025/043. This triggers the documented deterministic-fixture fallback while native checks remain required.
- Compose BOM 2025.05.01 introduces graphics-path 1.0.1. Its unaligned RELRO was detected before packaging; the pinned matching source is rebuilt with both 16 KB flags under the approved UI path, preserving its Java/resources/licenses. Both locally rebuilt ABIs pass ELF alignment; production package checks remain mandatory.
- Local app and emulator-test Kotlin compilation passed with JDK 17.0.15+6/Gradle 8.13/Kotlin 2.2.0. An ignored compile-only init script uses the existing original SQLCipher API jar because aligned SQLCipher is built on Linux. It cannot package or execute tests; this is compilation evidence only, not packaged SQLCipher evidence.
- The following dated validation/review entries supersede the initial compilation-only evidence. No hardware certification is claimed.

## Review and merge

- Branch: `ticket/MC-028-android-shell-and-channel-chat`.
- Review/PR: [#30](https://github.com/wickesjon/meshChat/pull/30).
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


Proposal prepared on dedicated branch ticket/MC-028-android-shell-and-channel-chat from main af7680a49dae969470cf8b1e70fda2d4aea59b7d, after MC-026 PR #29 merged. The user approved this extension; the Scope and depends_on above now include it. All hard dependencies are complete on main. The ticket is in review on its dedicated branch.


### Initial PR review and validation

- Separate `gpt-5.6-terra`/medium worker reviewed `ea3d1329f6ab9d9040c4e8ae3627fdbc900fc940` against main `af7680a49dae969470cf8b1e70fda2d4aea59b7d`. Outcome: one P1, pending signed CHAT could enter ordinary unverified history/display. Fixed by refusing pending work in Kotlin and Rust, rejecting semantic signed/encrypted flags at the unsigned boundary, and filtering them from its history. A corrupted signed golden-vector regression asserts neither pending nor misclassified signed CHAT persists. Relay behavior stays unchanged. Follow-up review passed at `270a0af28cb612cd30bff9199d0caf41938faf47`, as recorded below.
- At the initial PR revision, local Rust debug/release: 174 tests each; formatting and all-target/all-feature Clippy pass. Both native Android ABIs and host bindings pass. 26 actual JVM tests pass. Dependency advisory/license/bans/source gates pass with cargo-deny 0.20.2. Ticketboard/default and 12 unit tests pass.
- Measured nickname palette contrast is at least 7.70:1 on dark surfaces and 6.07:1 on light; primary message text is 14.25:1 dark and 15.65:1 light. Emulator large-text verification remains required.
- Hosted run `35184834984` passed Rust and ticketboard. Android stopped at the source archive checksum before app packaging: the downloaded archive wrapper differed. The fix checksums the entire sorted set of file names and contents before extraction, retaining the pinned release and rejecting changed content. Rebuilt SQLCipher from that run is artifact `10481996081`; its downloaded ZIP SHA-256 is `2944089ee0aad571d5f36d3185a33a51b9248bf14e1a1f176695111047417ce8`. It is used only for local native validation; no failed job is claimed as a pass.


### Reviewed revision validation

Production revision: `270a0af28cb612cd30bff9199d0caf41938faf47`.

- Terra medium follow-up review: **PASS**, no remaining blocking findings. It independently checked the pending-authentication fix, the signed-vector regression and pinned graphics source-content verification. Actual outcome is also recorded in PR #30's body; no formal GitHub self-approval is claimed.
- Local Windows: `cargo test --workspace --all-features --locked --release` passes all 174 tests after the fix; all eight channel regressions and all-target/all-feature Clippy pass. `python -B src/core/build_bindings.py android` builds current host bindings and arm64-v8a/x86_64 using NDK 27.3.13750724.
- Local production app: `:app:assembleDebug :app:assembleRelease :app:assembleDebugAndroidTest :app:lintDebug :app:testDebugUnitTest` passes with JDK 17.0.15+6/Gradle 8.13/Kotlin 2.2.0; all seven app JVM FFI tests pass. Both APKs pass `tests/integration/android-ui/check_apk.py` and build-tools 35.0.0 `zipalign -c -P 16 4`. A transient Windows Gradle cache rename failure was resolved by rerunning unchanged commands, without disabling checks.
- The locally validated Debug APK SHA-256 is `3b24933b9dfb137ad8da8c52a88d32441dd9153456a9dd0a3fb0ce51167516be`; Release unsigned is `22755ace0fa800e66cfcc622faf30cbac0885f98985d0d6f68d3771e6cba8ddb`.
- Hosted run [35185288348](https://github.com/wickesjon/meshChat/actions/runs/35185288348): Rust and ticketboard **PASS**. macOS job `105085990013` **PASS**: actual Swift 6 channel codec/disposable posts/byte limits/persistent reactions, existing transport/driver traces, Xcode 16.4 Debug/Release app and BLE/security device/simulator builds, security interoperability and all five SQLCipher lifecycle phases. The protected macOS test wrapper is synthetic evidence, not hardware certification.
- Android job `105085989936` passed the source rebuild, production build/lint/unit tests, native-package checks and all MC-005/018 emulator checks. Its initial MC-028 UI timeout is recorded below and superseded only by the corrected local UI results; the failed job is not a passing hosted run.

### Emulator investigation

Run `35185288348` completed: Android production and security builds/lint/package checks passed, as did all MC-005 and MC-018 emulator lifecycle phases. MC-028's create phase timed out at its 20-second wait for the channel list after profile creation; no UI pass or hardware pass is claimed. The runtime cause remains under investigation.

Terra medium independently reviewed `001655a5e7052b6b25142b5db6fcdfae91d813aa`: PASS, no blocking findings. That revision waits for the asynchronous theme write, retains synthetic screenshots on assertion failures, and adds unbuffered CI phase output. Follow-up diagnostics allow up to 120 seconds per protected-state assertion and 600 seconds per complete emulator phase, failing immediately if the protected app reports unavailable. Software-emulated SQLCipher retains its production key-derivation cost; these functional deadlines are not device latency evidence. All existing functional assertions remain required.


### Local emulator acceptance and final review scope

- Terra medium reviewed `19c702d098cbb1a7613b868d4356b503a50db583`: **PASS**, no blocking findings. The bounded software-emulation deadlines retain all functional assertions and immediately reject a protected-data failure. Production source is unchanged from reviewed `270a0af`.
- Local Windows acceleration was already available (WHPX); no host settings were changed. An isolated, hidden `emulator-5580` uses Android Emulator 37.1.11.0 build 15917651, adb 37.0.1-15733141, API 29 default x86_64 revision 8, and repository-local AVD/temp data. Official image ZIP SHA-256: `b5c3fda1f4b4931c30518d342e4ad5f7464945e0cdced3538d4ff2e12f7bf201`; its published SHA-1 was verified before extraction.
- Local reproduction identified asynchronous navigation assertions that ran before the protected channel read finished, plus a runner expecting the raw Android completion code without requesting raw output. The final test waits for the destination screen, uses `am instrument -w -r`, and refuses non-PNG output when collecting screenshots. The production behavior and acceptance assertions remain unchanged.
- `python -B tests/integration/android-ui/run_emulator.py --serial emulator-5580` passes all three actual production-app phases: create, force-stop/reopen, and 200% font-scale composer. They cover real protected SQLCipher history/reaction persistence, four subscriptions and light-theme persistence, sanitized nickname/plain native HTML/URL text, unverified chrome, offline-send failure, 280-byte boundary, and real-core rate limit/refill. Raw output requires both `OK (1 test)` and completion code `-1`; assertion failures remain fatal.
- All four synthetic PNGs were visually inspected: dark chat, light channel list, reopened light chat, and large-text composer. Text and trust indicators remain legible and separate; HTML/URL render literally; the 284-byte draft disables Send. Screenshot files/logs are ignored `.work/mc028/` evidence, not committed personal data. No RF or hardware-protection claim is made.
- Local-policy applicability: only test/CI logging/documentation changed after production `270a0af`. Its passing Rust, Kotlin, Swift, Android/iOS build/lint/package and MC-005/018 security checks remain applicable. The final changed test APK was rebuilt and run against that exact production app locally. The newer hosted run is supplemental; it is not required to replace successful applicable local validation. Final Terra review of the completion revision is still required and will be recorded in PR #30 before merge.

Final local fixture rerun passed in 12.695 s (create), 5.004 s (reopen), and 5.130 s (composer). Exactly the four required PNGs were retained; missing-file adb text is excluded. Production Debug APK remains SHA-256 `3b24933b9dfb137ad8da8c52a88d32441dd9153456a9dd0a3fb0ce51167516be`; final test APK is `596d3cfb0023f00a91c9f3b2b439ab14a58a9f5b16449831495861ccc8f28c45`. Final changed-test compilation/packaging and `:app:lintDebug` pass. Ticketboard regenerated/default validation, all 12 ticketboard tests, and `git diff --check` pass. The final reviewer should compare this completion revision against main and record the exact revision/outcome in PR #30 before merge.
