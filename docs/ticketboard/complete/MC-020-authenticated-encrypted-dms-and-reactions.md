---
id: "MC-020"
title: "Authenticated encrypted DMs and reactions"
depends_on: ["MC-019","MC-008","MC-006","MC-016"]
kind: "security"
branch: "ticket/MC-020-authenticated-encrypted-dms-and-reactions"
---

# MC-020 — Authenticated encrypted DMs and reactions

## Objective

Implement the selected authenticated encryption construction with exact plaintext/padding, suite parameters and immutable-header bindings.

## Dependencies

`MC-019`, `MC-008`, `MC-006`, `MC-016` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/core/**`, `tests/vectors/crypto/**`, `tests/integration/dm/**`, `tests/simulator/**`, `Cargo.lock` (user-approved selected dependency integration only).

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Implement the selected authenticated encryption construction with exact plaintext/padding, suite parameters and immutable-header bindings.
- Implement rotating tag lookup with collision handling, friend-key validation, clock-boundary behavior and encrypted reactions.
- Integrate blind relay/SYNC and encrypted local persistence; authenticate UI identity from the pin/key proof.

## Exit criteria

- [x] Integrated ingress and real production verification of encrypted CHAT/REACTION pass invalid-first/cryptographically-valid-second same-ID and identical-authenticated-replay cases. Later valid acceptance remains possible once budget is available; replay repeats no effect or trust refresh. Include failure, eviction, concurrent arrivals and a budget-available recovery phase; no test-verifier substitute.
- [x] Reference/KAT, sender-forgery, all-zero X25519, malformed-key and header/ciphertext tamper vectors pass.
- [x] Relays cannot decrypt; valid DMs and reactions round-trip across the simulator and restart safely in encrypted history.
- [x] Epoch skew/collision, replay and changed-key tests fail closed without plaintext downgrade.
- [x] Relevant source checks and automated peer review pass. Final completion metadata is pending review and squash merge; completion takes effect only on main.

## Potential fallbacks

- If the crypto review rejects the construction, block the feature and revise MC-008 before proceeding.
- If recipient key/tag resolution fails, keep bounded opaque relay behavior and do not display a guessed sender or plaintext.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

The user-approved 2026-09-15 MC-013 sequencing correction assigns the real ingress/crypto replay gate here. MC-013 covers admission and pending/unverified state tests only; MC-022 requires these integrated results before full-wire freeze. No criterion is marked passed by this scheduling change.

Implemented and tested locally as detailed below. Automated peer review is recorded below; native platform checks passed; physical certification and independent construction review are not claimed.

## Review and merge

- Branch: `ticket/MC-020-authenticated-encrypted-dms-and-reactions`.
- Review/PR: [PR #24](https://github.com/wickesjon/meshChat/pull/24).
- Squash commit title: `MC-020: Authenticated encrypted DMs and reactions`.
- Completion becomes effective only when the reviewed squash commit lands on main.

### Dependency scope approval (2026-09-16)

Started on the dedicated branch from main `0b99ad9bb5ab70b35bcaa510af274bb06646c637`, after MC-019 PR #23 squash-merged; all hard dependencies are complete. The user explicitly approved adding root `Cargo.lock` to this ticket's permitted paths solely for the selected MC-020 dependency integration. The [MC-008 decision](../../decisions/MC-008-dependency-scope-proposal.md) already selects HPKE 0.14.1 and exactly nine duplicate-version pairs. Existing pins and those exact exceptions are preserved; no broader dependency refresh or security-policy change was approved.

### Implemented MC-020 integration

`dm::Dms` creates and accepts only profile-01 Auth CHAT/REACTION. It resolves a unique confirmed full tuple using both public hints, revalidates pin generations and provider availability, reserves seven send/eight receive work units, and never tries every friend's tags. Missing/ambiguous pins remain opaque pending; tag misses remain bounded pending for possible local-clock recovery. Malformed/authentication failures enter the rejected-byte cache without poisoning another variant sharing the ID. Work remains limited to two jobs and the original pending deadline.

Protected operations live under `identity/dm.rs`, hold the provider material lock, and instantiate the pinned HPKE key types internally. Production reads exactly 32 bytes from the fallible OS source and uses the inspected one-use adapter; unexpected consumption discards output. No private-export method or new UniFFI surface is added. HMAC and constant-time comparisons use the selected library family. The existing symmetric dependencies' supported zeroization features are enabled with exact direct pins (ChaCha20Poly1305 0.11.0, Poly1305 0.9.1, HMAC 0.13.0, SHA2 0.11.0). Seed, failed-open plaintext, caller-owned content and accepted-history scratch have explicit clearing; HPKE/library key/context clearing is used where supported. Native/FFI/SQLite transient copies remain subject to the existing MC-017 memory-erasure limitation; complete runtime memory erasure is not claimed.

Wire encoding retains the exact MC-008 info/AAD domains, tuple roles, final lengths, TTL exclusion, epoch tags and minimal padding. Each send uses a fresh context exactly once; retries must retain original ciphertext. The returned send token remains subject to current-pin validation at native egress. Native feature wiring follows in its existing owning tickets; loaded public metadata, protected operation lifetime, actual HELLO handles/capacities and clock inputs remain caller contracts.

Incoming/outgoing direct history uses the full peer tuple, both direction/type keys, the persistent ledger and current clock policy. No plaintext fallback exists. A new reaction and its current target-state mutation commit in the same SQL transaction as its ledger/history record; transaction failure leaves it retryable. Target IDs with multiple authenticated meanings or conflicts give no reaction effect. One current reaction per actor is stored; add replaces and remove clears. Internal generic record kind 5 holds a durable ordering counter; kind 6 keys are peer64/target8/direction1 and values sequence8/remove1/code1. DM history provenance is subject65/sequence8. These are owner-defined records in the existing extensible schema, not a wire or native schema migration. State is pruned with target history; sequence comparison prevents an old deferred reaction overwriting a newer one after recovery.

Orphans retain only opaque pin/record handles: 32 per link, 256 node, 120 seconds from first acceptance, oldest-first eviction. They are not revived on restart or authenticated replay. Reaction queries may recover still-live orphans once a unique authenticated target exists; persisted applied state survives reopening. Pin/provider changes and clock discontinuity refuse stale effects. The deterministic simulator uses actual HPKE bytes through admitted fragments, a blind third node's opaque cache, ordered SYNC and endpoint decryption; this is model evidence, not radio or hardware certification.

`src/core/check_dependency_pairs.py`, invoked by the normal DM integration suite, checks the all-target/all-feature graph against precisely the nine MC-008 approved duplicate pairs. Existing root and simulator lockfile package/version pins are preserved; no version was removed or upgraded. The simulator's own lockfile update is already within `tests/simulator/**` scope. The private-provider test module is included from `tests/integration/dm/provider.rs` solely to reach non-exported internals; this documents the framework-required source-adjacent registration while keeping test source under `tests/`.

### Validation status

Windows host: Rust/cargo 1.85.1, Python 3.14.4, cargo-deny 0.20.2, Node 24.15.0/OpenSSL 3.5.5. The reference generator first reproduces the published RFC Auth fixture before generating exact meshChat bytes and malformed/forgery cases; production protected encryption output matches the independent ciphertext. It demonstrates recipient-fabrication/KCI as an expected limitation. No independent security assessment is claimed.

Initial review revision `b732eefea17412f8d719a4dcbb161339a46ee62e` passed the following local checks after the symmetric zeroization features were enabled:

- `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`.
- `cargo test --workspace --all-features --locked`, both debug and `--release`: 123 tests each, including 14 real DM integration cases and three protected-provider/KAT tests.
- `cargo build --workspace --all-features --locked --release`.
- `python -B tests/integration/storage/run_policy.py`: 11 policy/transaction regressions.
- `cargo test --manifest-path tests/simulator/Cargo.toml --locked`, both debug and `--release`: 15 tests each, including the production encrypted exchange harness.
- `cargo-deny 0.20.2 --all-features --locked --config src/core/deny.toml check`: refreshed advisories, bans, licenses and sources pass; only pre-existing unmatched-license-allowance warnings. The all-target/all-feature exact duplicate-pair guard passes within the normal test suite.
- Node 24.15.0/OpenSSL 3.5.5 reference fixture regeneration verifies the RFC Auth KAT before producing the meshChat fixture; see the vector README for reproduction/provenance.
- `git diff --check`, ticketboard regeneration/default validation, and its 12 validator regressions.

An initial simulator wrapper path error in the graph guard was corrected; the shared test now resolves the core manifest correctly from either harness. Final debug/release simulator checks pass. Logs remain under `.work/mc020/`. Source revision is recorded in the PR review/check record to avoid a self-referential hash.

Native platform checks are required because the shared dependency graph changed. Hosted Android/iOS build/FFI/storage regressions completed on the PR source; no physical phone is needed for those compilation checks. MC-022 independent construction review and all scheduled real-device certification gates remain outstanding separately.

### First automated peer review and fixes

Separate `gpt-5.6-terra` medium worker reviewed `b732eefea17412f8d719a4dcbb161339a46ee62e` and posted [review 5227409792](https://github.com/wickesjon/meshChat/pull/24#pullrequestreview-5227409792). It found two blocking reaction-recovery issues: deferred effects could commit before wall-clock validation, and a post-acceptance database error could discard the sole outgoing ciphertext (or incoming orphan registration). This is automated peer review, not MC-022's independent security assessment.

Recovery now validates the persistent wall-clock policy and prunes expired records before applying an orphan. Initial acceptance returns both the replay outcome and target-effect status from its existing transaction; subsequent volatile orphan registration cannot perform a fallible database operation. Regression cases exercise missing/rolled-back clocks with direct committed-state inspection and recovery, and a callback that makes the database unavailable immediately after the real ledger/history COMMIT. Both incoming and outgoing reactions, with and without a target, preserve their acceptance result, outgoing bytes and orphan handle. No wire, dependency or FFI contract changes.

Post-fix validation passes: formatting/clippy, 125 workspace tests each in debug/release, 17 simulator tests each in debug/release, 11 storage-policy regressions, ticketboard validation and diff check. The exact dependency graph is unchanged; the refreshed dependency gate above remains applicable. The release build also passes. Native checks on the fixed revision passed as recorded below.


Separate Terra medium follow-up [review 5227463926](https://github.com/wickesjon/meshChat/pull/24#pullrequestreview-5227463926) reviewed `1a67cfca35c9774b3f898746a0345ebba5a34ce7`, confirmed both blockers fixed, and found no new blocker. Its own targeted `cargo test --test dm --locked` passed all 16 tests. The source implementation and full local results above apply to that revision.


### Native checks and final merge record

[CI run 35141497235](https://github.com/wickesjon/meshChat/actions/runs/35141497235) for source `1a67cfca35c9774b3f898746a0345ebba5a34ce7` completed successfully in all four jobs: ticketboard, Rust, Android and iOS. Android passed the shared native library/Kotlin build, app build/lint/tests, packaged-library/alignment checks, BLE and security builds, emulator key lifecycle and native SQLCipher storage regression. iOS passed device/simulator library builds, Swift FFI/CryptoKit parity, unsigned app/BLE/security builds and Swift SQLCipher create/reopen/checks/key-loss/reset. The workflow pins Rust 1.85.1, Android NDK 27.3.13750724 and Xcode 16.4. These are compile, emulator and host-integration results; physical protection/radio certification and independent construction assessment remain separate.

The initial CI run 35140817424 was superseded by the fixes and is not counted as a successful full run. The passing fixed-source run above is the native gate evidence. Final changes after that source revision are ticket status/evidence and generated index metadata only; source, dependency locks, native code and CI configuration are unchanged. The recorded fixed-source tests remain applicable under the local-validation policy. Final metadata receives ticketboard/default validator checks, all 12 validator regressions, diff check and a separate Terra follow-up recorded on PR #24 at the final head. No external security approval is claimed. Pending the final recorded review, squash title is `MC-020: Authenticated encrypted DMs and reactions`.
