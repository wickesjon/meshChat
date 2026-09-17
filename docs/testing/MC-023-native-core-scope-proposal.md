# MC-023 native transport integration scope proposal

Status: **approved by the user on 2026-09-16**. The user approved the scoped core bridge and shared binding tests below. MC-022's separate construction assessment is underway; this approval does not waive that gate.

## Reproducible gap

MC-023 starts from main `835a7966f4edb2dead99f5768ec24b75059a337c`; MC-004 and MC-016 are complete. The ticket permits `src/android/ble/**`, `tests/bench/android/**`, `docs/testing/**` and its own board records.

The actual generated Kotlin `CoreInterface` exposes `connected`, `handleEvent` and `prepareSend`. In `src/core/src/lib.rs`, `DriverEvent::InboundBytes` validates the value length and returns an `InboundObserved` byte count; its documentation explicitly says bytes have not been parsed or authenticated. `prepare_send` validates and returns a native send command. Neither operation owns the implemented handshake, framing/reassembly or ingress state.

The production implementation exists in `framing.rs`, `ingress.rs` and `friends.rs`/`friends/proof.rs`. `Friends::start_link`, `hello_transmitted`, `receive`, proof handling and `disconnect` are Rust APIs without UniFFI exports. A Kotlin driver cannot call them through the existing bindings. MC-016's transport assumptions explicitly assign native HELLO/driver integration to MC-023/024/026, and MC-023 requires real-core exchange with no duplicate native parsing.

Reproduce with:

```text
rg -n 'pub fn (connected|handle_event|prepare_send)' src/core/src/lib.rs
rg -n 'InboundObserved|bytes have not been parsed' src/core/src/lib.rs
rg -n 'pub fn (start_link|hello_transmitted|local_proof|disconnect)' src/core/src/friends/proof.rs
rg -n 'uniffi::export' src/core/src/friends.rs src/core/src/friends/proof.rs src/core/src/framing.rs src/core/src/ingress.rs
```

The final search finds no exports. The MC-022 fixture facade is test-only and is on an unmerged branch; it is not a shipping transport API or a substitute for this boundary. A native byte-count sink or handwritten Kotlin HELLO/parser would not establish MC-023's real-core integration criteria.

## Approved narrow extension

Keep MC-023 as the implementation owner and add these permitted paths solely for its transport boundary:

- `src/core/src/native_transport.rs`: a production UniFFI owner around existing core handshake, ingress, frame and link-lifecycle state.
- `src/core/src/lib.rs`: register the new module; retain existing foundation API compatibility.
- `src/core/Cargo.toml`: register focused integration tests if required. No production dependency changes or root lockfile changes are proposed.
- `tests/integration/transport/**` and `tests/integration/ffi/**`: production-owner and Kotlin/Swift binding regressions using synthetic data.

Add MC-019 as a hard dependency because the native boundary uses its completed handshake/proof implementation and existing protected identity/storage handles. Its dependency closure already includes protected identity/storage; all are complete on main. MC-022's independent assessment and physical gates remain unchanged.

The Android implementation and callback tests stay in the existing permitted BLE/bench paths. Native build settings there can consume the same generated bindings and pinned JNI libraries already built for the Android app. Existing CI entry points must be sufficient; if an additional out-of-scope workflow or platform source change proves necessary, obtain a separate scope decision before editing it.

## Boundary and behavior to implement

The new core owner should accept existing protected identity/store handles, explicit limits and clocks. It should expose bounded operations for a native-ready connection (physical role and separately measured transmit/receive capacities), inbound raw GATT values, outgoing framed objects, native submission/completion/failure, time advancement, capacity changes and disconnect. Returned effects identify generation-scoped links, one-frame-per-value sends, readiness and failures. Exact type signatures are implementation details to review in the PR, not a new wire format.

Rust must retain HELLO encoding/validation, fresh nonces, admission, proof handling, frame validation/reassembly and protocol budgets. Kotlin owns scanner/advertiser, GATT characteristics/descriptors, runtime capacities, serialized native operations, readiness/backpressure, bounded queues, callbacks and foreground-service lifecycle. A native successful submission/completion is not a peer delivery receipt or authenticated presence.

Both native directions must have successful runtime capacity evidence and meet the existing 146-byte floor; never infer the result from a requested MTU or INFO ceiling. Any capacity change invalidates pending transfers/admission. Keep object fragments contiguous, no faster than the existing one-frame/second rule, with bounded pending work and existing absolute deadlines. Permission loss, disabled radio, stale callbacks and disconnect discard the old generation's work. Native code must not parse or grant trust to untrusted protocol bytes.

## Required acceptance evidence

1. Real Rust handshake/admission and whole/fragmented exchange through the native boundary in both roles; unknown/below-floor/asymmetric capacities and stale generations refuse correctly.
2. Controlled Android callbacks exercise CCCD success/failure, busy/refused submission, callback errors/timeouts, MTU changes, disconnect and permission loss without duplicate, interleaved, oversized or unbounded sends.
3. Relevant Rust format/clippy/debug/release/build/security checks and Android build/lint/JVM checks. The shared exported API also requires the existing Swift/Mac binding regression/native compilation checks; Windows alone cannot satisfy that change.
4. Ready PR, Terra medium review, correction/re-review and squash merge only after all applicable gates pass. MC-025 retains real Android radio acceptance; MC-022 retains the independent cryptographic assessment.

This proposal adds integration plumbing and tests. It does not authorize a protocol change, weaker security, new algorithm/dependency, claim of physical support, full-wire freeze, app distribution or store submission.

## Platform references checked

Android's [GATT reference](https://developer.android.com/reference/android/bluetooth/BluetoothGatt) requires using the result callback for negotiated MTU; Android 14's requested 517 is not a guaranteed payload capacity. The [GATT server callback reference](https://developer.android.com/reference/android/bluetooth/BluetoothGattServerCallback) supplies separate server-side MTU and notification-completion events. The [background BLE guidance](https://developer.android.com/develop/connectivity/bluetooth/ble/background) retains process/service restrictions. These documented APIs guide implementation; physical radio/lifecycle results remain unmeasured.
