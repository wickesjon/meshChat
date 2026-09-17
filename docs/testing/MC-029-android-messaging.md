# MC-029 Android friend and encrypted-message integration

Status: required checks and Terra follow-up review pass. Completion is effective on the squash merge of [PR #31](https://github.com/wickesjon/meshChat/pull/31). No physical certification is claimed.

## Implemented boundary

`NativeTransport` owns the live `Friends`, `Dms`, ingress budget and relay scheduler. Its child `native_messaging.rs` exposes typed application operations using temporary protected store/provider sessions. Incoming ciphertext must match an existing pending record and its ingress generation before real HPKE authentication can run. Signed content uses the existing friend verifier. Four pending slots are revisited per retry call, with the same work budgets and original deadlines. The unsigned channel entry point still rejects signed/encrypted records.

Confirmation reparses the exact displayed URI, validates both keys through the existing friend owner and persists a full-tuple/revision pin. Proposals, network names and radio observations cannot mutate pins. The displayed grouped fingerprint is the full SHA-256 digest of the 64-byte Ed25519/X25519 public tuple; it is not the eight-byte routing hint. Replacement requires an explicit user action, suspension of the old context, a fresh scanner result and old/new fingerprint confirmation. Removing/replacing stops and drains the radio before changing pins.

Outgoing DMs are encrypted and persisted by `Dms` before enqueue. Every protected fragment carries an egress guard checked under the Android radio monitor and the provider's global reset lock immediately before native submission. The provider uses that single lock order throughout; redundant per-instance monitors are removed to avoid inversion when storage has entered the global lock first. Feature errors retain their category while temporary sessions still close in `finally`. No database, passphrase or private-key session enters the UI.

ANNOUNCE is signed. Private-channel CHAT is signed when at least one friend is pinned. A broadcast uses one immutable signed representation on all links, including the full key if any link needs it; native completion advances first-key transmission state. This avoids producing different replay digests for the same message on different paths. Stored verified channel content displays the local petname only for a unique active matching pin. Unknown valid signers remain distinct from friends; unsigned lookalikes show a warning. Presence comes from the existing session-bound proof and expires at the 60-second nonce-age boundary or disconnect. A signed avatar does not refresh presence.

History selection and count eviction now use existing local insertion IDs. Sender timestamps still govern acceptance/age expiry, and replay tombstones, schema and cryptography are unchanged. Old DM history remains bound to its original full tuple. A separate encrypted navigation index holds at most 64 old identities (eight settings records, eight entries each); if full, a further remove/replace requires explicit deletion of an old conversation first. The original history/replay bounds still apply. Deleting an old conversation retains replay protection.

## Android behavior

The Friends tab includes a public QR code, offline scanner, validated paste flow, local petnames, full fingerprint comparison, authenticated-response age and explicit management actions. Messages opens active or archived full-key threads, encrypted reactions and honest queued/native-handoff/delivery-unknown states. There is no plaintext DM fallback. Draft text stays visible when submission is refused.

QR generation and decoding use pinned ZXing Android Embedded 4.3.0 and ZXing core 3.4.1 under Apache-2.0. The [upstream integration guide](https://github.com/journeyapps/zxing-android-embedded/blob/v4.3.0/README.md) documents the offline scanner and activity-result integration. Camera permission is requested only from the scanner action; denial leaves explicit link entry available. Barcode-image persistence and scan beeps are disabled. HTTPS input is parsed, but domain association is explicitly not verified here; installation/domain deployment belongs to MC-031.

## Validation record (in progress)

- Windows Rust 1.85.1: all 185 debug and 185 release checks pass, including eight new native messaging traces and three storage-order regressions. Strict formatting/clippy pass.
- Fourteen Python/UniFFI storage-policy checks pass, including shared public history ordering and unchanged timestamp-based expiry. Their SQLite double is not encryption evidence.
- Android normal Kotlin/Swift binding generation and both pinned-NDK Android libraries build. Initial Android Debug/Release/test APK, lint and JVM checks pass. The final provider-lock revision passes Debug/Release/test APK, lint and all ten app JVM tests; the BLE probe passes both builds, lint and 26 driver JVM tests.
- Rust messaging traces cover inert/malformed proposals, persistent full-key pins, encrypted chat/reactions, tampered-before-valid same-ID intake, replay/restart, unknown keys, omitted-signing-key recovery, pinned petname versus unsigned copycat, queued removal, replacement/history separation and proof freshness.
- New Kotlin tests exercise opaque friend handles, error mapping, offline QR decoding and protected session cleanup. The new Swift consumer trace passes on the Mac runner.
- Emulator acceptance uses `run_emulator.py` first to create the isolated synthetic profile, then `run_friends_emulator.py` for pair/reopen/replace phases. The latter exercises actual ACTION_VIEW confirmation, synthetic QR decoding, camera denial, protected SQLCipher/provider exchange with a synthetic peer, restart and old-identity history. All three phases pass locally, and the five screenshots have been inspected. Initial fixture failures were corrected: dialog-aware screenshot capture, ActivityScenario intent tracking, lazy-list scrolling, and awaiting asynchronous Back navigation. After the review fix, the full app Debug/Release/test APK, lint and ten JVM checks pass again, as do all six channel/friend emulator phases. The replacement phase fills all 64 encrypted archive entries and checks that both replacement and removal refusals retain the pin and connection generation; it then clears the synthetic entries and completes replacement/removal.

The existing MC-005 emulator phases (create, reopen, key-loss) and MC-018 SQLCipher phases (create, reopen, checks, key-loss, reset) pass against the updated native libraries and protected adapter. MC-017 JVM provisioning/reopen/sign/agree/lock/key-loss/reset and seven injected recovery faults pass. Cargo-deny 0.20.2 passes advisories, bans, licenses and sources; only baseline unused license allowances warn. Both app and security Debug/Release APKs pass both-ABI ELF LOAD/RELRO 16 KiB checks; app APKs pass `zipalign -c -P 16 4`. Ticketboard validation (46 tickets/131 dependencies), 12 board unit tests and `git diff --check` pass.

Host: Windows, Rust 1.85.1, Python 3.14, JDK 17.0.15+6, Gradle 8.13, Kotlin 2.2.0, AGP 8.11.1, compile/target SDK 36, build tools 35.0.0, NDK 27.3.13750724. Synthetic emulator: API 29 default x86_64 revision 8, emulator 37.1.11 (15917651), adb 37.0.1 (15733141), 4 KiB runtime pages; ELF/ZIP alignment checks do not claim a 16 KiB runtime test. The pinned rebuilt SQLCipher 4.17.0 and graphics-path 1.0.1 artifacts from MC-028 are reused unchanged locally; hosted CI rebuilds their sources.

Commands (repository-local caches/output; exact environment setup is in the Windows Android build guide):

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo test --workspace --all-features --locked --release
cargo build --workspace --all-features --locked --release
cargo-deny --all-features --locked --config src/core/deny.toml check
python -B tests/integration/storage/run_policy.py
python -B src/core/build_bindings.py android
src/android/gradlew.bat -p src/android --no-daemon --max-workers=2 :app:assembleDebug :app:assembleRelease :app:assembleDebugAndroidTest :app:lintDebug :app:testDebugUnitTest
src/android/gradlew.bat -p src/android/ble --no-daemon --max-workers=2 assembleDebug assembleRelease lintDebug testDebugUnitTest
python -B src/core/build_bindings.py android --security-probe
python -B tests/integration/identity/run_kotlin.py
src/android/gradlew.bat -p src/android/security --no-daemon --max-workers=2 assembleDebug assembleRelease assembleDebugAndroidTest lintDebug
python -B tests/bench/security/run_android_emulator.py --serial emulator-5580
python -B tests/integration/storage/run_android.py --serial emulator-5580
python -B tests/integration/android-ui/run_emulator.py --serial emulator-5580
python -B tests/integration/android-ui/run_friends_emulator.py --serial emulator-5580
```

Local app artifact SHA-256:

- Debug: `ffd25ea5d974709bcd30af2c56e95e3284df29f4f0819a63a6111cb966c7d17e`.
- Release unsigned: `d11140c7b4b3bdb1ab2ac0a0ed90cc94dc309896303857b46c9ac95109ae1fd1`.

Ignored logs and screenshots live under `.work/mc029/`. The entire iOS job passes in [hosted run 35194417611](https://github.com/wickesjon/meshChat/actions/runs/35194417611/job/105114154530) on source `545158ea3859732d3889189e1159a151887bf934`: generated Swift host FFI including the new messaging trace, skeleton Debug/Release simulator builds, BLE/security probe Debug/Release device/simulator builds, curve/identity checks and SQLCipher lifecycle integration. Host is macOS 15.7.9 arm64, Xcode 16.4 (16F6), Apple Swift 6.1.2. Rust and ticketboard hosted jobs also pass. The follow-up changes affect only Android model/test code and evidence; shared Rust, generated API inputs and Swift consumers are identical to this passing revision. Android checks were rerun locally after the fix under the approved local-validation policy. Hosted Android remains supplemental; this record does not claim it passed. Terra medium independently reviewed that source and found one P2: a full archive refused the pin change but stopped the connection first. The fix reserves archive capacity before stopping, with emulator regression checking unchanged connection generation and pin for replacement/removal refusals. The new refusal regression and all affected Android checks pass. Terra medium follow-up approved `4ed8a3398421c80ecc274817a6cc9f1bffe660f2`: the P2 is resolved, both refusal paths are covered and no new actionable issues were found. The final completion-metadata revision and its review are recorded in PR #31; no formal self-approval is claimed.

## Deferred evidence

These automated results cannot certify real camera optics, Bluetooth range/throughput, OEM/background behavior, battery, hardware key protection, backup/restore or real-device lock behavior. MC-025/027 and MC-043/044 retain their integrated-candidate physical gates; MC-034 and later security/release descendants retain their prerequisites. Independent integrated security assessment remains separate from this ticket's Terra PR review.
