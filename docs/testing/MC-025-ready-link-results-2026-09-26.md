# MC-025 ready-link candidate: physical results

Session date: September 26, 2026 (America/Los_Angeles; raw timestamps fall on September 27 UTC). Exploratory adjacent Galaxy S24 Ultra/SM-S928U1 and Samsung SM-X510 tablet, both Android 16/API 36, existing synthetic profiles and public `#general`. This is not the formal mixed-OEM/two-phone or larger-mesh gate.

## Candidate and conditions

Both devices were upgraded in place from app APK `a8951afe78cc0669bac9fde4ad9be251ed22ea6159c88f8622d8bcb78f90f7d2` to reviewed candidate `2bb9ab10c2fd9864e6c34ff622806fde38be10a39a249686a2578285e313953c`. Test APK: `5bd1de7f02a26d15c886ec673ceb1c78d9e92e23c4f267b54fa9d6eb4591f814`. Installed package hashes were read back on both devices. Source checkpoint: `0027adf`; production implementation reviewed at `dc4fc679dfdbec5ea0f6c06eaa270723258adb12`, prepared harness reviewed at `39c0e523fa9225408ebd6ff103a5c5a915653787`.

Phone OS build: `S928U1UES6DZH3`; tablet OS build: `X510XXSEEZG3`. Both USB powered, foreground and unlocked for round trips; the test temporarily keeps its activity awake. Initial phone/tablet battery readings: 87%/83%, 34.3°C/24.7°C. Radio environment is uncontrolled. These are Debug/instrumentation measurements, not battery-drain evidence.

Raw sanitized instrumentation, inventory, install and package-hash records remain ignored under `.work/mc025/2026-09-26-ready-link-physical/`. No identity/history reset was performed. The prepared tests use normal production admission, pacing, protected storage and radio behavior.

## Upgrade and recovery checks

| Check | Phone A | Tablet B |
| --- | --- | --- |
| Repeated Connect/protected reads | Pass | Pass |
| Initial Bluetooth off/on recovery | **Fail:** timed out after 30 seconds waiting for the Android adapter to become enabled; protected-data failure was not reported | Pass |
| Phone recovery retry | Pass | Not repeated |
| Existing-history read equivalence | Pass, three comparisons of 100 retained rows | Pass, three comparisons of 100 retained rows |

The first phone preflight ran two tests in 65.174 s with one failure at `BluetoothShutdownTest.kt:51`. A subsequent system-state query reported Bluetooth ON. The same phone recovery test then passed in the two-test recovery/snapshot run (4.678 s). Tablet initial preflight passed 2/2 in 38.226 s; its snapshot passed in 2.604 s. The timeout remains an unresolved intermittent recovery observation; a successful retry does not erase it or establish its cause. A later bounded Bluetooth-tag log query returned no matching diagnostics, so it does not explain the failure.

Snapshot comparison times, individual protected reads versus one batched read, were phone 90/77/78 ms versus 36/36/35 ms and tablet 138/136/141 ms versus 63/60/60 ms. These compare two API read strategies within this candidate; they do not independently prove migration crash safety, unchanged pre-upgrade identity bytes or a battery improvement.

## Predeclared latency experiment

Use [the prepared protocol](MC-025-prepared-device-runs.md): 60 request/reply pairs per initiator, alternating 30 short and 30 280-byte payloads. Ready means direct peer present and normal sender cooldown expired; the production public-channel refill remains one send per 12 seconds after its initial burst. Cooldown precedes the round-trip timer. Round trip includes both legs, processing and exact remote-echo history observation on the initiator's monotonic clock, sampled every 100 ms. It is not one-way radio latency.

Run `U01A` completed with both runners passing (phone 683.935 s, tablet 683.525 s), 60/60 observed pairs and no analyzer issues. The phone-initiated median target passes for both lengths. Run `U01B` also completed with both runners passing (phone 682.674 s, tablet 683.688 s), 60/60 observed pairs and no analyzer issues. Both orientations meet the predeclared median-below-1,000-ms target for both lengths: 120/120 round trips and 240/240 directed messages observed. Separate foreground/background follow-ups are recorded below; their setup failure is not removed from the session record.

| Run / initiator | Size | Delivered pairs | Median RTT | Nearest-rank p95 | Maximum |
| --- | --- | --- | --- | --- | --- |
| U01A / phone | Short | 30/30 | 627 ms | 1,473 ms | 1,676 ms |
| U01A / phone | 280 bytes | 30/30 | 643 ms | 1,509 ms | 2,381 ms |
| U01B / tablet | Short | 30/30 | 701.5 ms | 1,133 ms | 1,533 ms |
| U01B / tablet | 280 bytes | 30/30 | 684 ms | 1,120 ms | 1,578 ms |

These are complete declared populations, with no refused/missing samples removed. Sub-second typical latency does not mean every round trip is under a second. Raw results and the failure-aware analyzer output are retained together.

## Foreground/background follow-up

The separate untraced seven-pair smoke `S01` failed on both endpoints at the six-minute peer-discovery assertion, before any message request. Its seven scheduled pairs remain unattempted, not successful or silently excluded. Both devices were unlocked with the app foreground and Bluetooth ON; a bounded crash-tag query returned no matching output. Screens displayed a loading indicator, but a brief phone debugger attach showed the runner waiting for a peer and idle main/worker threads, not a demonstrated deadlock. Debugger and screen inspection perturbed this diagnostic run; it is not performance evidence. The app subsequently reopened normally with connection off on both devices. Cause remains unresolved.

After reopening the normal apps, fresh untraced `S02` passed 7/7 pairs (14 directed deliveries), including background reception, with both runner cleanups passing (phone 46.362 s, tablet 46.298 s). RTT samples were 752/1039/981 ms short, 1178/975/710 ms long and 718 ms background. This retry does not erase S01's setup failure.

Matching traced `T01` also passed all 7/7 pairs, both runner cleanups (phone 45.610 s, tablet 45.563 s), complete-frame attribution, same-clock partition checks and zero trace overflow. RTT samples were 722/1083/1092 ms short, 968/1056/724 ms long and 717 ms background. The three-sample traced short median was 1083 ms; trace results are not substituted for the larger untraced acceptance population or treated as a precise tracing-overhead estimate.

Every traced request and reply, including all 280-byte messages, used **one attributed native GATT frame per direction**. Request/reply fragment spans were zero. This directly confirms the negotiated-encoding benefit on this particular ready phone/tablet link; it does not establish a universal MTU or count controller/air retries. Aggregate transport counters still include control traffic.

| Traced mean component | Short (3 pairs) | Long (3 pairs) |
| --- | --- | --- |
| Request start to first submit | 305.0 ms | 340.0 ms |
| Responder receive to history observation | 245.3 ms | 208.3 ms |
| Reply dispatch/cooldown | 3.7 ms | 2.7 ms |
| Reply call to first submit | 99.3 ms | 107.0 ms |
| Initiator receive to history observation | 205.3 ms | 137.0 ms |
| Delivery/callback residual | 107.0 ms | 121.0 ms |

Each component uses one local clock; the residual is not pure airtime or one-way latency. Queue/work markers overlap these components and are not added again. Native submit calls took 0–2 ms. After stopping transport, five protected-read samples were phone 28/23/34/21/26 ms (median 26) and tablet 46/36/28/30/29 ms (median 30); native status queries were 0–1 ms. These measurements include each protected open/read/close path, not an isolated database or Keystore cost.

The session therefore retains 141 scheduled pairs across five runs: 134 observed pairs (268 directed deliveries) and seven unattempted pairs from S01's setup failure. The 120-pair ready-link experiment is a complete subset, not a claim that every session startup succeeded. Normal apps were reopened after all runners exited; connection was off on both and Bluetooth remained enabled. No device security setting, permission blanket grant or identity reset was used.

## Reproduction and validation

Use the exact candidate/test hashes above and the [prepared invocation arguments](MC-025-prepared-device-runs.md). U01A/U01B use `mode=acceptance` and initiator A/B respectively; S01/S02 use untraced `mode=smoke`, initiator A; T01 repeats smoke with `latencyTrace=true`. Run IDs are unique. Analyze acceptance with `tests/bench/android/acceptance_analysis.py` and completed smoke/trace runs with `tests/bench/android/latency_analysis.py`. Preserve S01's two failed runner logs and its explicit 7-scheduled/0-attempted outcome; the strict smoke analyzer cannot convert it into a successful result.

No implementation changed during this device session; existing native/build/security checks remain applicable to the installed artifacts. Report/ticket/link/whitespace checks and independent evidence review are recorded in the ticket. This physical evidence is not independent security certification.

## Open evidence

Physical acceptance, failed follow-up peer discovery, intermittent phone Bluetooth recovery, matched energy/idle/endurance workloads, minimum-OS security coverage, Mac/Xcode native validation and independent security assessments remain open. Both devices here are API 36 and cannot satisfy MC-043's API 29 cohort. No merge or full-gate completion is implied.
