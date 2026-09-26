# MC-025 Android physical acceptance packet

Status: procedure prepared; full matrix scenarios remain **NOT RUN**, apart from the limited startup preflight and two-device exchange smoke below. Initial candidate: `e20a05e4c55df857de2dc8168258138bf2092fbc`, the integrated main revision after MC-035. Record a new exact revision and APK hash if any candidate changes. This packet does not close MC-025.

## Phone/tablet exchange smoke — 2026-09-26

Candidate source: `0f5e0c4db9ff66380677e483bb240ed52d77b03e`. Both devices used Debug APK SHA-256 `61c4a84a7ec02490cb83c983422610edb07ab1e6631ccddadc0ecf5000bd0ce7`.

Device B is a Samsung SM-X510 tablet, Android 16/API 36, build X510XXSEEZG3, security patch 2026-07-05, arm64/4096-byte pages, with declared BLE support. ADB authorization, app installation and user onboarding succeeded. Initial battery reading was 36%, USB charging, battery temperature 31.6°C; this is not an MC-007 energy run. Device A is the S24 Ultra described below. A tablet supplies an additional physical BLE endpoint for exploratory exchange; it does not satisfy the required mixed-OEM phone cohorts or phone-specific energy/endurance criteria.

The operator used the existing synthetic profiles, foreground/unlocked apps and their visible Connect controls. Both screens reported one direct connection. The predeclared workload was one short synthetic public-channel message in each direction (two expected origin/recipient pairs). **Both unique messages were confirmed in the opposite device's rendered channel history**, rather than inferred from sender status or aggregate counters. Phone-to-tablet receipt was initially absent in an immediate UI sample and present in a later sample. Tablet-to-phone receipt was confirmed at host UTC 12:17:13. UI/keyboard transitions required repeating screen reads and tapping the still-unsent tablet draft after dismissing its keyboard; no extra unique message was introduced. No identity reset, history deletion, permission grant, lock-setting change or emulator runner was used.

Result: **PASS for this limited two-way UI exchange smoke only; stability finding remains OPEN.** The phone's app-scoped logs show repeated GATT server close/register cycles and Android framework `onServiceAdded` null-callback warnings while one direct connection remains visible. A later bounded 100-line app-radio snapshot still contained 14 server registrations; that is a log-snapshot observation, not a measured restart rate. Root cause, remediation and controlled retest are pending. Do not infer stable duplicate-link convergence, acceptable battery/load or successful recovery from the two received messages. Retain server-epoch/stale-callback protections when diagnosing this finding.

No per-egress attempts/completions/bytes, controlled topology, clock synchronization, radio latency, role reversal, fragmented-message or backpressure measurements were collected. MTU-request/callback logs are not proof of measured usable capacity. The full matrix and MC-007 denominators/thresholds remain required. Local inventory/install evidence is in `.work/mc025/2026-09-26-device-b-preflight/`; the declared plan, host send/receipt observations and bounded app-radio logs are in `.work/mc025/2026-09-26-two-device-smoke/`. Serials, raw logs and message content remain ignored rather than committed.

## Device A startup preflight — 2026-09-26

One physical Samsung Galaxy S24 Ultra (SM-S928U1), Android 16/API 36, build S928U1UES6DZH3, patch 2026-08-05, arm64/4096-byte pages is available. Installation and onboarding passed with the original MC-035 Debug APK `2f167a92e6aec3081182028f57a8df373715d1828fa340d6f2001579654294b0`. Connect then failed: the startup data read raced the newly started radio's clock and entered the protected-unavailable screen. This is an application failure, not passed OEM acceptance or evidence of invalidated keys.

The [active ticket](../ticketboard/inprogress/MC-025-android-bench-mesh-and-early-field-gate.md) records the scoped startup-monitor fix. The original APK fails the focused physical regression (9.874 seconds); updated Debug APK `61c4a84a7ec02490cb83c983422610edb07ab1e6631ccddadc0ecf5000bd0ce7` passes three Connect/protected-read/stop cycles (76.843 seconds total), preserving the existing nickname, avatar and channel names/IDs. Test APK: `705bca39aaff601ae15b5c4bec5ba5889745cbccc0a94e34e70fef2e5ae088be`. Source is the MC-025 branch's startup-fix checkpoint; validation/review status is recorded in the ticket. Local logs: `.work/mc025/2026-09-26-device-a-preflight/connect-before.log` and `connect-after.log`.

Invoke only the explicit test class with instrumentation arguments `physicalConnect=true` and `class=org.meshchat.ui.ConnectStartupTest` on an already onboarded synthetic profile. It uses real protected storage and Bluetooth, keeps the screen awake only during its run, restores the selected channel, and stops the radio on exit. It does not reset identity, clear history, grant permissions or change lock settings. An authorized device serial is required when invoking ADB; do not alter emulator runners to accept it.

This covers startup with one unlocked, USB-powered phone only. There were no peer delivery, capacity, backpressure, lock-transition, background/OEM, battery or endurance measurements. All remaining scenario requirements below are still mandatory.

The [ticket](../ticketboard/inprogress/MC-025-android-bench-mesh-and-early-field-gate.md) owns the Android radio/OEM/Beacon bench gate before beta. [MC-007](../decisions/MC-007-budgets-and-acceptance.md) defines workloads, accounting and thresholds; the [physical scheduling policy](../decisions/local-validation-policy.md#physical-acceptance-scheduling--approved-2026-09-14) preserves them. Use synthetic identities, credentials and messages until MC-043 permits sensitive-data use. No physical key/storage result is inferred from these radio trials.

## Before a run

1. Inventory the named devices: manufacturer/model, Android/API/build/security patch, battery health, secure-lock configuration, Bluetooth capability and native page size. Assign disposable labels A–J; keep serials in ignored local evidence. Start with two phones; retain Pixel, Samsung and Xiaomi coverage and the required five-/ten-phone cohorts.
2. Record source revision, production APK SHA-256, build variant and exact build/tool versions. Confirm native checks for that revision, required permissions and a working explicit protected start. Any missing native check remains a blocker.
3. Declare topology and actual measured adjacency, roles, directional capacities, workload/seed, traffic interval, run duration, expected reachable pairs, clock synchronization/uncertainty and measurement method **before** traffic. Physical placement alone does not establish an isolated edge. Record interference and inability to enforce a graph.
4. Verify the workload sender/receiver and per-egress tracing against the production core/driver, including attempts, completions, retries and bytes. Record trace/tool revision and instrumentation overhead. Existing contribution totals cannot establish end-to-end delivery, unique people, over-air bytes or per-link latency. If needed instrumentation is absent, record NOT RUN rather than substituting aggregate estimates. Any added instrumentation requires scoped implementation and review before use.
5. Use only disposable test data. Keep generated APKs, credentials, captures and raw traces in ignored `.work/mc025/<run-id>/`; retain durable aggregate results and relevant limitations under `docs/testing/`. Never commit keys, provisioning QR images or message content. Existing emulator scripts reject phones; preserve that guard and prepare an explicit physical-device procedure/tool instead.
6. Inventory and record device-specific permission/OEM settings as found. Any requested changes are part of an explicitly recorded run configuration, not a hidden workaround. Do not disable protected locking to obtain screen-off or battery success.

## Required scenario matrix

Every row is NOT RUN until attributable physical results exist. Record PASS, FAIL, BLOCKED or NOT RUN per device pair/configuration; a failure requires diagnosis and retest, not a narrower denominator.

| ID | Procedure | Required observation and acceptance source |
|---|---|---|
| A: two-phone framing | Swap central/peripheral roles; send whole and fragmented traffic both ways. Measure write and notification capacity after actual negotiation/subscription. Exercise the 146-byte admission floor and explicitly refused unsupported cases when the peer/tool permits them. | Actual submitted/completed GATT values, frame/byte counts and validated receipts; no readiness inferred from a requested MTU. [MC-023](MC-023-android-gatt-driver.md). |
| B: callback recovery | Exercise CCCD enable/disable/refusal, busy/error/stalled completions, MTU change, disconnect/reconnect and stale callbacks with a controlled physical peer. Label injected faults separately from naturally observed radio faults. | No stale-generation effects, unlimited buffering, ambiguous retry or credit reset; explicit failure/recovery and measured backpressure. Record unavailable fault controls as coverage gaps. MC-023 and [MC-024](MC-024-android-connection-policy.md). |
| C: OEM lifecycle/policy | Pixel/Samsung/Xiaomi: permissions and revocation, location restrictions, Bluetooth off/on, foreground/background, screen-off, lock/unlock, OEM service termination and authorized restart. Measure duplicate-link convergence, slots/reconnection, isolation scanning and power thresholds. | Honest degraded states; retained budgets and actual supported link counts. Locked-device shutdown and whole-server replacement are known implementation constraints to measure, not permission to bypass protection. MC-024. |
| D: two/five/ten-phone mesh | Predeclare controlled topologies including paths, cycles, dense neighborhoods and bridge/corridor cases. Execute MC-007 workload and seeds 7/19/43 where applicable; compare production results with the same declared simulator configuration. Include a Saver/tier-zero bridge and out-of-TTL endpoints. | Report all scheduled TTL-reachable origin/recipient pairs; never discard admission refusals. The lossless named live gate is 100% reachable delivery and p95 ≤30 s; beyond seven edges receives no live copy from that origin. Preserve scenario-specific conditions; uncontrolled radio loss cannot be labeled lossless. MC-007. |
| E: late join and SYNC | Run simultaneous bidirectional selected-set catch-up on one ready lossless Normal link at C=146, concurrent ANNOUNCE/REACTION workload, pagination, reconnect and stale-response attempts. Repeat on actual supported capacities and the integrated bridge setup. | Up to eight items/8192 encoded logical bytes plus terminal marker within 120 s of initial request's first frame for the named gate. No false completion or stale-link binding. This is not full-cache replication. Use final valid crypto fixtures for authenticated cases. MC-007/MC-015. |
| F: churn and abuse | Reproduce declared outage/loss cohorts and run 600-second identity-rotating/flood workloads; include invalid-first/valid-later and bounded queue/credential/key/orphan pressure. | Actual node/link/crypto/memory bounds, no rejected-frame display or unverified trust, no TTL-zero relay or encrypted-to-clear fallback. Keep the static reachable denominator and report outage cohorts separately; no delivery-rate promise under intentional flooding. MC-007. |
| G: phone Beacon | Six real wall-clock hours powered, with production traffic and periodic memory/cache/queue measurements. Exercise manual unplug above 30%, manual downgrade at ≤30%, auto charge/unplug and restart/exit. Run sparse-gap trials with beacon, ordinary phone and no bridge, three paired repetitions. | No manual recovery during endurance; bounded state, preserved core/history/credits on transitions, correct powered hint and actual coverage/relay/load/battery differences. No assumed offload benefit. [MC-033 procedure](MC-033-beacon-mode.md). |
| H: energy | Predeclare paired screen-off runs per MC-007: 15-minute warm-up, four hours, 60–90% starting battery, 20–25°C ambient, same health/OS/build/radio configuration, idle-app control and three repetitions per mode/device with randomized order. Separate fixed-brightness screen-on measurements. | Raw and incremental percentage-point slope/hour, supported energy measurements, held/discovered links, GATT work and delivery. Normal ≤5 points/hour, Saver ≤2, alongside the named delivery gate. A stopped/suspended app cannot pass by low drain. Powered Beacon reports energy and traffic. MC-007. |
| I: early field trial | Execute a small integrated Android field trial before MC-034; document actual site, spacing, participant/device count, observed graph/interference, permissions, duration and synthetic workload. | Compare delivery, latency, relay cost, churn and power with the declared bench/simulator cases. List mismatches and resolved deviations. This is not the later 30–50-device MC-038 field gate. |

MC-007's full named scenarios, accounting and bounds remain authoritative; the matrix is a traceability aid, not replacement thresholds. Record suppression against the same workload/topology with the approved baseline and report actual reduction including zero. Any baseline requiring code changes must be reviewed and identified as a test configuration. Never infer battery savings from simulator send counts.

## Copy for each run

```text
Run ID / scenario / evidence state: NOT RUN
Candidate revision / APK SHA-256 / build variant:
Operator / UTC start and end / wall-clock duration:
Devices, OS builds, battery health and page sizes:
Roles / native TX-RX capacities / notification readiness:
Permissions / OEM configuration / foreground-screen-lock state:
Topology / measured adjacency / TTL-reachable pair denominator:
Workload, seed, traffic sizes/rates / baseline / repetition order:
Instrumentation revision, clock uncertainty and overhead:
Battery start/end, warm-up, temperature, control and power source:
Scheduled / refused / validated unique pairs / delivery ratio:
Latency p50/p95 with undelivered count:
GATT attempts/completions/retries, frames/bytes by origin-relay-SYNC-control:
Per-node memory/queue/cache/work peaks / held-discovered links:
Raw drain slope / incremental drain / instrumented energy if available:
Failures, unavailable observations, simulator differences and deviations:
Raw evidence paths / aggregate report / reviewer / retest reference:
Result against named acceptance criteria: NOT RUN
```

## Completion prerequisites

Hardware inventory and candidate-specific instrumentation are still pending. Resolve failed or unmeasurable scenarios with scoped fixes and physical retests; an acceptance change requires an explicit decision. Populate every ticket criterion with device/OS/duration and evidence, run applicable checks, publish a ready PR and obtain the required separate Terra review. Keep this ticket in progress until its physical criteria pass; no preparatory PR or emulator result may mark it complete or unblock beta.
