---
id: "MC-019"
title: "Verified friends and fresh presence"
depends_on: ["MC-017","MC-018","MC-011","MC-008"]
kind: "security"
branch: "ticket/MC-019-verified-friends-and-fresh-presence"
---

# MC-019 — Verified friends and fresh presence

## Objective

Implement two-key QR pinning, petnames, signed CHAT/ANNOUNCE verification and bounded public-key distribution/cache behavior.

## Dependencies

`MC-017`, `MC-018`, `MC-011`, `MC-008` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/core/**`, `tests/integration/friends/**`, `tests/vectors/crypto/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Implement two-key QR pinning, petnames, signed CHAT/ANNOUNCE verification and bounded public-key distribution/cache behavior.
- Implement the approved freshness mechanism or last-seen semantics; bind presence to the actual authenticated link/session as specified.
- Handle unsigned impersonation, unknown signatures, changed keys, removal and explicit re-pairing without nickname-based auto-trust.

## Exit criteria

- [x] Integrated ingress and real production verification of friend signatures pass invalid-first/cryptographically-valid-second same-ID and identical-authenticated-replay cases. Later valid acceptance remains possible once budget is available; replay repeats no effect or trust refresh. Include failure, eviction, concurrent arrivals and a budget-available recovery phase; no test-verifier substitute.
- [x] Pinned valid signatures show verified identity; copied IDs/nicknames without valid proof never do.
- [x] Replayed ANNOUNCE cannot create unsupported current-nearby claims.
- [x] Key-change and removal tests block stale-key sends and require explicit pin replacement.
- [x] Relevant local checks pass and source review is recorded; completion is staged for the reviewed squash commit and becomes effective only when it lands on main.

## Potential fallbacks

- If the signer key is unavailable, mark the message unverified/pending and use bounded recovery.
- If identity continuity is uncertain, preserve the old pin and ask for an explicit re-scan rather than matching by nickname.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

The user-approved 2026-09-15 MC-013 sequencing correction assigns the real ingress/crypto replay gate here. MC-013 covers admission and pending/unverified state tests only; MC-022 requires these integrated results before full-wire freeze. No criterion is marked passed by this scheduling change.

Started 2026-09-15 from main `a4cd519d86110fe3b2a331fc992a3b68d867e379` after MC-018 PR #22 merged. MC-017/018/011/008 are complete on main. Read the normative design, MC-006/007/008 contracts and active plan before implementation. Implementation and validation are in progress; no new test/review result is claimed.

The production friend protocol will remain a shared Rust module, following the existing ingress/SYNC modules. Native feature wiring belongs to later integration tickets. Tests live under `tests/integration/friends/`; actual pinned dalek verification and the MC-018 SQLite policy are exercised with synthetic inputs. No new dependency or private-key export is planned.

## Review and merge

- Branch: `ticket/MC-019-verified-friends-and-fresh-presence`.
- Review/PR: [PR #23](https://github.com/wickesjon/meshChat/pull/23).
- Squash commit title: `MC-019: Verified friends and fresh presence`.
- Completion becomes effective only when the reviewed squash commit lands on main.

### Implemented architecture and bounds (2026-09-16)

`friends::Friends` owns explicit two-key confirmation, durable pin revision/send tokens, conservative replacement/removal, strict production Ed25519 verification, bounded public-key recovery, HELLO/LINK_PROOF and signed content creation. User confirmation validates canonical strong Ed, canonical X and protected-provider nonzero agreement before an atomic mutation. Network inputs never create/replace a pin. Same Ed with multiple distinct pinned X tuples is ambiguous for friend attribution. Pin changes invalidate live proof/cache state; old send tokens cannot revive after remove/re-add/restart.

MC-018 records kind 2 now use `version=1 || revision:u64_be || replacing:u8 || petname:UTF8` (11–30 bytes), keyed by the full 64-byte tuple. Internal records kind 5 holds the monotonically increasing revision counter, outside the generic public Setting API. No schema migration or exported storage API change is needed. Store transactions enforce 128 pins, atomic replacement and rollback. Existing history stays attributed to its original full signing identity.

Friend intake charges existing frame/byte/logical limits, gates traffic on both HELLOs, enforces negotiated directional limits, and reserves one work unit before each actual signature operation. At most two opaque verification jobs are admitted concurrently; original pending lifetime bounds the work reservation. Invalid signatures enter only the short rejected-byte cache. Valid CHAT commits the existing full-identity ledger/history transaction before returning an effect. Invalid-first/valid-second same ID, exact replay, conflicts, store failure, absent/backward clock and full ledger cannot create duplicate verified effects. TTL alone is excluded from the immutable transcript/hash. ANNOUNCE authenticates content, never presence. SYNC remains deferred until its MC-015 owner validates session/order and admits the inner record; only then may `retry_pending` verify it.

The public-key cache has 256 LRU entries with 15-minute unused expiry, existing bounded pending recovery and no pin authority. Omitted-key ambiguity stays pending; included keys must match both hints. First actual signed send per link and every signed ANNOUNCE include the full key. Stable signing refuses Confessions. The existing strict dalek verifier plus canonical coordinate checks reject weak keys, noncanonical R/S and wrong hint bindings.

Up to eight admitted links use exact 54-byte HELLOs and role-bound 66-byte LINK_PROOFs. Local nonces must come from the supplied fallible OS CSPRNG, be nonzero and differ from live local nonces. Both HELLOs precede ordinary traffic; changed HELLO disconnects admission. One distinct remote candidate, one local proof with at most one identical retry, ten-second deadlines and both-proof duplicate arbitration follow MC-008. Freshness expires at local nonce age 60 seconds; duplicates/ANNOUNCE do not renew it. Disconnect, discontinuity, pin changes and observed provider invalidation clear live evidence. Last-response age remains a historical observation only.

Native feature wiring remains with its existing integration tickets. Callers must use actual admitted handles/physical role and native limits, preserve node/address connection gates, supply monotonic time including suspend (clear session trust on uncertainty), invalidate session trust on provider lifecycle events, and acquire current protected store/key sessions per operation. These Rust components retain no private-key material and expose no new UniFFI surface. Missing-key recovery is the specified bounded pending fallback; no security acceptance requirement was weakened.

### Automated evidence

Windows host, Rust/cargo 1.85.1, Python 3.14.4, cargo-deny 0.20.2. Exact reviewed source revision will be recorded in the PR (the first implementation commit contains this evidence).

- `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`: pass.
- `cargo test --workspace --all-features --locked` and the same command with `--release`: all 105 tests/doctests pass, including the first 21 friend integration cases. After the provider-invalidation tightening, the affected suite was rerun in debug/release: all 22 friend cases pass; clippy and release build pass again. Logs: `.work/mc019/debug.log`, `release.log`, `final-friends-debug.log`, `final-friends-release.log`.
- `cargo build --workspace --all-features --locked --release`: pass. `git diff --exit-code -- Cargo.lock`: unchanged.
- `python -B tests/integration/storage/run_policy.py`: all 11 existing SQL policy regressions pass. The new friend harness uses a real SQLite transaction callback double with fault injection; synthetic plaintext fixtures do not claim encryption/device evidence.
- `cargo-deny --all-features --locked --config src/core/deny.toml check`: advisories/bans/licenses/sources pass with refreshed advisory database; only existing unmatched license-allowance warnings. The first sandbox network attempt was unavailable, not a passing result.
- `python -B src/core/build_bindings.py host --security-probe`: pass. Generated Swift and Kotlin are byte-for-byte identical to the retained MC-018 bindings: SHA-256 Swift `3d168c928f237815a514a2345caf021c3792e4addc6f58f3e8159c5e9307d19e`, Kotlin `08a05881acbc7700ea54ef6aeac237a618f2b5f5aa195ffee1b1b40b01ad7330`.
- `node tests/vectors/crypto/friend_vectors.cjs`: Node 24.15.0/OpenSSL 3.5.5 independently generates/verifies committed public CHAT/proof fixtures; the Rust integration test verifies them through production intake and matches production signer bytes exactly. This is interoperability evidence, not independent security assessment.
- Ticketboard default validation and all 12 unit tests pass; board regeneration and diff checks run again for the review move.

Local-policy applicability: internal Rust modules and non-exported storage/identity helpers, no native sources/build scripts/dependencies or wire-contract change; exact generated API parity supports host component checks. Terra must confirm this rationale. Hosted jobs may supply additional native regression evidence. MC-022 independent security assessment and scheduled MC-025/027/043/044 device certification remain outstanding separate gates; no physical result is claimed.

### Review disposition and merge staging

Separate automated `gpt-5.6-terra` medium worker `/root/review_mc019` completed read-only review at `bbb9ef16ffd84b85a85a1ee1fb650e9584d62b8b`: no blocking findings. It checked the ticket, MC-006/007/008 contracts, diff, final friend test logs, vectors and the local-policy host-only rationale. It could not rerun Cargo in its own shell because Cargo was absent from that shell's PATH; the implementation agent's recorded checks remain the execution evidence. Its actual result is transparently relayed in PR COMMENT review `5220125339`, not a formal self-approval or external security assessment. The initially attempted label “Independent Terra review” was rejected by automatic approval review and withdrawn; the successful record expressly describes a separate automated worker and retains the MC-022 gate.

CI run `35071716315` supplied an additional passing ticketboard result and native core/binding builds; the remaining hosted steps were still running when completion metadata was prepared. Hosted completion is not claimed. Required merge evidence is the applicable successful local checks and separate Terra review under the approved policy. The final metadata revision must receive follow-up review before squash merge.