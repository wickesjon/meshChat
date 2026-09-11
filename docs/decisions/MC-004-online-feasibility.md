# MC-004 — Published BLE evidence and feasibility decision

Decision date and source access: 2026-09-11. Evidence state: platform contracts specified; vendor measurements externally reported; probe source implemented, compiled and agent-reviewed. No meshChat physical execution is claimed.

## Approved acceptance replacement

The user explicitly instructed: "let's replace the physical acceptance gate", after discussing online measurements as the replacement for MC-004. MC-004 therefore accepts cited primary platform documentation, attributable published measurements, a conservative development matrix, a permission/RSSI decision, compiled probes and independent agent review. The linked MC-016 requirement consumes this replacement evidence rather than requiring the removed MC-004 hardware report. This is an approved change of acceptance, not a claim that the old gate passed.

Physical phone-to-phone results, battery, throughput and recovery remain unverified. MC-025 and MC-027 retain physical radio acceptance; MC-038 retains scale/battery field testing. MC-005 protected-storage evidence and independent security/release gates are unchanged. MC-006/007 may proceed after MC-004 merges, using explicit assumptions and runtime refusal, without representing vendor rates as meshChat guarantees.

## Published measurements

[Microchip's BLE Throughput report](https://onlinedocs.microchip.com/oxy/GUID-99E91F8E-E9F7-4C2C-B98A-E9662A2ABA50-en-US-11/GUID-63B6423B-1BB1-4742-8669-C50C2F94A50F.html), developer guide revision URL `en-US-11`, tables 5-39/40, reports:

| Phone / OS | Phone to board (Kbytes/sec) | Board to phone (Kbytes/sec) |
|---|---:|---:|
| iPhone 13 Pro / iOS 15.6.1 | 53.11 | 56.89 |
| Samsung S10 / Android 10 | 46 | 65 |
| Google Pixel 2 / Android 10 | 50 | 67 |

Setup: Microchip MBD application as GATT client, WBZ351 board as server, proprietary TRP, write without response, checksum downlink and fixed-pattern uplink. These are selected reference rows, not a statistical distribution. The table does not supply per-row duration, repetitions, raw traces or background state; publication date is unspecified. Its older OS versions and phone-to-board topology cannot establish our current phone-to-phone capacity, discovery, energy or reliability. No numeric throughput acceptance threshold is derived from these values.

[Silicon Labs' interoperability report, version 11.0.0](https://docs.silabs.com/bluetooth/11.0.0/bluetooth-interoperability-testing-report/) reports testing 354 mobile devices against its IC/stack, covering discovery, GATT and other operations. This supports broad ecosystem feasibility only; it does not test our probes or their phone-peripheral roles. We do not infer that every listed device supports every meshChat case.

## Capacity, flow control and recovery contract

- Android: use actual successful MTU callbacks, not the requested value. Android 14+ requests ATT MTU 517 on the first GATT-client request and ignores subsequent requests; this is not evidence of a negotiated 517-byte payload. Refuse oversize values before API submission. Serialize writes and distinguish API refusal from callback outcome and receiver delivery. [BluetoothGatt reference](https://developer.android.com/reference/android/bluetooth/BluetoothGatt#requestMtu(int)).
- iOS writes: obtain the limit for `.withoutResponse` on the actual peripheral. [maximumWriteValueLength](https://developer.apple.com/documentation/corebluetooth/cbperipheral/maximumwritevaluelength(for:)). Stop when write readiness is false and resume via its delegate callback. [canSendWriteWithoutResponse](https://developer.apple.com/documentation/corebluetooth/cbperipheral/cansendwritewithoutresponse).
- iOS notifications: obtain the selected central's separate notification limit. [maximumUpdateValueLength](https://developer.apple.com/documentation/corebluetooth/cbcentral/maximumupdatevaluelength). A full transmit queue returns false; retain the pending value until the ready callback. Oversize notification values may truncate, so preflight them. [updateValue](https://developer.apple.com/documentation/corebluetooth/cbperipheralmanager/updatevalue(_:for:onsubscribedcentrals:)).
- Our decision: no guaranteed minimum negotiated capacity. MC-006 must choose a protocol admission floor and prove packet fit mathematically, with explicit unsupported-capacity refusal. The existing 182-byte example is a design input only. MC-007 must include lower capacities, asymmetric directions, readiness stalls and disconnects in its scenarios; no vendor rate can waive queue or ingress bounds. Disconnect/revocation discards pending link work; fresh connections obtain fresh capacities. Synthetic tests establish those software invariants, not radio performance.

## Development support matrix

Targets remain Android API 29+ and iOS 15+. "Conditional" means allowed by the documented API model and selected for implementation, not physically certified. All rows remain unmeasured with meshChat. Both roles require permissions, radio availability, appropriate background configuration and runtime capability checks.

| Discovery initiator → advertiser | Both foreground | Advertiser background |
|---|---|---|
| Android → Android | Conditional | Conditional; service/process and permission dependent |
| Android → iOS | Conditional | Unsupported in our baseline; use iOS initiation or foreground the iPhone |
| iOS → Android | Conditional | Conditional; Android advertiser must remain available |
| iOS → iOS | Conditional | Conditional; explicit service-UUID scan, discovery may slow |

Apple documents UUID overflow and background scanning/advertising constraints; background scanning is slower when all scanners are backgrounded. Existing links must be evaluated separately: background modes allow event handling but do not guarantee survival or continuous timers. [Apple background guide](https://developer.apple.com/library/archive/documentation/NetworkingInternetWeb/Conceptual/CoreBluetooth_concepts/CoreBluetoothBackgroundProcessingForIOSApps/PerformingTasksWhileYourAppIsInTheBackground.html).

Android permits background connections but process death closes them; background service starts are restricted. A foreground service is not an indefinite-survival guarantee. [Android background BLE](https://developer.android.com/develop/connectivity/bluetooth/ble/background). If either process is killed, connection survival is unsupported; reconnect/relaunch remains conditional and unmeasured. Treat denied/revoked permission or disabled Bluetooth as unavailable, and show recovery instructions.

Our baseline provides no automatic recovery guarantee after user force-quit/force-stop. Apple's archived iOS 11 restoration table explicitly excludes user force-quit; it is a historical contract reference, not a current OS measurement. [QA1962](https://developer.apple.com/library/archive/qa/qa1962/_index.html). Screen lock is not proof of suspension. Duplicate-link races, low-MTU behavior and failure/recovery are retained in the [physical procedure](MC-004-ble-feasibility.md) for later gates.

iOS 26+ documents additional background privileges when a Live Activity is started with a CBManager before backgrounding. Our probe has no Live Activity, and the iOS 15+ baseline does not rely on this feature; a future change needs its own implementation and evidence. [Current Core Bluetooth overview](https://developer.apple.com/documentation/corebluetooth).

## Android permission and RSSI decision

Select the location-disclosed path already anticipated by design §8.3. Retain RSSI for advisory link scoring/proximity; it must not assert identity, precise distance or coordinates. Do not assert `neverForLocation`. On API 31+ request scan, advertise and connect permissions; request coarse/fine location together for location-derived proximity, with clear rationale. Denial disables proximity without a false location claim. On API 29/30, require fine location for discovery and separate background-location permission for background discovery. Keep location collection local; do not add GPS collection or transmit location.

This is a conservative product/permission choice, not a finding that every RSSI operation legally requires location permission. Android documents location requirements when scan results derive location and warns that `neverForLocation` can filter beacons. [Android permission guide](https://developer.android.com/develop/connectivity/bluetooth/bt-permissions). Production drivers must enforce denial/revocation behavior; the existing temporary probe is an experimental permission exerciser. Store disclosures and physical permission tests remain downstream obligations.

## Acceptance and remaining risk

MC-004 acceptance checks the source/method/limitations above, explicit matrix and permission choice, compiled bounded probes and recorded independent review. It does not certify any device pairing. Before radio/release acceptance, run the retained procedure on physical Android/Android, Android/iOS and iOS/iOS pairs, retain sanitized results and revise the supported matrix. If measured behavior contradicts an assumption, reopen MC-006/007 and affected implementations before release; do not silently weaken the protocol or safety bounds.
