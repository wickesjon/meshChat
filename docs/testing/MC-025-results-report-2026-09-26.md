# meshChat physical test results

**Report date:** September 26, 2026. **Status:** exploratory phone/tablet testing; full Android acceptance remains open.

The Galaxy S24 Ultra and Samsung tablet exchanged short and long messages, recovered from Bluetooth and lock transitions, and showed a connection in all 22 samples during a ten-minute observation. Bounded Android catch-up passed its limited item-cap scenario. On the current candidate, untraced L01 and traced L02 each delivered **14/14 directed messages**, including a background exchange. L01 median app-observed round trips were **14.933 seconds short / 19.501 seconds at 280 bytes**. L02 attributes most measured time to app preparation, queueing and post-receipt processing; repeated protected-storage work is the leading optimization target. These results do not establish battery efficiency, one-way latency or large-mesh reliability.

## Test setup

| Item | Configuration |
|---|---|
| Phone A | Samsung Galaxy S24 Ultra, SM-S928U1; Android 16/API 36 |
| Tablet B | Samsung SM-X510; Android 16/API 36 |
| Environment | Adjacent devices; uncontrolled radio environment; USB powered |
| Application | Debug build, existing synthetic profiles, public test channel |
| APK used for M02 and earlier final recovery observation | `eeb6fe05aba5b6acc763ce578fc163e55717245f71578901c83da60a61b263ec` |
| Catch-up candidate used for C05/C06 | `ab6d7d1413917db3064e62e39ed42613f4fce4ec04660022f5cb164e90dc4360` |
| Final catch-up candidate APK | `d80af3a9c54e8df01d422bdbc35a5e870fab63c6053e0e3950a6c728913e4733` |
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
| Bounded missed-message catch-up | PASS, C06/C07 limited scenario | Newest eight recovered; ninth/expired items excluded; protected history and no live replay verified |
| Current-candidate latency baseline/trace | PASS, L01 and L02, 14/14 each | Same app/test packages; complete trace for every sample, no overflow, background exchange and runner cleanup passed |

These are separate small scenarios on identified candidates, not one statistically representative delivery-rate study. Failed candidates remain recorded and are not removed from the evidence. No multi-hop, range, five-/ten-phone, flood or field-trial result is claimed.

## Bounded missed-message recovery

The Android app now attempts one bounded history session after a newly admitted peer supplies valid live activity. Recovered rows are stored through the existing protected owners and labelled as recovered history without granting trust. Native limits, expiry and incomplete-history wording remain intact; it does not repeatedly walk the peer's full history. The [procedure](MC-025-bounded-catchup.md) separates controlled source-cache/age setup from actual Bluetooth transfer.

Early runs exposed two distinct constraints. C01's same-time source fixture exceeded the sender burst allowance before transfer. C02–C04 then failed the eight-item receipt assertion: eager catch-up delayed initial presence traffic enough for the existing inactivity policy to close the link. C04's diagnostic trace showed SYNC preceding ANNOUNCE. The corrected candidate waits for valid live activity before requesting history; it does not extend the inactivity deadline.

On that correction, single-author run C05 kept the connection and recovered **six** rows, with a limited-history notice. This is consistent with the public sender bucket applying to rapid historical replay: old original posting times do not grant extra receive credit. The eight-item ceiling is a maximum, not a promise to recover eight messages under every workload. C05's exact-eight expectation remains a failed test and is not relabelled as a pass. The subsequent item-cap fixture uses three stable synthetic claimed senders, three recent messages each, to test the cap without overloading one sender's allowance.

**C06 passed on both devices**, including confirmation delivery and cleanup (phone 218.271 s; tablet 219.272 s). The receiver recovered exactly the newest **8/8 selected entries**, excluded the ninth and expired entry, retained the same native link for 30 checks separated by at least one second, and admitted no recovered item into its live forwarding cache. Relayed-chat count remained zero before confirmation; recovered rows remained unverified. Recovery was observed in **42.549 s** from the test's post-connection starting point, including application processing and polling. This is one functional sample, not a first-wire-frame latency measurement. Serialized checks took additional time beyond their nominal one-second sleeps, so the whole test duration is not catch-up latency.

The eight-item result appropriately retains the limited-history notice because a ninth eligible entry exists. C06 test APK SHA-256: `4faf76ddc4a3892f11d7e9e353bc291a9d8151399559991ebb72678530bfbf3a`. Existing profiles and history were preserved throughout.

**Final repeat C07 also passed on both devices** (phone 121.447 s; tablet 118.569 s) using the final app listed above and test APK `1e115f7d6853e758f67009badcdad3e9aa2997ab3fe966536d3890e2934e604e`. It repeated every selected-set, expiry, trust, no-live-replay and confirmation assertion. Recovery observation was **44.410 s**; the following same-link observation lasted **30.304 s across seven serialized checks**, with no ninth entry recovered. The shorter total runner duration reflects measuring a real 30-second observation window instead of thirty one-second sleeps plus dispatch overhead; it does not demonstrate faster Bluetooth recovery. Neither run is a battery or formal first-wire-frame latency measurement.

Earlier latency, traffic, memory and battery figures apply to the earlier APK, not automatically to this changed catch-up candidate. Controlled expiry timestamps are not a real 17-minute disconnection, and these small public messages do not certify the 8192-byte ceiling or the full simultaneous/authenticated scenario-E matrix.

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

With only three foreground samples per size, nearest-rank p95 is just the largest sample, not a reliable tail estimate. Polling is nominally 100 ms plus variable main-thread dispatch, encrypted-store/model refresh and logging overhead. The test measures model-history observation, not final UI painting. These RTTs cannot certify the different MC-007 one-way/multi-hop workload. The later same-candidate baseline and trace below investigate delay attribution separately.

Initial run **M01** also recorded all 14 directed messages, but both test runners subsequently failed activity cleanup. This is **not a passing instrumentation run**. The harness's conflicting foreground launch was removed, and M02 passed including cleanup. Preserve M01 separately rather than pooling its samples: short-message RTTs 19.844, 21.096 and 14.833 seconds; 280-byte RTTs 24.943, 19.501 and 23.838 seconds; background RTT 19.783 seconds. No production code changed for these measurements.

## Round-trip tracing on the catch-up candidate

The [trace procedure](MC-025-latency-tracing.md) repeats the seven-pair workload on production APK `d80af3a9c54e8df01d422bdbc35a5e870fab63c6053e0e3950a6c728913e4733`, with test APK `0bc9745607f389a4a2abbbda1a619b6b1e765147eda4f5b099a3919749ab3987`. Both traced and untraced runs first let the initial bounded catch-up settle. Only the test package changed; profiles/history and all storage, admission and lock policies remain intact. M02 used a different production candidate and is not an optimization comparison.

Untraced **L01 passed on both devices**, including cleanup (A 188.989 s; B 194.379 s). All **14/14** directed messages arrived without an observed refusal/timeout; B was verified below STARTED for the last pair.

| L01 workload | Pairs | Individual round trips, seconds | Median, seconds |
|---|---:|---|---:|
| Foreground, 12-byte text | 3/3 | 13.807; 20.042; 14.933 | 14.933 |
| Foreground, 280-byte text | 3/3 | 19.501; 21.666; 18.145 | 19.501 |
| Background responder, 12-byte text | 1/1 | 19.187 | — |

Traced **L02 passed on both devices**, including protected-read probes and cleanup (A 199.159 s; B 203.357 s). Again **14/14** directed messages arrived, with no observed refusal/timeout. All 26 sample frame submissions were correlated and accepted: one frame per short direction and three per 280-byte direction. A/B buffered 91/84 trace entries with zero overflow. The analyzer accounted for all seven exact RTTs and rejected no timing gaps.

| L02 workload | Pairs | Individual round trips, seconds | Median, seconds |
|---|---:|---|---:|
| Foreground, 12-byte text | 3/3 | 21.014; 12.184; 19.032 | 19.032 |
| Foreground, 280-byte text | 3/3 | 16.956; 21.837; 19.715 | 19.715 |
| Background responder, 12-byte text | 1/1 | 16.387 | — |

The traced short median is 4.099 s above L01; the long median is 0.214 s above it. These three-sample groups overlap substantially. They neither establish negligible tracing overhead nor separate overhead from normal variability. No production optimization was made or speed improvement claimed.

The following are **arithmetic means of the same L02 sample groups**, so the component means add to the mean full RTT (apart from rounding). Every component duration uses one device's clock; there is no subtraction of unsynchronized clocks.

| Round-trip component, seconds | Short foreground, n=3 | 280-byte foreground, n=3 | Background, n=1 |
|---|---:|---:|---:|
| Request start → first submission, plus echo call → first submission | 5.350 | 4.760 | 2.995 |
| First → last frame submission, both directions | 0.000 | 4.306 | 0.000 |
| Complete receive callback → model observation, both devices | 11.354 | 9.478 | 11.889 |
| Responder observation → echo send call | 0.002 | 0.001 | 0.005 |
| Remaining delivery/callback time | 0.704 | 0.958 | 1.498 |
| **Full round trip, mean** | **17.410** | **19.503** | **16.387** |

For short foreground messages, roughly **96%** of traced elapsed time lies in preparation/queueing and post-receipt processing/observation. For long messages, those intervals account for roughly **73%**, while the two fragmentation spans account for about **22%**. The fragment spans include scheduling, completion callbacks and contention between submissions; they are not pure on-air time. The remaining delivery/callback component is a residual that includes platform and admission/lock waiting before the observed callback, not a one-way measurement.

Queue diagnostics reinforce the application-side finding. Across the seven samples, send-queue wait was **0.808–2.165 s on A** and **1.099–3.369 s on B**; receive-queue wait reached **3.279 s / 3.373 s**. Send-work brackets were **2.943–3.419 s / 3.914–3.940 s**, and receive-work brackets **2.088–2.611 s / 2.785–3.326 s**. These brackets may contain interleaved work and overlap the partition above. Actual native submission calls took **0–4 ms** at millisecond resolution; protected-egress entry-to-submit waiting ranged **0–496 ms**. A fast submission call does not prove fast controller completion.

After radio stop, five ordinary protected setting reads took **429, 425, 426, 442, 422 ms** on the phone (median **426 ms**) and **550, 555, 546, 552, 541 ms** on the tablet (median **550 ms**). The interleaved native status calls took **0–1 ms**. The protected-read measurement includes identity loading, Keystore/key access, encrypted database/native-store open, read and cleanup. It does not isolate SQLCipher derivation or any other subcomponent.

Source inspection explains why this cost can accumulate: `MeshModel.refresh` separately loads events, history and friend cards; send/receive paths call refresh repeatedly; periodic work also performs protected retry/organizer operations. `EncryptedStorage.operation` opens and closes the protected store for each call, and some of that work shares serialization with radio callbacks. **Repeated protected operations and redundant refreshes are the leading cause hypothesis**, supported by the measured queue delays and read cost; exclusive per-method/storage-subcomponent profiling is still needed to assign causality precisely.

The next optimization should reduce redundant refreshes and repeated protected opens within a bounded authorized operation, with unchanged lock/reset checks, key zeroing, resource cleanup and native limits. Measure protected-operation counts/durations and radio-monitor hold time before changing behavior, then repeat the same baseline and regression workload. This investigation does not justify weaker encryption settings or longer-lived unguarded keys/handles.

Raw L01/L02 logs, installed-package hash manifest and derived per-pair JSON remain under ignored `.work/mc025/2026-09-26-latency/`. Test compilation/lint, five attribution tests, ticketboard validation and twelve board tests pass. The production APK hash remains unchanged. Both normal apps were reopened after runner exit; MC-025/full physical acceptance remains open.

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
| Android did not request/process missed history | Connect the native requester to protected history processing with one attempt per ready peer | C06 newest-eight physical test passed |
| Eager catch-up delayed presence traffic and triggered inactivity closure | Wait for native-observed valid live activity before requesting history | C02–C04 failed; C06 passed with existing inactivity limits preserved |

The earlier recovery candidate passed both targeted refusal/Bluetooth regressions (2/2, 26.999 seconds). Applicable Android app/BLE builds, lint, 22 app JVM tests, 26 BLE JVM tests and APK alignment checks passed. Storage-policy checks passed 14 tests. These automated checks complement device observations; they do not replace hardware or independent review gates.

The final catch-up candidate also clears queued catch-up attempts when the radio stops, loses permission or is disabled. Its repeated-Connect/protected-read and Bluetooth shutdown/recovery regressions passed **2/2 in 114.704 s**. Fresh Debug/Release app builds, lint and 22 app JVM tests passed; both APKs passed native-library and 16 KiB alignment checks. The protected native catch-up suite passed 4/4 and storage-policy checks 14/14. The shared security adapter compiled and passed lint, but a fresh standalone Android security-probe native rebuild was unavailable because the recorded NDK installation was missing. This is not a security-probe runtime or independent-assessment pass.

## Next evidence to collect

1. Attribute the observed 15–26-second app round trips with finer timing instrumentation, and repeat with a larger declared sample. Do not assume radio transmission alone accounts for that delay.
2. Extend bounded catch-up evidence to the full simultaneous-direction, authenticated, negotiated-capacity and long-disconnection matrix. Keep single-author burst behavior distinct from the non-overloaded item-cap fixture. The normal composer's offline-send refusal remains unchanged.
3. Six-hour powered Beacon endurance and separately controlled battery/energy runs.
4. Additional phones for mixed-manufacturer, multi-hop, five-/ten-device and field scenarios; separate physical key/storage verification.

## Evidence and status

The [physical acceptance packet](MC-025-android-physical-acceptance.md) records candidate hashes, failed runs, UTC observations and limitations. The [active MC-025 ticket](../ticketboard/inprogress/MC-025-android-bench-mesh-and-early-field-gate.md) records implementation scope and validation. Raw device mappings, synthetic payloads, UI snapshots and logs remain in ignored repository-local evidence directories.

M02 test APK SHA-256: `36961bab4794c82cdac08367a63689ee9dd74d4de9df9d71c956fefcf7be1984`. The test-only build passed; the paired physical tests passed on A in 180.559 seconds and B in 180.004 seconds. Raw M01/M02 records, hash manifests, battery snapshots and aggregate JSON are retained under `.work/mc025/2026-09-26-measurement/`. The [test source](../../tests/integration/android-ui/android/PhysicalRoundTripTest.kt) and [procedure](MC-025-paired-measurement.md) make the measurement reproducible; existing emulator runners retain their physical-device guards.

MC-025 remains in progress. No final independent review, main-branch merge, beta approval or full physical certification is claimed.
