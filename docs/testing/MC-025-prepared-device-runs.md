# Prepared latency, idle-work and battery runs

Status: harnesses prepared; **no device run or performance result is claimed**. These extend [PR #39](https://github.com/wickesjon/meshChat/pull/39). The [current report](MC-025-results-report-2026-09-26.md) keeps the measured earlier candidate separate. Production pacing, polling, encryption, key lifecycle and radio policy are unchanged by this preparation.

Local preparation validation: Android test APK/lint pass, 26 app JVM tests pass, and 16 Python analyzer tests pass, including missing/truncated/mixed logs, failed requests and unavailable energy evidence. Test APK SHA-256: `80908a1745d8ab390e0f7b35ac0b2763cb25bbb67bcdc7fde821efb6608f861a`. This validates code/build behavior; it does not demonstrate that the new physical workloads run successfully on a device.

## Before testing

Use synthetic profiles, preserve existing identity/history, and record model/OS, app and test APK hashes, date, screen state, power source, battery temperature, nearby devices and workload. Keep raw output under a fresh ignored `.work/mc025/<run>/` directory; never reuse run IDs. Build `:app:assembleDebugAndroidTest :app:lintDebug :app:testDebugUnitTest` with the repository toolchain. Install a matching app/test candidate only when device testing is authorized. Do not run an old installed app with a newly compiled incompatible test APK.

The acceptance harness does not install APKs, grant permissions, reset profiles or bypass send limits. Existing emulator-only security runners must remain emulator-only. These instructions do not initiate any hardware work now.

## 1. Full request/reply latency

Start `org.meshchat.ui.PhysicalRoundTripTest` through `org.meshchat.app.test/androidx.test.runner.AndroidJUnitRunner` on both endpoints concurrently. Supply the same arguments except `role`:

```text
physicalMeasure=true mode=acceptance initiator=A runId=<unique> role=A
physicalMeasure=true mode=acceptance initiator=A runId=<same unique> role=B
```

Each run schedules 60 request/reply pairs: 30 short and 30 280-byte messages, alternating lengths. Keep both devices unlocked and foregrounded. The harness waits for a ready direct link and initial catch-up, then uses the ordinary production send cooldown. Normal announcements/maintenance remain active. An unavailable-ready state is recorded and does not bypass admission. A failed echo is retained; no retry is silently substituted for that sample. A crash/setup failure leaves missing slots visible in analysis.

For example, execute each endpoint's invocation in its own terminal, capturing output into the fresh run directory:

```text
adb -s <device-A> shell am instrument -w -e physicalMeasure true -e mode acceptance -e initiator A -e runId <unique> -e role A org.meshchat.app.test/androidx.test.runner.AndroidJUnitRunner
adb -s <device-B> shell am instrument -w -e physicalMeasure true -e mode acceptance -e initiator A -e runId <same-unique> -e role B org.meshchat.app.test/androidx.test.runner.AndroidJUnitRunner
```

Under repeated failures, the responder can wait up to about 96 minutes after setup. A cancelled run is incomplete evidence, retained by the analyzer; do not relabel it passed.

Repeat using a **fresh run ID and `initiator=B` on both endpoints**. Roles A/B identify the physical devices; they do not swap in the log filenames. Run untraced trials first. Separate `latencyTrace=true` repetitions provide sample-attributed outgoing frame counts and queue timing; tracing overhead is not silently mixed into untraced latency. Untraced native counters include control traffic and are not per-message frame counts. The bounded trace refuses overflow/incomplete frame attribution. Remote history observation is sampled every 100 ms; report that timing resolution.

Analyze each orientation, retaining both successful and failed instrumentation logs:

```text
python -B tests/bench/android/acceptance_analysis.py .work/mc025/<run>/A.log .work/mc025/<run>/B.log .work/mc025/<run>/summary.json
```

The analyzer lists all 60 scheduled slots, readiness/refusal/timeout/interruption/missing-peer outcomes, successful-only median/nearest-rank p95/max, and traced frame counts where complete. Both endpoint summaries and all scheduled pairs must pass before the orientation can meet the sub-second median target for both sizes. A missing endpoint prevents a pass; mismatched run IDs, duplicate records or incompatible plans are rejected. Both orientations are required for the overall experiment. Report every run, including failures; do not select only a favorable run. No one-way radio latency is inferred by subtracting device clocks.

The historical seven-pair workload remains the default `mode=smoke, initiator=A`, including its final background pair. Its existing `latency_analysis.py` output and old results retain their meaning.

## 2. Idle work and energy observations

Run `org.meshchat.ui.PhysicalResourceTest` through the same app test runner with:

```text
physicalResources=true runId=<unique> workload=<name> durationSeconds=<60..21600>
```

This observer deliberately does not start/stop radio, send messages, change Beacon settings, wake the display or keep it on. Instrumentation may restart the app process: launch/reopen the normal app and establish the declared workload during its three-minute readiness wait. Start measurements only after normal stabilization/catch-up. Prepare each workload in the app:

| Workload | Preparation | What it isolates |
|---|---|---|
| `disconnected` | Nearby connection off; Beacon/Auto Beacon off; keep the same channel view/screen state as the matched comparison. | App/UI observation baseline. |
| `connected_idle` | Exactly one direct peer, no test message sender, catch-up settled. | Ordinary maintenance/announcements plus history refresh. |
| `messaging` | Exactly one peer plus a declared synthetic sender cadence on the other endpoint. Record payload size, duration, offered/accepted/observed counts separately. | Incremental work under traffic; radio counters alone do not establish delivery. |
| `beacon` | Explicit powered Beacon Mode; use a declared peer count/traffic pattern, maintain external power. | Powered CPU/memory/relay endurance, **not battery drain**. |

Use 10-minute trials for preliminary idle CPU/task-queue comparison, and at least 30 minutes (prefer longer repeated runs) for unplugged battery-gauge observations. The existing MC-007/025 power and six-hour powered endurance gates retain their required durations, topology and thresholds. These small comparisons do not replace them. Run at least three matched repeats per condition/candidate, alternate their order, keep ambient/radio/screen conditions alike and record other device activity. Do not use artificially faked battery state or charging-disabled shell settings. Arrange a real unplugged logging connection before an energy trial; a USB-powered trial cannot measure drain. Establish any wireless debugging through the user's normal device controls, record its overhead consistently, and do not change host/device security settings automatically.

Samples every 60 seconds contain process CPU time, sampled process memory, completed/pending model tasks, transport counters, battery charge counter/percentage, temperature, power source, screen-interactive state and whether the workload still matches. APK SHA-256/model/API accompany the plan. CPU includes the observer; queue counts do not count SQLCipher opens or attribute individual methods. This is a way to locate avoidable idle work before proposing a production scheduling change. Use the existing separate latency tracing to investigate message-path delays. Memory maxima are sampled, not absolute peaks, and short transitions between samples can be missed.

The instrumentation lifecycle may close the app when it exits; reopen it normally afterward. A resource run does not prove uninterrupted operation after test teardown.

```text
python -B tests/bench/android/resource_analysis.py .work/mc025/<run>/resources.log .work/mc025/<run>/resources.json
```

Incomplete runs, counter resets, workload/screen/power changes or sample gaps are reported as unsuitable for matched comparison. Missing charge-gauge support remains unavailable; it is not zero drain. Only complete, stable, unplugged runs of at least 30 minutes with valid non-increasing charge/percentage readings produce whole-device charge-loss rates. Those rates include screen, OS and other apps; do not label them app-only energy, watts or a proven battery improvement. Powered Beacon, gauge jumps and insufficient duration produce no battery-drain claim.

## Later execution order

1. Verify preserved-profile upgrade and Connect/recovery on the new candidate.
2. Run both untraced latency orientations, then separate traced runs and classify all failures.
3. Compare disconnected/connected-idle CPU/task counts before deciding whether maintenance or view refresh should change.
4. Run matched unplugged messaging/idle battery trials, followed by separately powered Beacon endurance.
5. Attach sanitized evidence and hardware limitations to MC-025. Prepare MC-043's separate key/storage security matrix; emulator evidence and this observer cannot certify it.
