# MC-025 missing MTU callback — 2026-09-26

## Problem and source investigation

Continue the [reconnect investigation](MC-025-reconnect-results-2026-09-26.md) from `aba6a2e` on PR #39. Both devices remain Galaxy S24 Ultra SM-S928U1 and SM-X510, Android 16/API 36, existing synthetic profiles, USB powered. Previous J01/J04 exposed a tablet peripheral stuck at unknown capacity although its client received MTU 517. The opposite phone client timed out and both roles lost their shared connection. Neither launch-order workarounds nor the earlier backoff tests establish general startup reliability.

Pinned AOSP Android 16 Bluetooth revision `4b73ee6039271ffbf71ebdc8c109fc98eac8e137` supplies a concrete explanation. The [client response handler](https://android.googlesource.com/platform/packages/modules/Bluetooth/+/4b73ee6039271ffbf71ebdc8c109fc98eac8e137/system/stack/gatt/gatt_cl.cc#1100) records the negotiated ATT size in the connection control block and completes the client operation. The [server request handler](https://android.googlesource.com/platform/packages/modules/Bluetooth/+/4b73ee6039271ffbf71ebdc8c109fc98eac8e137/system/stack/gatt/gatt_sr.cc#855) updates that same property but emits server callbacks only on the incoming-request path. The [request logic](https://android.googlesource.com/platform/packages/modules/Bluetooth/+/4b73ee6039271ffbf71ebdc8c109fc98eac8e137/system/stack/gatt/gatt_api.cc#778) finds an existing measurement by remote address and LE transport. Thus a client-observed ATT size can be real even when the local server callback was absent. This is upstream implementation evidence consistent with the Samsung trace, not an audit of Samsung's complete vendor stack.

## Narrow repair

At the client's one MTU request, capture the already admitted same-device peripheral token and current local server epoch. Consume that association on the next MTU callback from the exact live BluetoothGatt object. Only a successful, in-range observed value can reach the captured token, and only while that token/address and server epoch remain unchanged. No target is created later, no size is cached for reconnection, and a second request cannot rebind an older response to a newer peer. Requested MTU 517 is never substituted for observed MTU.

The native driver still applies its 146-byte floor, 512-byte cap, conflicting-size teardown, independent CCCD readiness and HELLO/proof checks. Existing server callbacks remain supported. This changes adapter observation routing, not wire format, admission budgets, retry timing, lock/storage handling or authentication. No measured size is inferred across retired tokens, different addresses or server/client generations.

`GattMtuObservationTest` uses a real Android adapter/native admission with synthetic callback addresses and injected observations; it is not an over-the-air MTU test. Each method runs in its own instrumentation invocation with the other device's radio disabled, preserving the existing fresh-process admission precondition without refunding counters. On unchanged Debug APK `d32ccafe916f8efe9ba6456c4206569277bb3956cbeed5aee46c252df8c5c8e3`, the positive regression fails in 3.266 seconds at expected capacity 512 / actual 0. Raw evidence: `.work/mc025/2026-09-26-mtu/baseline-B.log`. Profile and identity were not reset.

## Candidate and local validation

Candidate SHA-256 identities:

| Artifact | SHA-256 |
|---|---|
| Debug app | `5fd6ec9ade1d28353cf4399c3e42b2e932609166d34bac7645c3c07954985dad` |
| Unsigned Release app | `67b8e8f264b080be90ce8bd4239374e69c13e48d71901b6750cee8159dbc5cf4` |
| Final instrumentation APK | `26db887cf943b0c7131ecff1c7615fdce43eb7d75cbba8cf7ee67065c7942d53` |

Using repository-local JDK 17.0.15+6, Gradle 8.13, AGP 8.11.1, Kotlin 2.2.0 and SDK 36, the app Debug/Release/androidTest builds, lint and 26 JVM tests pass (2m7s). Standalone BLE lint, builds, 29 JVM tests and probe APK verification pass (1m21s). After adding the final late-peripheral regression, androidTest build/lint passes again (30s). App Debug/Release `tests/integration/android-ui/check_apk.py` native-set/ELF checks and `zipalign -c -P 16 4` pass. Ticketboard validation and all 12 ticketboard tests pass. Logs are `.work/mc025/mtu-app-build.log`, `mtu-ble-build.log`, `mtu-test-final-build.log` and `.work/mc025/2026-09-26-mtu/`.

This delta changes Android callback routing and tests only: no Rust, FFI, cryptography, protected-storage implementation, wire format, dependency or iOS consumer change. Those components' prior applicable checks remain recorded in the ticket; they are not new passes for this delta. Independent security and other platform/hardware gates remain open.

## Tablet callback regressions

The unchanged-app positive regression failed as recorded above. With the repaired app, the same installed test passes in 3.570 seconds. The final ten-case test package then passes **10/10**, one fresh instrumentation invocation per method, with the phone adapter disabled:

| Case | Result | Runner seconds |
|---|---|---:|
| Current requested observation reaches existing peripheral; CCCD still required | PASS | 3.150 |
| Unsolicited observation cannot supply peripheral capacity | PASS | 3.016 |
| Failed observation cannot supply peripheral capacity | PASS | 2.967 |
| Below-floor observation still refuses peripheral | PASS | 2.891 |
| Different address cannot supply peripheral capacity | PASS | 2.953 |
| Retired client cannot supply peripheral capacity | PASS | 2.890 |
| Retired peripheral cannot receive observation | PASS | 2.959 |
| Replaced server cannot receive old request observation | PASS | 2.877 |
| Second request cannot rebind old observation | PASS | 2.918 |
| Later peripheral cannot receive earlier observation | PASS | 2.889 |

Raw logs are `callback-B-<method>.log` under the current evidence directory. The 500 ms fixture registration wait allows Android's asynchronous client registration; the test requires actual request dispatch acceptance, not a fabricated dispatch result. The callback payload itself is deliberately injected. These results verify routing/lifecycle guards, not real two-device negotiation or reconnect reliability.

Existing tablet retirement (3 cases), refusal (1) and Bluetooth off/on recovery (1) regressions also pass **5/5**; logs are `existing-B-{retirement,refusal,bluetooth}.log`. Installed app and final test APK hashes were read back and matched to the candidate (`installed-tablet.json`).

## Outstanding validation

Phone callbacks and repeated paired startup/message/reconnect trials are pending the phone being unlocked. The paired candidate has not yet been measured; previous startup failures remain unresolved physical observations until retested. Do not transfer the earlier ready-link latency or recovery results to this APK. USB-powered tests provide no battery-drain result. MC-025 stays in progress and PR #39 remains unmerged pending its broader physical/platform/security gates and the independent review of this delta.
