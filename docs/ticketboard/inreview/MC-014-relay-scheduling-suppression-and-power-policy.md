---
id: "MC-014"
title: "Relay scheduling, suppression and power policy"
depends_on: ["MC-013","MC-012"]
kind: "core"
branch: "ticket/MC-014-relay-scheduling-suppression-and-power-policy"
---

# MC-014 — Relay scheduling, suppression and power policy

## Objective

Implement TTL semantics, per-egress knowledge and default-to-relay when no coverage evidence exists.

## Dependencies

`MC-013`, `MC-012` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/core/**`, `tests/simulator/**`, `tests/integration/relay/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Implement TTL semantics, per-egress knowledge and default-to-relay when no coverage evidence exists.
- Schedule priorities, bounded own-message admission, hold-offs, fairness, disconnect cleanup and frame-preserving flushes.
- Implement Normal/Saver/Auto and beacon policy parameters without turning advisory peer or infra hints into trust.

## Exit criteria

- [x] TTL-reachable chain and cut-vertex scenarios meet MC-007 delivery thresholds.
- [x] Dense results report reduction relative to the baseline with actual GATT sends and measured power-tier work shares.
- [x] Own-message overflow gives a visible admission failure; no queue exceeds its packet or byte cap and control traffic cannot starve indefinitely.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If probabilistic thinning adds no benefit under coverage-only suppression, omit it and retain deterministic per-egress suppression.
- Under power pressure shed relay work according to budget and expose degraded operation; do not refresh TTL or claim guaranteed delivery.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Implementation started from main `bb0d92954b1d9accb34fece2da1236c23b9b97e4`, with MC-013 and MC-012 complete. The wire contract and dependencies are unchanged. Source review and final check evidence remain pending.

### Core implementation and consumer contract

`src/core/src/relay.rs` owns 128 fixed outbound slots (32 per link), borrowed admission before payload retention, actual MC-010 encoding, one-frame/second/link pacing, node/link frame+byte accounting, Normal/Saver forwarding buckets and finite Beacon aggregate limits. All GATT attempts, including the one allowed native-failure retry, consume work credit before a native call; no refunds follow failures. A private attempt token binds completion to its instance/generation/attempt. Capacity changes require disconnect and fresh registration. Node buckets survive disconnect and mode changes; downgrades clamp capacity and upgrades grant no new credit.

The five classes use frame-cost deficit round robin with quanta 8/16/8/4/8 and a 32-credit cap. A class uses its remaining deficit before rotating; every native retry debits that class and bounded debt is retired before later objects. SYNC/ANNOUNCE alternate within their shared class. Global scheduling rotates eligible links. Fragments stay contiguous and in index order. Unstarted and started objects have separate absolute 30-second deadlines that never refresh on progress/readiness/retry. Failure, disconnect, eviction, coalescing, suppression and native completion synchronously emit bounded `ResultEvent` callbacks; enqueue refusal returns an explicit error. Native completion is not a delivery receipt. A native consumer must surface own failures and abort a served walk on eviction/failure, and must invoke completion even when its send fails. The callback contract avoids a hidden unbounded event queue.

Overflow evicts oldest unstarted forwarded REACTION, then forwarded CHAT/control/unknown, then served SYNC; it preserves own and started work. ANNOUNCE coalesces only before its first attempt. Queue exhaustion refuses new own admission explicitly. Actual reserved-capacity accounting includes object/link metadata and allocator capacity; no per-packet heap allocation occurs in the scheduler.

`observe` accepts only complete, structurally valid ingress-admitted bytes (including duplicate copies) or admitted local origins. It records exact variants and per-link knowledge in 200 recent entries. Direct observations apply only to that egress peer and immutable variant; a received ANNOUNCE digest is an advisory ID-based filter with 60-second expiry. Digest hashing uses the specified salt, six positions and bit order. No coverage evidence means forward when budgets permit; all current egresses must be independently covered to suppress all copies. TTL clamp/decrement uses the production codec exactly once on received bytes. A received TTL 1 does not forward; TTL 0 is invalid live input. No extra probabilistic thinning is introduced, consistent with MC-007's approved decision.

Clear REACTION admission retains 512 bounded target counters for 15 minutes and permits at most 30 reserved variants per target. One reservation covers all egress copies of a recent exact variant; eviction/failure does not refund it, and lost recent membership is conservative. The ciphertext target of an encrypted reaction remains opaque; addressed decryption/orphan/application reaction state stays with its crypto/product owners. No trust or authenticated-display decision depends on relay coverage.

`src/core/src/power.rs` implements Auto's local battery/visible-peer table, 60-second continuous-condition hysteresis, forced modes, Android-only Beacon, its unplugged 30% exit, infra indication only for a charging Android Beacon and native scan/ANNOUNCE/link-limit parameters. Received peer-count or infra claims do not grant authority. The native connection owner explicitly tears down excess links before lowering the scheduler ceiling; driver integration remains MC-023/024/026 and UI Beacon integration MC-033. ANNOUNCE production, HELLO/session/credential state and actual crypto operations remain those consumers' work. This core exports no new UniFFI surface and does not claim native radio, battery, physical key protection or independent security acceptance.

### Validation

Host: Windows x86_64, Rust/cargo 1.85.1, Python 3.14.4. Repository-local toolchains/cache/temp paths; locked dependencies, unchanged root and simulator lockfiles.

- `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings`; `cargo test --workspace --all-features --locked --offline` and `--release`; `cargo build --workspace --all-features --locked --offline --release`: pass. Logs are retained under `.work/mc014/`. The 14 relay integration tests cover TTL/peer isolation and variants, encoder/reassembler at C146/182/512, pacing/contiguity/retry, stale callbacks, queue refusal/eviction/caps, absolute deadlines, weighted deficits/fairness, SYNC/ANNOUNCE alternation/coalescing, advisory filter expiry/omission, reaction-target reservations, node credit across reconnect, 600-second forwarding pressure, stable link rotation and Auto/Beacon thresholds.
- The 600-second three-link Saver flood offers new forwarded CHAT every 100ms/link. Every attempted send remains spaced at least one second per link and the total remains within 40 + (2/3)*600 = 440 mode frames; all queue reservations stay bounded. API failures are explicit. This measures admitted work, not battery consumption.
- Simulator formatting/clippy/release build and the existing 15 driver regression tests pass. `--core-relay` uses actual core ingress, TTL/coverage, queueing, DRR, pacing, mode budgets, native completion and reservation counters. Python supplies topology/time, synthetic origin fixtures and simulated native GATT outcomes. The baseline disables only hold-off/coverage; admission, TTL, queues, loss and budgets remain identical. Unacknowledged air loss is distinct from native failure/retry. No production SYNC/session/authentication or physical radio result is claimed.
- MC-007 scenario-definition/worksheet checks, ticketboard validation (46 tickets/127 dependencies), all 12 board tests and `git diff --check` pass. Cargo-deny 0.20.2 advisory/bans/licenses/sources checks pass with a freshly fetched RustSec database; only unused license-allowance warnings remain.

The final development suite `.work/mc014/final-development/metrics.json` contains 108 reports / 216 executions (18 cases × seeds 7/19/43 × coverage/unsuppressed policies × exact rerun). All required lossless cases have 100% reachable delivery and no beyond-TTL delivery. Maximum p95 across seeds: chain 18,599ms; cycle 11,765ms; barbell 9,705ms; bridge 11,970ms; Saver bridge 12,268ms; dense and mixed tiers 5,020ms, all below 30 seconds. K6 forwarded-attempt ratio is 0.80 against the identical unsuppressed baseline (720 vs 900 forwarded frames, 20% reduction). Loss/outage, late join, capacity refusal, version rejection and backpressure retain missed scheduled pairs in their denominator; no impossible recovery result is claimed.

Mixed-tier K6 assigns nodes 0/1 tier 3 and nodes 2–5 tier 1 through the existing synthetic power-event input. Each node makes 330 total GATT attempts and 120 forwarded attempts: tier 3 accounts for 660/1980 total and 240/720 forwarded attempts, with equal per-node work in this workload. This measures no per-node load-shifting benefit and no battery saving; equal results are retained. Normal/Saver forwarding counts and per-tier total attempts come from the production scheduler. Peak K6 link/node queues are 2/10 objects. On this x86_64 build, outbound reservation is 149,528 bytes node /37,376 per-link slots; recent state 11,224 bytes; target state 16,408; link state 3,096; manager 280. All remain below MC-007 limits. Final clean-source provenance is recorded after committing this source.

This is an internal core and simulator change: no native ABI/export, native source, build script or dependency changes. Under the approved local validation policy, host component checks apply; Terra must confirm this rationale for native-job applicability. Relevant hosted results, exact-source simulator metrics and actual source/final review references are recorded before merge.

## Review and merge

- Branch: `ticket/MC-014-relay-scheduling-suppression-and-power-policy`.
- Review/PR: [PR #18](https://github.com/wickesjon/meshChat/pull/18); actual reviews recorded below and in the PR.
- Squash commit title: `MC-014: Relay scheduling, suppression and power policy`.
- Completion becomes effective only when the reviewed squash commit lands on main.


### Source review and correction

Separate Terra medium [review 5208448417](https://github.com/wickesjon/meshChat/pull/18#pullrequestreview-5208448417) at `5d5b614ba3a3993cfe7574c55cfa7d705b56840c` found one blocking defect: `infra` was set for charging Normal/Saver phones, contrary to the §15.7 table. It now requires Android Beacon mode plus external power. A regression checks every platform/mode/charging combination and Auto-Beacon entry/power removal. The same review independently passed 13 existing relay tests. Follow-up review of this fix remains required before merge.

The initial clean-source suite at `5d5b614ba3a3993cfe7574c55cfa7d705b56840c` reproduced all 216 executions with `source_dirty=false` and exact reruns in `.work/mc014/5d5b614-clean/`. These simulator results do not exercise the corrected infra hint and are not evidence that the earlier hint was correct; the new regression specifically checks that behavior.
