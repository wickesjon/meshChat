# meshChat physical test results

**Report date:** September 26, 2026. **Status:** exploratory phone/tablet testing; full Android acceptance remains open.

The Galaxy S24 Ultra and Samsung tablet exchanged short and long messages, recovered from Bluetooth and lock transitions, and showed a connection in all 22 samples during a ten-minute observation. The latest measured run delivered **14/14 directed messages**, including a background exchange. Median app-observed round-trip time was **16.006 seconds for short messages** and **21.955 seconds for 280-byte messages**. Testing found and fixed several connection/recovery faults and identified missing Android catch-up integration. These results do not establish battery efficiency, one-way latency or large-mesh reliability.

## Test setup

| Item | Configuration |
|---|---|
| Phone A | Samsung Galaxy S24 Ultra, SM-S928U1; Android 16/API 36 |
| Tablet B | Samsung SM-X510; Android 16/API 36 |
| Environment | Adjacent devices; uncontrolled radio environment; USB powered |
| Application | Debug build, existing synthetic profiles, public test channel |
| Final installed APK SHA-256 | `eeb6fe05aba5b6acc763ce578fc163e55717245f71578901c83da60a61b263ec` |
| Production source checkpoint | `c91894f`; later evidence checkpoint `cc053df` |
| Preservation | No identity/history reset, permission blanket grant or security/power-setting change |

Both endpoints are Samsung devices on the same Android API level. A tablet is useful for development testing but does not satisfy the required second-phone or mixed-manufacturer coverage.

## Connectivity and delivery

| Scenario | Result | What was verified |
|---|---|---|
| Initial short-message exchange | PASS, 2/2 | Exact message present in each opposite device's history |
| Long messages | PASS, 2/2 at 280 bytes each | Exact full text at each recipient; earlier restart-fix candidate |
| App restart | Initial FAIL; later functional retest PASS, 2/2 | Stale service handles were invalidated; real peer recovered without bypassing connection limits |
| Bluetooth off/on | Initial FAIL; final retest PASS, 2/2 | Correct Bluetooth-disabled notice, reconnect and fresh delivery both ways |
| Foreground connection observation | PASS for 619.488 seconds | 22/22 sampled screens connected; same application processes; no manual recovery |
| Messages after observation | PASS, 2/2 | Fresh messages confirmed on both recipients |
| Real device lock | PASS for observed shutdown | Android reported locked; transport service absent four seconds later |
| Unlock and explicit reconnect | PASS, 2/2 | Existing profiles available; two consecutive connected samples; fresh messages both ways |
| Measured request/echo run M02 | PASS, 14/14 directed deliveries | Seven requests and seven exact remote echoes; zero observed refusals/timeouts; both test runners passed |
| Tablet activity in background, M02 | PASS, 2/2 directed deliveries | Activity verified below STARTED when the last request arrived; reply received on phone; not a screen-off/locked endurance test |
| Missed-message catch-up | NOT PASSED; integration gap | Core requester exists, but Android production code does not call `requestCatchup` or `processCatchup` |

These are separate small scenarios on identified candidates, not one statistically representative delivery-rate study. Failed candidates remain recorded and are not removed from the evidence. No multi-hop, range, five-/ten-phone, flood or field-trial result is claimed.

## Latency and recovery time

**One-way delivery latency and its p50/p95 have not yet been measured.** Existing send/receipt timestamps include typing, scrolling, delayed UI reads and operator pauses. They cannot be used as radio-latency measurements.

| Observation | Result | Interpretation |
|---|---|---|
| Restart retest on earlier candidate | Both screens connected about 286 seconds after peer stop | An existing five-minute address cooldown was encountered; this remains a recovery usability concern |
| Final Bluetooth retest | Bluetooth restored 13:38:10 UTC; both connected at 13:38:34 and 13:39:02 | Connection was observed 24 seconds after restoration; includes explicit Connect and polling, not a precise protocol latency |
| Unlock recovery | Both connected at 19:02:55 and 19:03:26 UTC | Successful operator-assisted recovery; hours spent awaiting the operator are not recovery latency |

A paired instrumentation follow-up measures **app-observed round-trip time** with one monotonic clock. It includes sending, Bluetooth scheduling, recipient processing, the echo, and application observation. It must not be divided by two and presented as measured one-way latency. The [measurement procedure](MC-025-paired-measurement.md) defines the workload and counters.

Run **M02** completed on the unchanged production APK. Each request and echo had the same byte length. All seven requests were accepted and all seven echoes arrived within the predeclared 60-second observation limit.

| M02 workload | Completed pairs | Individual round trips, seconds | Median, seconds | Range, seconds | Nearest-rank p95, seconds |
|---|---:|---|---:|---|---:|
| Foreground, 12-byte text | 3/3 | 14.650; 16.006; 18.120 | 16.006 | 14.650–18.120 | 18.120 |
| Foreground, 280-byte text | 3/3 | 18.972; 21.955; 26.310 | 21.955 | 18.972–26.310 | 26.310 |
| Background responder, 12-byte text | 1/1 | 16.556 | — | — | — |

With only three foreground samples per size, nearest-rank p95 is just the largest sample, not a reliable tail estimate. Polling is nominally 100 ms plus variable main-thread dispatch, encrypted-store/model refresh and logging overhead. The test measures model-history observation, not final UI painting. These RTTs cannot certify the different MC-007 one-way/multi-hop workload or explain which layer contributes the delay; a separate timing trace would be needed for attribution.

Initial run **M01** also recorded all 14 directed messages, but both test runners subsequently failed activity cleanup. This is **not a passing instrumentation run**. The harness's conflicting foreground launch was removed, and M02 passed including cleanup. Preserve M01 separately rather than pooling its samples: short-message RTTs 19.844, 21.096 and 14.833 seconds; 280-byte RTTs 24.943, 19.501 and 23.838 seconds; background RTT 19.783 seconds. No production code changed for these measurements.

## Measured Bluetooth traffic

M02 used Normal power mode. The following are differences between existing production contribution snapshots; no counters were reset. A's snapshots were observed 145.193 seconds apart and B's 143.006 seconds apart, around 19:27:37–19:30:02 UTC. Snapshot fields are refreshed asynchronously, so these are observation windows, not synchronized packet-capture boundaries.

| Counter increase | Galaxy S24 Ultra | Samsung tablet |
|---|---:|---:|
| Received frames | 21 | 21 |
| Received frame bytes | 2,122 | 2,111 |
| Received logical packets, including control | 11 | 11 |
| Received chat packets | 7 | 7 |
| Scheduled frames | 21 | 21 |
| Successfully completed native frames | 21 | 21 |
| Successfully completed native frame bytes | 2,111 | 2,122 |
| Completed logical objects | 11 | 11 |
| Relayed chat copies | 0 | 0 |

Both snapshots on both devices showed one direct peer. Counters include control traffic and application frame bytes; they exclude Bluetooth link-layer overhead and do not enumerate every controller/air retry. Successful local completion does not itself prove delivery—the exact remote history checks establish the 14/14 result. Zero relayed copies is expected for this direct two-endpoint test and says nothing about mesh suppression efficiency.

## Battery, power and memory

The connection observation ran from **13:45:15.031 to 13:55:34.514 UTC**. Both devices were plugged into USB with screens active.

| Measurement | Galaxy S24 Ultra | Samsung tablet |
|---|---:|---:|
| Duration | 10 min 19.488 s | 10 min 19.488 s |
| Connected samples | 11/11 | 11/11 |
| Battery charge, start → end | 67% → 68% | 36% → 35% |
| Net charge change | +1 percentage point | −1 percentage point |
| Battery temperature, start → end | 36.5°C → 36.5°C | 27.3°C → 27.3°C |
| Total app PSS, minimum–maximum | 139,989–155,418 KiB | 122,766–128,666 KiB |
| Total app PSS, first → last | 147,457 → 144,087 KiB | 122,766 → 128,570 KiB |
| Matching restart/fatal-crash events in app-scoped capture | 0 | 0 |

**Battery drain, power in watts, energy in watt-hours and projected runtime are not established.** Charging input, coarse battery percentages, short duration and screen/automation overhead prevent attributing these changes to meshChat. In particular, the tablet's one-point decrease while USB-powered is not a measured meshChat drain rate. No hourly extrapolation is appropriate.

PSS measures total app memory including runtime/library overhead. These samples do not establish absence of leaks or prove the protocol's native memory bounds. Empty app-scoped diagnostic captures are supporting observations, not proof that no lower-level event occurred.

The formal energy procedure requires a 15-minute warm-up, four-hour paired runs, 60–90% starting battery, 20–25°C ambient conditions, an idle-app control and three repetitions per mode/device. The six-hour powered Beacon endurance run is also outstanding.

During the separate approximately three-minute M02 instrumentation run, A remained at 100% and B at 58%, both USB-powered. Battery temperature changed from 35.3°C to 36.2°C on A and 26.8°C to 26.9°C on B. Unchanged integer battery percentages do not mean zero energy consumption; these readings also cannot establish battery drain or energy efficiency.

## Faults found and corrected

| Finding | Correction | Evidence |
|---|---|---|
| Connect incorrectly reported protected data locked | Serialize model reads with startup radio activity | Original startup regression failed; fixed three-cycle regression passed |
| Rejected incoming links repeatedly recreated the Bluetooth server | Cancel/retire on the current server epoch | Original injected regression failed; fixed regression passed |
| Peer restart left stale Bluetooth characteristic handles | Invalidate established cached handles on service change | Functional restart recovery retest passed |
| Bluetooth off incorrectly reported protected data locked | Treat temporary busy state separately from protected-storage failure | Original Bluetooth regression failed; corrected regression passed |
| Permanent refusal retirement could strand isolated peers | Replace the server epoch after a bounded delay only when no setup/established link exists | Enhanced regression passed; final paired recovery and observation passed |

The final installed APK passed both targeted refusal/Bluetooth regressions (2/2, 26.999 seconds). Applicable Android app/BLE builds, lint, 22 app JVM tests, 26 BLE JVM tests and APK alignment checks passed. Storage-policy checks passed 14 tests. These automated checks complement device observations; they do not replace hardware or independent review gates.

## Next evidence to collect

1. Attribute the observed 15–26-second app round trips with finer timing instrumentation, and repeat with a larger declared sample. Do not assume radio transmission alone accounts for that delay.
2. Wire the Android app to the existing protected catch-up requester/processor, then run a reproducible selected-set test. Source inspection found no Android calls to `requestCatchup` or `processCatchup`; the core APIs and iOS caller exist. This is a product integration gap, not just missing physical evidence. The normal composer also refuses sends with no connected peer, so simply disconnecting the only receiver cannot create a valid missed-message workload. Do not relax that rule to manufacture a test pass.
3. Six-hour powered Beacon endurance and separately controlled battery/energy runs.
4. Additional phones for mixed-manufacturer, multi-hop, five-/ten-device and field scenarios; separate physical key/storage verification.

## Evidence and status

The [physical acceptance packet](MC-025-android-physical-acceptance.md) records candidate hashes, failed runs, UTC observations and limitations. The [active MC-025 ticket](../ticketboard/inprogress/MC-025-android-bench-mesh-and-early-field-gate.md) records implementation scope and validation. Raw device mappings, synthetic payloads, UI snapshots and logs remain in ignored repository-local evidence directories.

M02 test APK SHA-256: `36961bab4794c82cdac08367a63689ee9dd74d4de9df9d71c956fefcf7be1984`. The test-only build passed; the paired physical tests passed on A in 180.559 seconds and B in 180.004 seconds. Raw M01/M02 records, hash manifests, battery snapshots and aggregate JSON are retained under `.work/mc025/2026-09-26-measurement/`. The [test source](../../tests/integration/android-ui/android/PhysicalRoundTripTest.kt) and [procedure](MC-025-paired-measurement.md) make the measurement reproducible; existing emulator runners retain their physical-device guards.

MC-025 remains in progress. No final independent review, main-branch merge, beta approval or full physical certification is claimed.
