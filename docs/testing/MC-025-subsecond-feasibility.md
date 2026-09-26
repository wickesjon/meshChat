# MC-025 sub-second ready-link round-trip target

The user selected **typical full request/reply RTT on a ready direct connection**. This is a performance target, not a promise for every message, busy link, reconnect or multi-hop path. Current measured production candidate is `3454acc`, APK `a8951afe78cc0669bac9fde4ad9be251ed22ea6159c88f8622d8bcb78f90f7d2`; its observed results remain in the [report](MC-025-results-report-2026-09-26.md#android-latency-optimization). No sub-second production RTT has been demonstrated.

## Remaining constraints

The [budget contract](../decisions/MC-007-budgets-and-acceptance.md#logical-rates-forwarding-and-fairness) requires one frame per second per link. `Relay.poll` implements this with `next_send = now + 1000`. The native bridge deliberately registers the scheduler with a 146-byte encoding ceiling, even though HELLO and ingress negotiate actual directional capacity. The measured 280-byte messages use three frames per direction: two one-second gaps in the request and two in the reply impose **at least four seconds** before application processing. This floor describes the current framing, not an inherent Bluetooth limit.

A compatible optimization to investigate is using the actual negotiated transmit capacity after HELLO, with a conservative bootstrap and unchanged frame/byte/crypto budgets. A long message that fits one admitted frame avoids the intra-message gaps. This depends on measured capacity in each direction; requesting MTU 517 does not establish it. Native capacity changes currently require generation teardown, so bootstrap-to-negotiated encoding needs explicit tests preserving queued-object framing, token/deficit credit, pending completions and stale-generation refusal. Do not reset/refill budgets to make a timing test pass. Shared-core changes require relevant Android and iOS consumer validation.

Storage remains a separate constraint. O02 ordinary protected reads cost median 389 ms on the phone and 543 ms on the tablet. Its two-endpoint receive-to-model interval still averages 4.829 s for short messages. Each production operation loads protected identity/key material, opens SQLCipher, performs bounded work and closes/clears resources. Repeating those costs cannot support a dependable sub-second RTT through the existing path merely by shortening polling.

## Storage direction and bounded prototype

SQLCipher's vendor identifies repeated key derivation on database opens as expensive and documents raw-key mode for already high-entropy random material. It is not appropriate for human passwords or low-entropy secrets. This motivates a hypothesis about the measured open cost, not proof that all current protected-read time is derivation. See [performance guidance](https://www.zetetic.net/sqlcipher/performance/) and [key API](https://www.zetetic.net/sqlcipher/sqlcipher-api/#PRAGMA_key).

`StorageKeyCostTest` uses fresh temporary synthetic databases to compare the pinned library's 64-byte random-passphrase path with a 32-byte random raw-key path. Both use the production `CipherConnection` PRAGMAs and native `EncryptedStore` open/read/close, five repetitions, wrong-key refusal and plaintext-marker absence checks. It omits Keystore/identity wrapping on both paths and never opens or migrates the installed profile. No connection persists between samples; synthetic keys are cleared and the invocation's scratch directory is removed. Results cannot establish production lock, reset, migration or full-RTT performance.

If the prototype supports the hypothesis, the production design must specify a versioned wrapped-key envelope and crash-safe migration for existing passphrase databases. A fresh random raw key does not open an existing passphrase database. A candidate migration should create a separately encrypted copy, validate it, durably commit the matching envelope/database pair, and retain a recoverable old pair until commit is established. Disk-full, interruption at every durable transition, wrong key, real lock, key loss, reset and downgrade behavior must be tested. This is a design direction, not an implemented or approved migration algorithm. Never log keys, use plaintext export, silently recreate history, lower password derivation work for user passwords, disable page authentication or retain unguarded handles.

## Prototype results

Both device invocations pass, including wrong-key refusal, absence of the synthetic plaintext marker in each database file, and scratch cleanup (phone 3.298 s; tablet 4.292 s). App APK remains unchanged; only test APK `53bebb15ac6d79aa6e32db322501e2fa885facdcf2bc3c9079bc4967218bd6c6` was installed.

| Open/read/close, milliseconds | Galaxy S24 Ultra | Samsung tablet |
|---|---|---|
| Random passphrase, five samples | 425; 420; 436; 415; 415 | 561; 541; 534; 545; 538 |
| Random raw key, five samples | 16; 13; 13; 13; 13 | 20; 20; 21; 20; 20 |
| Passphrase median → raw-key median | **420 → 13** | **541 → 20** |

These small fixed-order synthetic samples support pursuing the raw-key design while preserving encryption; they do not isolate every cryptographic subcomponent or include production wrapping/identity costs. The production profile was neither opened by the benchmark nor migrated, so its end-to-end RTT remains the previously measured O01/O02 result. The evidence makes a sub-second target plausible, not proven.

Build/lint passes in one minute using the recorded repository Android toolchain. Raw logs and APK hashes are retained in ignored `.work/mc025/2026-09-26-subsecond/`. A production implementation still requires storage migration/lifecycle validation and negotiated-frame interoperability checks. This feasibility checkpoint does not satisfy those gates or authorize a relaxed pacing/security invariant.

## Proposed acceptance experiment

After storage and negotiated-framing changes pass their applicable reviews and regressions, predeclare at least 30 short and 30 long full round trips per initiator role, on the same devices with normal maintenance/announcement work active. Use one monotonic clock for each full RTT, ordinary send cooldown and unique sample IDs. Record median, empirical p95, maximum, all scheduled attempts/refusals/timeouts, frame counts and traced versus untraced behavior. Treat **median below 1000 ms with every scheduled message accounted for** as the typical-latency target; loss/refusals cannot be silently omitted to improve it. Low-load ready-link results do not replace busy-link, multi-hop, security or battery gates.

Keep existing rates while measuring. If ordinary control traffic or a device's negotiated capacity still prevents the target, report that limitation before proposing a reviewed joint budget change. A faster app-level acknowledgement must not be substituted for the current exact remote model-history observation.
