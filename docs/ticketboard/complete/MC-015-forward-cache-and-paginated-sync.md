---
id: "MC-015"
title: "Forward cache and paginated SYNC"
depends_on: ["MC-014","MC-007"]
kind: "core"
branch: "ticket/MC-015-forward-cache-and-paginated-sync"
---

# MC-015 â€” Forward cache and paginated SYNC

## Objective

Implement bounded live caches with explicit storable types, local retention and stable walk cursors.

## Dependencies

`MC-014`, `MC-007` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/core/**`, `tests/simulator/**`, `tests/integration/sync/**`, `tests/vectors/base/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its Â§0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Implement bounded live caches with explicit storable types, local retention and stable walk cursors.
- Implement session admission/continuation, empty completion, cancellation and exact embedded-packet ingress accounting using the MC-007 budgets.
- Preserve stored TTL and message identity; define cache expiry during pagination, fixed-Bloom omissions and byte-bounded recent-context selection.

## Exit criteria

- [x] Late-join tests meet the approved eligible-workload gate at each supported link capacity.
- [x] Empty caches, full-byte-budget pages, concurrent mutation, disconnects and stale cursors terminate predictably.
- [x] SYNC cannot reset TTL, bypass crypto/ingress caps or turn expired cache entries into indefinite replay loops.
- [x] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If the eligible data exceeds budget, return explicit truncation/continuation metadata and deliver bounded recent context.
- If a Bloom positive hides an item, report probabilistic best effort; retries with the unchanged filter must not be claimed to repair it.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Implemented from main `953debe57d3119e88524ef287dab6a664e95d874`; MC-014/007 dependencies are complete. Initial review found an admission API blocker, resolved at `ee7e6b9127d71fcc571483f133e8fbbe54c45191`; Terra follow-up review 5208995661 accepted that revision with no new blockers. Completion becomes effective only on squash merge.

### Implementation and lifecycle

`sync::Cache` admits only stored CHAT with explicit unverified/pending state. Normal/Saver retain at most 500 records and 256 KiB encoded bytes (320 KiB allocation ceiling); Beacon 5000 and 5 MiB (6 MiB ceiling). Exact bytes and stored TTL survive reads. Expiry stays anchored to first arrival. Expired payload is cleared; bounded metadata tombstones prevent tracked expired variants from being revived by replay. Ordinary pressure can evict that finite history; no permanent global replay ledger is claimed. Snapshots retain sequence references only, newest first; eviction/expiry is checked during selection, and later inserts are excluded. Mode changes preserve original arrival and cannot resurrect already expired payload.

`sync::session::Sessions` caps one walk per direction/link, two serving plus two requesting/node, eight links, 40 KiB reference arrays per serving snapshot and total allocated session state below 256 KiB. It tracks all remote-used u16 session IDs per link without assuming ascending IDs. Refused initial IDs are consumed, exact duplicates do not restart a walk, cursors are opaque monotonically allocated tokens bound to active link/session/filter/count/snapshot position and consumed once. Four items/page, eight items/8192 encoded bytes/session, explicit empty/complete/truncated endings, 120-second absolute lifetime, disconnect/cancel/native-failure endings and four bounded gap objects/session with a 30-second absolute unresolved-gap deadline are enforced. Native consumers must call `advance` with monotonic time, report the first request attempt, and feed each Relay terminal result back to `served_complete`; the integration harness exercises that contract.

Deferred ingress issues a non-Clone/non-Copy opaque borrowed `SyncAdmission` only after actual outer frames/bytes/staging/reassembly and applicable logical control admission. Public session dispatch requires consuming that token, bound to immutable output bytes, link, object kind and dispatch time; raw bytes cannot manufacture admission. Embedded admission is crate-private. The production receiver correlates the session before inner admission; only ordered, active-session data invokes normal embedded CHAT admission. Stale/unsolicited/gapped objects cannot alter inner logical state. Exact duplicates are ignored, conflicts abort, markers cannot complete or permit continuation before prior sequence processing. This preserves structural pending state and does not invent cryptographic verification. MC-019/020/021 and MC-022 retain integrated crypto/replay and independent-assessment ownership under the approved base sequencing.

### Measured automated evidence

The production Cache/Sessions/Ingress/Relay/Encoder two-way harness uses a lossless ready Normal link, one frame/second, 20 ms native completion/transit, initially pre-admitted links/full buckets/source cache, signed-structure 421-byte ANNOUNCE at 0/30/60/90 seconds and clear 36-byte REACTION every 10 seconds. Nine setup frames/1227 bytes per direction are explicitly reserved before the initial request; this is not a valid proof or radio test. All eight selected packets plus ordered terminal markers arrive in both directions:

| Encoded CHAT fixture | C=146 | C=182 | C=512 |
|---|---:|---:|---:|
| unsigned 338 bytes | 48.02 s | 43.02 s | 16.02 s |
| organizer structure 556 bytes | 71.02 s | 52.02 s | 25.02 s |
| encrypted structure 387 bytes | 57.02 s | 43.02 s | 16.02 s |

All 18 direction/capacity/fixture cases pass the 120-second selected-set timing gate with zero ingress budget drops, exact packet identity/TTL and complete frame accounting. Unsigned/encrypted fixtures use distinct synthetic sender headers to stay within eligible sender workloads; synthetic signed/encrypted bytes remain Pending and are never claimed cryptographically valid. A separate adversarial case exceeds a single sender's allowance and proves rejection without false completion.

The current codec's largest valid v1 CHAT is 556 bytes, so eight real envelopes cannot fill 8192 bytes. Production budget arithmetic is separately tested at 8192/8193 and overflow boundaries; the unchanged MC-007 1024-byte synthetic sizing worksheet passes at 102.02 seconds. That worksheet is capacity evidence only, not a production decode/crypto result. No malformed 1024-byte packet is labeled valid or used to weaken the codec. The 556/387-byte cases exercise actual maximum signed/final encrypted structures with the real scheduler, while full authenticated acceptance remains with the crypto owners.

The original MC-007 JSON fixtures are consumed directly by a Rust simulator test: mixed-channel selection matches all expected IDs/pages/bytes, including unsubscribed channels, and two walks with the fixed filter retain the specified false-positive omission. This triggers the documented probabilistic-best-effort fallback; unchanged-filter retries do not claim recovery or authenticated display. Cache/session tests additionally exercise empty/expired/mutated snapshots, byte/count limits, mode transitions, disconnect/stale tokens, one-use cursors/filter conflicts, session caps/admission credit, duplicate/conflicting/out-of-order responses, gap overflow and timeout, and native failure.

### Validation and applicability

Windows x86_64, Rust/Cargo 1.85.1, Python 3.14.4, cargo-deny 0.20.2. Logs in ignored `.work/mc015/`. All listed gates passed: 79 debug and 79 release Rust tests, four cache/11 session/one 18-case exchange tests included, one direct fixed-fixture simulator test in debug/release, 15 Python simulator and 12 board tests. The later strengthened byte-pressure cache test was rerun in debug/release. A release artifact collision after switching the standalone simulator feature set was resolved with `cargo clean -p meshchat-core --release`; the clean all-feature release suite/build passed. This was stale local build output, not a source fix. Exact committed source will be recorded in the PR before review. Required commands: `cargo fmt --all -- --check`; workspace all-target/all-feature clippy with `-D warnings`; locked offline debug/release all-feature tests; locked offline release build; cargo-deny all-feature locked advisory/bans/licenses/sources; simulator manifest fmt/clippy/debug/release tests/build; scenario definitions and budget worksheet; 15 existing Python simulator regressions; ticketboard default plus 12 unit tests; `git diff --check`; unchanged root/simulator lockfiles.

Corrected clean source `ee7e6b9127d71fcc571483f133e8fbbe54c45191` reproduced 18 exchange case metrics twice (36 executions, source_dirty=false, exact equality); `.work/mc015/fixed-replay.json`. After the review fix, 82 debug/release tests (including two compile-fail token tests), core clippy/fmt/release build and board checks pass. The real exchange remains 18/18, with the same measured timing. Standalone simulator validation uses `.work/mc015/simulator-target` to keep feature builds isolated.

No UniFFI export, native source, build script, native dependency or wire contract changes. This internal core module/ingress path uses the local host component gate under the validation policy; Terra must confirm omitted Android/iOS job applicability. No hardware, proof, actual crypto, independent-security or full hosted-matrix success is claimed.

## Review and merge

- Branch: `ticket/MC-015-forward-cache-and-paginated-sync`.
- PR: https://github.com/wickesjon/meshChat/pull/19.
- Terra medium review 5208909830 at `5638bee2060cab2b2bf4fcb749c12b1c89476f4c` found one P1: raw session APIs did not enforce prior outer ingress admission. Opaque consuming admission tokens replace that trust boundary; runtime and compile-fail regressions cover binding/forgery/reuse. Follow-up COMMENT review [5208995661](https://github.com/wickesjon/meshChat/pull/19#pullrequestreview-5208995661) accepted `ee7e6b9127d71fcc571483f133e8fbbe54c45191`, confirming the P1 resolved and no new blockers; independently reran 12 session tests, all 18 paced cases and two compile-fail doctests. Both reviews confirmed host-only native applicability. This is separate-agent review evidence, not formal author self-approval or the later independent security assessment.
- Squash commit title: `MC-015: Forward cache and paginated SYNC`.
- Completion becomes effective only when the reviewed squash commit lands on main.
