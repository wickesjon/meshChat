# MC-025 paired app measurement

This explicitly opted-in physical test uses the normal installed synthetic profile, protected storage, model send path, production Bluetooth service and admission limits. It does not grant permissions, reset identity/history/counters, replace radio ports or modify device power/security settings. The user must unlock both devices normally. It is exploratory phone/tablet evidence, not the formal two-phone or MC-007 lossless gate.

Build `:app:assembleDebugAndroidTest` using the repository Android environment. Install only the resulting test APK alongside the unchanged production candidate on each endpoint. Run `org.meshchat.ui.PhysicalRoundTripTest` through `org.meshchat.app.test/androidx.test.runner.AndroidJUnitRunner` on both endpoints concurrently, with arguments `physicalMeasure=true`, the same unique alphanumeric `runId` (1–16 characters), and `role=A` / `role=B`. Never reuse a run ID: retained history must not satisfy a fresh receipt assertion.

The test starts the real nearby service and selects the public test channel. Setup allows six minutes for ordinary discovery/cooldowns and waits another six seconds before taking baseline counters. No limit is bypassed. KEEP_SCREEN_ON applies only to the foreground test activity and is cleared on exit. The responder is briefly backgrounded for the last pair; a real device lock aborts the test. The test requests transport stop, waits for the stopped state, and leaves activity closure to the test runner. Reopen the normal app after both test processes exit.

## Predeclared workload

- Seven request/echo pairs: indices 0–2 short ASCII; 3–5 exactly 280 bytes per direction; 6 short with B's activity stopped/backgrounded.
- Fourteen expected directed message deliveries. A requests once per index; B echoes each received index once. There is no automatic resend of a missing/refused sample.
- A observes the normal send cooldown before attempting each request. Each requested echo has a 60-second observation limit. Timeouts and refused sends stay in the denominator.
- B records exact remote request reception, sends through the same normal composer path and checks local native completion. Index 6 additionally asserts its activity is below STARTED when received.
- A measures from immediately before request logging/send enqueue to observing the exact remote echo in its model history, using `SystemClock.elapsedRealtime` on A for both endpoints of the interval. No cross-device clock subtraction is used.
- State polling is 100 ms, plus main-thread dispatch/host-log overhead. This is app-observed round-trip time including both legs, processing, queueing, refresh/observation and logging. It is not one-way latency, PHY latency or time to final UI painting. Do not divide it by two.
- Compare beginning/end production contribution snapshots without resetting them. Received frames count core intake; scheduled frames count native scheduling; completed frames/bytes count successful native completion callbacks. They include control traffic, not only the synthetic messages. They do not expose every controller/air retry or prove remote delivery.
- Record host-side battery/temperature/PSS snapshots and power source separately. USB-powered, foreground Debug instrumentation cannot measure incremental battery drain or satisfy MC-007 energy acceptance.

`MC025` JSON records in instrumentation stdout retain run ID, role, monotonic/UTC observation times, sample index/size, outcomes and counters. They deliberately omit payload, profile identity and keys. Keep raw logs and installed APK hashes in ignored repository-local paths. Preserve harness failures separately from valid measurement runs. Report p50/p95 only with sample count, estimator, population and loss/refusal counts; with three samples per length, nearest-rank p95 is simply the maximum and is not a reliable tail estimate.

This does not establish missed-message catch-up. A reproducible SYNC test requires a declared selected cache set and supported production request path. The app refuses offline sends without a peer; changing that behavior or injecting cache entries would be a different test configuration that must be identified and reviewed.
