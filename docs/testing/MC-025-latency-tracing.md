# MC-025 physical round-trip timing trace

This is exploratory attribution for the [paired measurement workload](MC-025-paired-measurement.md), with the same seven request/echo pairs, normal composer path, admission limits and existing protected owners. It does not modify the production APK, retain protected handles or change device security/power settings. Raw logs and derived JSON belong in ignored repository-local evidence directories; no payload, key or profile identity is emitted.

## Procedure

1. Record production/test APK hashes, devices, app candidate and run IDs. Unlock both devices normally. Install only the test APK alongside the recorded app.
2. Run `org.meshchat.ui.PhysicalRoundTripTest` concurrently on A and B through the normal Android test runner, with `physicalMeasure=true`, a fresh shared alphanumeric `runId` and respective `role=A` / `role=B`. This first run omits tracing.
3. Both endpoints wait for ordinary peer discovery, terminal initial bounded catch-up and six seconds of settling. The predeclared workload is three short foreground pairs, three 280-byte pairs and one short pair with B's activity below STARTED. Preserve refusals, missing echoes and runner/cleanup failures.
4. After both runners finish, repeat with a new run ID and `latencyTrace=true`. Use the same installed packages and workload. Retain history; distinct IDs prevent stale receipts satisfying assertions.
5. Analyze each pair of logs with `python -B tests/bench/android/latency_analysis.py A.log B.log output.json`. The analyzer requires both passing runners, all seven pairs and, for tracing, complete accepted frame sets and timing markers. It rejects gaps, duplicates, negative intervals and overflow instead of silently dropping samples.
6. Reopen normal apps after both runners exit. Report small-sample differences without treating them as an optimization or a precise estimate of tracing overhead.

## Observations and boundaries

`PhysicalLatencyTrace` attaches through the model's existing serialized queue and core monitor. It wraps the real GATT callbacks and protected-egress function, preserving original arguments, return values, authorization and submission. Synthetic sample correlation exists only in memory. At most 4096 content-free entries are buffered; overflow fails attribution. Trace JSON is written after traffic stops, avoiding per-frame trace output during measurement. The extra callbacks, correlation and queue markers still perturb execution.

The initiator measures the complete round trip on its monotonic clock. Each attributed interval also uses only one device's clock:

| Component | Local interval |
|---|---|
| Request preparation | A request start to first native frame submission |
| Request fragmentation | A first to last frame submission |
| Responder processing/observation | B complete structurally admitted receive callback to exact model-history observation |
| Reply dispatch/cooldown | B model observation to composer send call |
| Reply preparation | B send call to first native frame submission |
| Reply fragmentation | B first to last frame submission |
| Initiator processing/observation | A complete receive callback to exact echo model-history observation |
| Residual | Full A round trip minus the seven local intervals above |

The residual combines two delivery legs, platform callbacks, any waiting/admission before the observed receive callback and timing uncertainty. It is neither pure Bluetooth airtime nor measured one-way latency. Clock offsets cancel because timestamps from different devices are never subtracted directly; clock-rate differences and millisecond resolution remain limitations. Model observation includes nominal 100 ms polling and variable main-thread dispatch; final UI painting is not measured.

Additional markers bracket queued send and receive work. Queue wait measures delay until the preceding marker executes. Work brackets can include interleaved production tasks and are not exclusive method execution times. These diagnostics overlap the round-trip partition and must not be added to it again. Per-frame protected-egress entry-to-submit and native-submit-call durations distinguish observable authorization/lock waiting from the submission call, without measuring controller completion or air retries.

After transport stops, five ordinary protected setting reads and five native status queries provide local baselines. A protected read includes its existing Keystore, encrypted-database/native-store open, read and close path; it does not isolate a SQLCipher derivation, database query or any individual subcomponent. Returned settings are discarded without logging. Protected resource lifetimes and cleanup remain unchanged. This comparison can identify an expensive path worth profiling; it cannot alone prove that path exclusively causes a particular packet delay.

No battery-drain, multi-hop, range, screen-off endurance or formal MC-007 result follows from these USB-powered phone/tablet runs. Results and failed attempts belong in the [results report](MC-025-results-report-2026-09-26.md) and active MC-025 ticket.
