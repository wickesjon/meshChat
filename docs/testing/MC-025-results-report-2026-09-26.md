# meshChat physical test results

**Report date:** September 26, 2026. **Status:** exploratory phone/tablet testing; full Android acceptance remains open.

Latest ready-link candidate results: **120/120 round trips delivered**, with median full RTT **627/643 ms** (phone initiating, short/long) and **701.5/684 ms** (tablet initiating). See the [complete physical report](MC-025-ready-link-results-2026-09-26.md) for p95/max, exact builds, the initial Bluetooth-recovery timeout and remaining gates. Battery drain remains unmeasured.

Earlier results: the Galaxy S24 Ultra and Samsung tablet exchanged short and long messages, recovered from Bluetooth and lock transitions, and passed a bounded catch-up scenario. A focused optimization reduced repeated protected-store opens and refreshes. Untraced O01 delivered **14/14 directed messages**, including a background exchange; median app-observed round trips fell from **14.933 to 7.496 seconds for short messages** and **19.501 to 11.432 seconds for 280-byte messages**, compared with L01. Those are observed reductions of **49.8% and 41.4%** in three-sample groups. These results do not establish battery efficiency, one-way latency or large-mesh reliability.

## Test setup

### Sub-second candidate: implementation validation

The next candidate implements [raw-key envelope migration and negotiated encoding](../decisions/MC-025-ready-link-latency.md). Debug APK SHA-256: `2bb9ab10c2fd9864e6c34ff622806fde38be10a39a249686a2578285e313953c`; Release unsigned APK: `1254f74fff7ef6ac0604e3479387772f5c9429f8491d7575060e89731b76df95`. These are locally built candidates, not the physically measured O01/O02 APK above. Neither device was exercised during the implementation session; the subsequent paired session is recorded in the [ready-link results](MC-025-ready-link-results-2026-09-26.md).

| Check | Result and limit |
|---|---|
| Core behavior | Full debug suite: 210 passed; final focused transport/relay suites: 31 passed; full release suite: 211 passed. The extra test covers local HELLO completion preceding peer HELLO. |
| Framing | Asymmetric 512/146-byte directions deliver the long fixture in one/three frames respectively; 182-byte SYNC uses two. HELLO ordering/replay, queued/in-flight refusal, stale/repeated capacity activation and unchanged pacing pass. These are host tests, not Bluetooth timings. |
| Storage policy | 14 host policy tests pass; binary derivation known answers and malformed key material pass through generated Kotlin/native bindings. |
| Android | Fresh arm64/x86_64 native libraries; Debug/Release/test APK builds, lint and all 24 app JVM tests pass, including full-wire vectors. Both app APKs pass packaged native-library and 16-KiB alignment checks. |
| Synthetic migration | Final isolated API-29 emulator run: **3 tests pass in 4.902 seconds**, covering legacy history, unchanged database bytes, pre/post envelope-commit failures, seal failure, lock-before-commit, wrong generation/key, key loss and fresh raw-key reopening. Actual Android wrapping-key deletion and guarded identity reset also pass; the fresh identity cannot access old history and stale identity handles are refused. This is emulator evidence. |
| Existing storage lifecycle | Fresh security-probe Debug/Release/test builds and lint pass. Its guarded emulator runner passes all five phases: create, reopen, encryption/transaction/replay checks, wrapping-key loss and reset. This includes the encrypted rollback-journal check using the new raw-key argument. |
| Dependency | Pinned RustCrypto PBKDF2 adds one package to each lockfile, reusing existing dependencies. Refreshed cargo-deny advisory/license/source/ban gates pass. |
| Physical RTT | Subsequent paired candidate runs pass the typical sub-second target in both orientations; see the [ready-link results](MC-025-ready-link-results-2026-09-26.md). Earlier 7.496/11.432-second medians belong to the prior APK. |
| Power/battery | **Not measured.** Less repeated derivation is not proof of reduced battery drain. |
| Remaining gates | Broader physical acceptance, intermittent phone Bluetooth recovery, real-device security and battery checks, Mac/Xcode native consumer validation and independent security assessment remain open. |

Validation used Rust 1.85.1, cargo-deny 0.20.2, JDK 17.0.15+6, Gradle 8.13, AGP 8.11.1, Kotlin 2.2.0, pinned NDK 27.3.13750724 and the pinned aligned SQLCipher 4.17.0 Android AAR. Commands: workspace `cargo fmt`, strict all-target/all-feature `cargo clippy`, debug/release workspace tests and release build; `tests/integration/storage/run_policy.py`; app assemble Debug/Release/androidTest, lint and JVM tests; `check_apk.py` and `zipalign -c -P 16 4`. Logs remain in ignored `.work/mc025/subsecond-*.log`.

Extended lifecycle commands: `src/core/build_bindings.py android --security-probe`; standalone security Gradle `assembleDebug assembleRelease assembleDebugAndroidTest lintDebug`; `tests/integration/storage/run_android.py --serial emulator-5580`. Migration suite component: `org.meshchat.storage.StorageMigrationTest` through the app AndroidJUnitRunner; test APK SHA-256 `bce40310a99670b13b93038025f94082c7548158c6d0c628c2d105bb0ffc6941`. The explicit emulator check protects physical profiles; only the isolated security fixture was reset.

Independent Terra/medium code review of [PR #39](https://github.com/wickesjon/meshChat/pull/39), revision `dc4fc679dfdbec5ea0f6c06eaa270723258adb12`, completed with no actionable findings. The PR remains unmerged because physical/platform/security acceptance is incomplete. This review does not replace the independent security assessment.

During development, existing test parsers that assumed a 146-byte encoder were corrected to the actual negotiated capacity. One full-wire JVM invocation could not refresh the registry under restricted networking; after updating its separate lockfile and using the populated offline cache, all 24 tests passed. A concurrent host-DLL link attempt was retried after the JVM released the DLL. Neither attempt is counted as passing evidence.

### Physical setup used for earlier results

| Item | Configuration |
|---|---|
| Phone A | Samsung Galaxy S24 Ultra, SM-S928U1; Android 16/API 36 |
| Tablet B | Samsung SM-X510; Android 16/API 36 |
| Environment | Adjacent devices; uncontrolled radio environment; USB powered |
| Application | Debug build, existing synthetic profiles, public test channel |
| APK used for M02 and earlier final recovery observation | `eeb6fe05aba5b6acc763ce578fc163e55717245f71578901c83da60a61b263ec` |
| Catch-up candidate used for C05/C06 | `ab6d7d1413917db3064e62e39ed42613f4fce4ec04660022f5cb164e90dc4360` |
| Pre-optimization catch-up/trace APK | `d80af3a9c54e8df01d422bdbc35a5e870fab63c6053e0e3950a6c728913e4733` |
| Optimization APK | `a8951afe78cc0669bac9fde4ad9be251ed22ea6159c88f8622d8bcb78f90f7d2` |
| Earlier recovery source checkpoint | `c91894f`; later evidence checkpoint `cc053df` |
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
| Pre-optimization latency baseline/trace | PASS, L01 and L02, 14/14 each | Same app/test packages; complete trace for every sample, no overflow, background exchange and runner cleanup passed |
| Optimized latency baseline/trace | PASS, O01 and O02, 14/14 each | Lower observed RTT; complete trace, background exchange, protected-read probes and cleanup passed |

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

These findings motivated the bounded-read and redundant-refresh optimization below. Further attribution of storage subcomponents, protected-operation counts and exclusive radio-monitor hold time remains useful. This investigation does not justify weaker encryption settings or longer-lived unguarded keys/handles.

Raw L01/L02 logs, installed-package hash manifest and derived per-pair JSON remain under ignored `.work/mc025/2026-09-26-latency/`. Test compilation/lint, five attribution tests, ticketboard validation and twelve board tests pass. The production APK hash remains unchanged. Both normal apps were reopened after runner exit; MC-025/full physical acceptance remains open.

## Android latency optimization

Following L01/L02, a focused production change combines event-card, selected-channel history and friend-card reads into one bounded `channelScreen` operation, instead of opening protected storage separately three times. It also removes the second refresh after a successful public send, keeps the reaction caller's refresh, and avoids rereading the selected channel's history solely for its preview. Other-channel previews/unread behavior and separate DM/event-message operations remain intact. The existing protected operation still performs its unlock/generation/reset checks and closes the store/connection and clears the key before returning. Encryption configuration, native limits and wire behavior are unchanged.

Optimization app SHA-256: `a8951afe78cc0669bac9fde4ad9be251ed22ea6159c88f8622d8bcb78f90f7d2`. Test APK: `5e7e07c79e662c98fa4430a866ac83102e20e4a298c66698eb28ccdbd8c0edc8`. Existing profiles/history were retained during installation. Raw evidence is under ignored `.work/mc025/2026-09-26-optimization/`.

The physical snapshot test compares serialized history/event fields and friend metadata between the individual reads and the batch, with fixed friend-observation time and radio stopped. All three repetitions match on each device, including the null-selected-channel empty-row case. It logs only counts and durations.

| Snapshot measurement | Galaxy S24 Ultra | Samsung tablet |
|---|---|---|
| Existing history rows compared each repetition | 71 | 99 |
| Three individual reads, ms | 1132; 1164; 1143 | 1679; 1676; 1678 |
| Combined read, ms | 389; 389; 382 | 578; 586; 578 |
| Median reduction | 66.0% | 65.6% |

These installed profiles have zero event/friend cards, so their empty results are compared but populated event/friend equivalence is not established by this physical check. The existing native feature and protected-provider checks remain separate evidence. Phone snapshot + repeated Connect + Bluetooth recovery pass **3/3 in 58.118 s**; tablet snapshot passes **1/1 in 19.464 s**. This comparison isolates fewer protected opens; it does not measure one-way Bluetooth or battery savings.

Bounded catch-up repeat **C08 passes** on both endpoints, including confirmation and cleanup (A 90.156 s; B 90.550 s). The receiver again gets exactly the newest eight, excludes the ninth/expired items, preserves recovered/unverified labels and zero recovered live-cache/relay entries. Recovery is observed in **30.994 s**, followed by **30.728 s across 13 same-link checks** with no ninth entry. C07's corresponding observation was 44.410 s; these are individual functional samples, not a controlled first-wire-frame latency estimate.

Untraced **O01 passes**, A 117.124 s/B 117.799 s, with **14/14** directed deliveries, no observed refusal/timeout and verified background receipt. It uses the unchanged paired workload and catch-up settling guard.

| Workload | O01 round trips, seconds | Previous L01 median | O01 median | Observed median reduction |
|---|---|---:|---:|---:|
| Short foreground, n=3 | 7.496; 10.344; 7.152 | 14.933 s | 7.496 s | 49.8% |
| 280-byte foreground, n=3 | 8.936; 13.532; 11.432 | 19.501 s | 11.432 s | 41.4% |
| Short background, n=1 | 7.169 | — | — | — |

The same devices, USB power, Debug configuration and workload were used. History was retained and grew between runs, and radio/thermal conditions were uncontrolled. These are small exploratory comparisons rather than a randomized benchmark or a reliable tail estimate. The within-device snapshot-equivalence measurements independently show the expected benefit of reducing three store opens to one.

Traced **O02 also passes**, A 120.745 s/B 124.279 s, **14/14** directed deliveries without observed refusals/timeouts. All 26 sample frames correlate and are accepted; 87/80 trace entries have zero overflow. Short RTTs are **6.171, 11.364, 6.346 s** (median **6.346 s**), long RTTs **11.745, 12.966, 11.645 s** (median **11.745 s**), and the background pair **5.042 s**. O02 versus O01 still does not isolate tracing overhead from variability.

| Mean traced component, seconds | L02 short | O02 short | L02 long | O02 long |
|---|---:|---:|---:|---:|
| Preparation to first submissions, both endpoints | 5.350 | 2.486 | 4.760 | 2.715 |
| Fragment submission spans, both directions | 0.000 | 0.000 | 4.306 | 4.593 |
| Complete receipt → model observation, both endpoints | 11.354 | 4.829 | 9.478 | 4.004 |
| Responder observation → echo send call | 0.002 | 0.002 | 0.001 | 0.002 |
| Remaining delivery/callback time | 0.704 | 0.643 | 0.958 | 0.804 |
| **Full RTT mean, n=3 per group** | **17.410** | **7.960** | **19.503** | **12.119** |

The improvement is concentrated in the expected application processing intervals. Across all seven O02 samples, send-work brackets are **0.808–0.864 s on A** and **1.157–1.713 s on B**; receive-work brackets are **0.786–0.844 s / 1.138–1.162 s**. These diagnostic brackets can contain interleaved work and overlap the partition. Tablet send/receive queue waits still reach **2.866/3.376 s**, so periodic work/contention remains worth profiling. Long-message fragmentation spans did not improve in this small trace and now account for about **38%** of mean long RTT; their scheduling/callback/lock components are not isolated radio airtime.

Ordinary protected reads after O02 still take **387, 396, 389, 403, 387 ms** on A and **541, 556, 540, 553, 543 ms** on B (medians **389/543 ms**). Native status calls remain **0–1 ms**. The optimization reduces how many protected opens the message path performs; it does not bypass or substantially change the cost of an individual protected open.

Debug/Release/test builds, lint and **22 JVM tests** pass; the standalone shared-security adapter compiles and passes lint. Native history/catch-up tests pass **3+4**, storage policies **14**, attribution tests **5**, ticketboard tests **12**, and both app APKs pass native-set/ELF/ZIP alignment checks. No production wire/crypto/FFI change was made. The previously recorded standalone security-probe native rebuild limitation and full physical/independent review gates remain open. All requested device runs finished and both normal apps were reopened.

## Sub-second feasibility follow-up

The user selected typical full RTT on a ready direct connection as the target. **Sub-second production RTT has not been achieved.** The [feasibility note](MC-025-subsecond-feasibility.md) identifies the two main changes needed: avoid repeated password derivation when opening a database keyed by high-entropy random material, and use actual negotiated frame capacity instead of the current fixed 146-byte encoding ceiling. With the existing one-frame/second shaping, the current three-frame long message has at least four seconds of round-trip fragment spacing. Changing the encoding ceiling must preserve bootstrap, queue/generation correctness and all traffic budgets.

A new isolated synthetic database benchmark passes on both devices (A 3.298 s; B 4.292 s). Five open/read/close samples per mode yield median **420 → 13 ms on the phone** and **541 → 20 ms on the tablet**, comparing the existing random-passphrase format with SQLCipher raw-key format. The same library, database settings and native store read are used. Wrong-key refusal and absence of the synthetic plaintext marker in each database file pass; scratch files are removed. Keystore/identity wrapping is omitted on both paths, and the small fixed-order benchmark does not establish full app performance or migration safety. SQLCipher documents raw-key mode for high-entropy keys in its [performance guidance](https://www.zetetic.net/sqlcipher/performance/).

The production app/profile remain unchanged. Test APK `53bebb15ac6d79aa6e32db322501e2fa885facdcf2bc3c9079bc4967218bd6c6` compiles and passes lint. A real storage migration needs a durable versioned key/database transition, lock/reset/key-loss/fault tests and applicable review; retaining plaintext keys or weakening page authentication is not the proposed route. The result supports further engineering toward the target, with no sub-second or battery claim yet.

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

1. Profile remaining periodic protected work and radio-monitor contention, then repeat with a larger declared sample. Current tracing and the first optimization are recorded above; remaining RTT is still several seconds and is not pure radio time.
2. Extend bounded catch-up evidence to the full simultaneous-direction, authenticated, negotiated-capacity and long-disconnection matrix. Keep single-author burst behavior distinct from the non-overloaded item-cap fixture. The normal composer's offline-send refusal remains unchanged.
3. Six-hour powered Beacon endurance and separately controlled battery/energy runs.
4. Additional phones for mixed-manufacturer, multi-hop, five-/ten-device and field scenarios; separate physical key/storage verification.

## Evidence and status

The [physical acceptance packet](MC-025-android-physical-acceptance.md) records candidate hashes, failed runs, UTC observations and limitations. The [active MC-025 ticket](../ticketboard/inprogress/MC-025-android-bench-mesh-and-early-field-gate.md) records implementation scope and validation. Raw device mappings, synthetic payloads, UI snapshots and logs remain in ignored repository-local evidence directories.

M02 test APK SHA-256: `36961bab4794c82cdac08367a63689ee9dd74d4de9df9d71c956fefcf7be1984`. The test-only build passed; the paired physical tests passed on A in 180.559 seconds and B in 180.004 seconds. Raw M01/M02 records, hash manifests, battery snapshots and aggregate JSON are retained under `.work/mc025/2026-09-26-measurement/`. The [test source](../../tests/integration/android-ui/android/PhysicalRoundTripTest.kt) and [procedure](MC-025-paired-measurement.md) make the measurement reproducible; existing emulator runners retain their physical-device guards.

MC-025 remains in progress. No final independent review, main-branch merge, beta approval or full physical certification is claimed.
