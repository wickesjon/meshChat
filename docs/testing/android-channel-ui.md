# Android channel application — MC-028

This is implementation evidence, not physical-device certification or a release
approval. Use synthetic messages until MC-025/043 hardware acceptance completes.

## Owners and boundaries

The production `org.meshchat.app` launcher now uses Compose with the existing
Android protected identity, SQLCipher storage, foreground service and GATT owner.
`MeshModel` serializes feature work on one executor (64 queued operations), with
at most 32 pending sends, 100 transient send receipts and 32 joined channels.
An executor overflow stops the mesh and fails closed. Service callbacks carry a
generation guard; stop/reset invalidates stale callbacks. Database connections,
keys and signing sessions never enter a UI snapshot or outlive a protected
operation. Settings and subscriptions are encrypted records, not preferences.

`NativeChannels` reuses the frozen channel IDs/glyphs, Unicode sanitizer, wire
codec and encrypted-history owner. It composes unsigned plaintext CHAT/reactions;
it grants no trust. The stable sender is replaced with fresh random bytes for
every Confessions post, with fixed Anonymous nickname and neutral mask. Reactions
retain the stable identifier and the UI discloses that distinction. Origin
buckets cover General/Confessions together, Event Updates, private channels,
aggregate CHAT and reactions. No wire format or existing security budget changes.

Budget-admitted transport events drive application delivery and forwarding.
Unsubscribed channels still relay; encrypted/control/SYNC data cannot become a
plaintext chat row. Existing transport dedup, TTL, copy suppression and egress
limits remain authoritative. Origin queues report refusal and native completion
separately; neither is a delivered receipt. A stopped send cannot remain queued.

The UI displays the most recent 100 history events per channel using the existing
bounded history API. Reactions in that window use per-sender replacement/removal,
saturating counts and an unknown-code fallback. Orphans are capped at 32/link and
256/node, expire after 120 seconds and disappear on disconnect. History uses
local arrival timestamps (clamped against backward wall-clock changes), with the
existing insertion tie-break; claimed peer time never orders the UI. Unread
counts are bounded session-local hints. Durable previews are reconstructed.

Onboarding, public and word-triple channel navigation, avatars, glyphs, plain
native text, byte-aware composer, reactions, mute/leave/delete-history,
Afterhours/light themes, power selection and explicit identity reset are wired.
Messages/Friends are entry states for MC-029; organizer trust is MC-030, sharing
and QR are MC-031, and later power/Beacon/Supporter screens retain their tickets.
Full integrated SYNC, scale and physical behavior remain later integration gates;
these UI tests do not certify them.

## Reproduce

Use repository-local caches/output as in `.github/workflows/ci.yml` and the
local-validation policy. Pinned tools: Rust 1.85.1, Kotlin/Compose compiler 2.2.0,
JDK 17.0.15+6, Gradle 8.13, Android SDK 36/build-tools 35.0.0, NDK 27.3.13750724.

1. Run Rust formatting, all-target/all-feature Clippy with warnings denied, and
   workspace debug/release tests/build with `--locked`.
2. Run `python -B src/core/build_bindings.py android` for actual host and both
   Android ABIs. Run the existing Swift 6 warnings-as-errors host harness and
   Xcode 16.4 platform builds on macOS, including the new channel trace.
3. Build the pinned SQLCipher AAR with
   `python -B src/android/security/build_sqlcipher.py` on Linux. Build graphics
   with `python -B src/android/ui/build_graphics.py` on Linux or Windows.
4. Run the production Android app Debug/Release, `assembleDebugAndroidTest`,
   `lintDebug`, `testDebugUnitTest`; run the existing BLE/security checks.
5. Run `tests/integration/android-ui/check_apk.py` on both production APKs, and
   `zipalign -c -P 16 4`. Require exactly Rust/JNA/SQLCipher/graphics JNI on both
   arm64-v8a and x86_64, including LOAD and RELRO alignment.
6. Start the isolated existing API-29 x86_64 security emulator. Run its security
   and storage lifecycle checks, then
   `python -B tests/integration/android-ui/run_emulator.py --serial emulator-5554`.
   This explicitly refuses a physical-device serial and clears only the
   synthetic production app on that emulator. Three phases cover onboarding and
   offline send, encrypted history across process restart, and real-core rate
   countdown/UTF-8 boundaries with 200% font scaling. Screenshots contain only
   synthetic fixture text and live under `.work/mc028/screenshots/`.
7. Run ticketboard generation/default validator, its unit suite and diff checks.
   A separate Terra review and all relevant successful checks are merge gates.

Rust tests exercise three real transport nodes: A sends through unsubscribed B
to C, C reacts through B to A, TTL decrements once, and C's stored view survives
reopen. The Rust/JVM/Swift database doubles are explicitly not encryption
evidence. Emulator tests use the actual protected SQLCipher adapter. No emulator
test claims RF propagation, hardware key isolation or real-device certification.

## Native graphics dependency

The new UI pins Compose BOM 2025.05.01 and activity-compose 1.10.1. The Compose
compiler follows the Kotlin 2.2.0 plugin version, per [Android's setup guide](https://developer.android.com/develop/ui/compose/setup-compose-dependencies-and-compiler).
The BOM is documented in [Android's May 2025 release announcement](https://android-developers.googleblog.com/2025/05/whats-new-in-jetpack-compose.html).

The transitive graphics-path 1.0.1 AAR's native RELRO ends are not 16 KB aligned.
The 1.1.0 AAR was inspected and has the same issue, so no dependency upgrade or
gate relaxation is used. `build_graphics.py` pins/checksums the 1.0.1 AAR and its
matching [release source](https://android.googlesource.com/platform/frameworks/support/+/8a05a22af450d589ef911d772a001a49dcb05b71/graphics/graphics-path/src/main/cpp/).
It keeps Java/resources/licenses unchanged and rebuilds only the two supported
JNI ABIs with static libc++, the upstream export map and both 16 KB linker flags.
Every resulting ELF is checked before the aligned AAR is emitted. Production
packaging refuses absent rebuilt dependencies. Hosted evidence retains only
the rebuilt AARs and synthetic UI screenshots for seven days.

## Current evidence

Implementation is pending final native/package/emulator validation and independent
PR review. The ticket/PR records exact revisions and final results; this guide
does not convert an unavailable or failed check into a pass.
