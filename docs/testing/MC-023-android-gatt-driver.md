# Android GATT driver integration

MC-023 implements transport in `src/android/ble/src/main/java/org/meshchat/transport/`. The original `bleprobe` package remains the MC-004 synthetic measurement application. These are separate surfaces: the probe's test UUIDs are not the shipping service contract.

## Ownership and application entry

`MeshTransportService.start` accepts a uniquely owned `NativeTransport` backed by the existing protected identity/store, an event sink, and a callback receiving the radio handle. Start it from a visible user action after the approved permission grants. Ownership transfers only when start returns true. A process restart has no retained session and returns `START_NOT_STICKY`; application feature integration must reopen the protected owner explicitly. The service closes its core on destruction. Never share that owned object with another active service/core generation.

`AndroidGattRadio.send` returns whether the trusted owner's object was accepted for scheduling; `TransportEvent.Finished` reports its terminal outcome. Neither a successful enqueue nor a native completion proves peer receipt. Supply only content already authorized by its friend/DM/organizer/SYNC owner. Received events are structural `Unverified`, `Opaque`, `Pending`, `Duplicate` or `DeferredSync`; they do not grant display or trust authority. This transport does not wire the later product UI or SYNC/crypto feature dispatch.

The proof call accepts a freshly unlocked provider session for the one operation and invalidates it on return, including failure or a stopped radio. It returns false on refusal; the caller may retry a work-budget refusal before the unchanged core deadline. Native key-loading/QR work accounting and full provider lifecycle integration remain the explicit MC-022 A-02 obligation for the feature owners. No private provider session is retained by the transport. Runtime ticks use Android's suspend-inclusive `elapsedRealtime`; a regressing core clock fails closed.

## Connection and callback behavior

The production service/characteristic UUIDs and INFO bytes `01 02 00` come from MC-006. Advertising contains only the service UUID. The scanner uses that UUID filter; all native connections first reserve the core's bounded address/node admission credit. Normal Android is six total connections, including setup. Dynamic power/link policy and authenticated duplicate-link selection remain MC-024; UI/identity bootstrap remains its native feature owners.

A central discovers the required service, requests MTU, then enables local notifications and writes CCCD. Both roles use successful native MTU evidence, with each local direction capped at `min(MTU - 3, 512)`. A requested 517 and the INFO maximum are never treated as measurement. Values below 146 refuse the link. Native readiness requires both that capacity and enabled notification subscription. A server commits CCCD state only after Android accepts its response submission; refusal closes the connection before HELLO. HELLO advertises the measured capacities, while the current scheduler conservatively encodes every outgoing frame at the supported 146-byte floor. Peer HELLO may further reduce each effective direction. Rust alone owns HELLO/proof encoding, parsing, framing/reassembly and budgets.

Established receive callbacks cross FFI synchronously under one monitor, avoiding an unbounded Handler queue. Up to two bounded startup values per link (twelve total at six links) bridge the race where a notification arrives before the CCCD completion callback. Oversized values are not copied; their reported length is submitted to Rust for frame/byte charging. Startup overflow drops newest before copying with a saturating counter. The core charges staged values when native readiness registers their link; nothing is parsed or displayed before then. The driver retains at most one scheduled outbound value per link. A shared notification queue reserves no more than six frames/3072 payload bytes, including the in-flight frame, and waits for the actual callback before submitting the next notification on any peer. The already reserved core send token/deadline covers queued native work; cancelled queued attempts receive no budget refund.

Core pacing, contiguous per-object fragments, one refusal retry and absolute object deadlines remain unchanged. A missing native completion after five seconds closes the generation instead of retrying an ambiguously completed write. Native setup expires after thirty seconds; core HELLO has its separate ten-second deadline after readiness. Subscription disable/failure, changed capacity, permission loss, disabled radio or a locked device close affected work. Screen/background transitions alone do not certify continued radio operation. Supported background policy and measured OEM behavior remain MC-024/025.

Android's server callbacks have an address but no connection-generation token. Any admitted peripheral teardown therefore closes and replaces the whole GATT server, invalidates its callback epoch, cancels all that server's peripheral work, and resumes advertising if still running. Central links have individual GATT object identity guards. Refused peripheral addresses are retired for the server epoch; after six such refusals further admission is refused until server replacement. With no admitted peripherals, a refused connection immediately replaces the server. These conservative disconnects preserve callback attribution and are not a seamless multi-peer reconnection guarantee. MC-024 owns reconnection policy.

## Validation and limits

`tests/integration/transport/native.rs` exercises the real core in both roles, whole/fragmented traffic, proof, capacity boundaries, stale completion, timeout, connection limits and clocks. The Kotlin/Swift FFI suites exercise the exported production bridge. `tests/bench/android/kotlin` drives the same callback state owner used by Android, with controlled native outcomes and real Rust parsing/scheduling; it also tests the production notification queue across peers, refusal, stale callbacks and reentrant teardown. These tests use synthetic plaintext SQL callbacks, not a replacement production store or a SQLCipher claim. Android framework wiring receives native compilation/lint and peer review, not a claim of emulated Bluetooth radios.

Reproduce with the pinned repository-local toolchains:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo test --workspace --all-features --locked --release
python -B src/core/build_bindings.py android
src/android/gradlew -p src/android/ble --no-daemon assembleDebug assembleRelease lintDebug
```

On Windows use `gradlew.bat`, local Rust/Cargo/Gradle/temp homes, JDK 17, and the installed NDK 27.3.13750724 as documented in the Windows build guide. The BLE lint task depends on all JVM tests, preserving the existing CI entry point. Its production dependencies reuse the existing pinned JNA/annotation versions; no new library version, Rust dependency or wire format is introduced. Mac CI executes the extended Swift host regression and builds the shared libraries for device/simulator; those native checks remain required. The ticket records exact results/revisions.

No physical BLE discovery, range, throughput, OEM background behavior, key protection, cross-phone pairing or battery result is claimed. MC-025 retains physical Android acceptance, MC-027 cross-platform behavior, and MC-043/044 protected-key/storage certification. All traffic here is synthetic until the applicable platform gate permits sensitive-data use.

API behavior was checked against Android's [GATT client reference](https://developer.android.com/reference/android/bluetooth/BluetoothGatt), [server reference](https://developer.android.com/reference/android/bluetooth/BluetoothGattServer), and [server callback reference](https://developer.android.com/reference/android/bluetooth/BluetoothGattServerCallback) on 2026-09-16. They require using negotiated MTU callbacks and waiting for notification completion; physical support remains measured separately.
