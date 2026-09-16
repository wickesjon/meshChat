---
id: "MC-021"
title: "Organizer trust, credentials and signed updates"
depends_on: ["MC-019","MC-006","MC-016"]
kind: "security"
branch: "ticket/MC-021-organizer-trust-credentials-and-signed-updates"
---

# MC-021 — Organizer trust, credentials and signed updates

## Objective

Implement canonical root/staff bundles, root adoption expiry, credential binding and authenticated pin metadata.

## Dependencies

`MC-019`, `MC-006`, `MC-016` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/core/**`, `tests/vectors/crypto/**`, `tests/integration/organizer/**`, `tests/simulator/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Implement canonical root/staff bundles, root adoption expiry, credential binding and authenticated pin metadata.
- Implement bounded opaque credential caching and CRED_OFFER/REQ recovery across non-adopting relays, with quotas and expiry.
- Separate display trust from structural relay allowance and verify staff/root/time bindings before assigning authority.

## Exit criteria

- [ ] Integrated ingress and real production verification of organizer signatures/credentials pass invalid-first/cryptographically-valid-second same-ID and identical-authenticated-replay cases. Later valid acceptance remains possible once budget is available; replay repeats no effect or trust refresh. Include failure, eviction, concurrent arrivals and a budget-available recovery phase; no test-verifier substitute.
- [ ] Root-to-credential-to-message vectors pass including multiple staff keys, altered pin fields and mismatched root/key IDs.
- [ ] A multi-hop late joiner resolves credentials when its immediate peer initially lacks them; stale or unavailable data stays pending/unverified.
- [ ] Unadopted roots and forged chains never grant badges; credential floods stay within memory and work limits.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If credentials cannot be recovered within bounded retries, keep the update unverified and retry only under the documented policy.
- If offline expiry makes verification uncertain, expose the clock/trust issue; never extend signed validity silently.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

The user-approved 2026-09-15 MC-013 sequencing correction assigns the real ingress/crypto replay gate here. MC-013 covers admission and pending/unverified state tests only; MC-022 requires these integrated results before full-wire freeze. No criterion is marked passed by this scheduling change.

Implemented and tested locally as detailed below; required automated peer review remains pending. No independent security assessment or physical certification is claimed.

## Review and merge

- Branch: `ticket/MC-021-organizer-trust-credentials-and-signed-updates`.
- Review/PR: [PR #25](https://github.com/wickesjon/meshChat/pull/25).
- Squash commit title: `MC-021: Organizer trust, credentials and signed updates`.
- Completion becomes effective only when the reviewed squash commit lands on main.


### Implementation and scope

Started from main `d6816b43e6c771dd32f9d9481bf8fa8116e5b3dd`, after MC-020 PR #24 merged; all hard dependencies are complete. No production dependency or wire-contract change was needed. Root `Cargo.lock` is unchanged. The simulator adds direct **test-only** references to the exact existing ed25519-dalek 2.2.0, sha2 0.10.9 and zeroize 1.8.1 versions; its lockfile only records those direct edges, adding/upgrading no package.

`organizer::Organizer` owns at most 16 active confirmed roots, 128 credential cache entries, and 8/link or 32/node recovery requests. Adoption consumes an explicit event proposal, checks the exact root self-signature and expiry, and commits only after node work reservation. Incoming EVENT_INFO/credentials cannot adopt a root. The same strict Ed25519 helper as Friends rejects weak/noncanonical keys and signatures. Full-root collision buckets use the existing full-key uniqueness resolver; distinct matching roots grant no authority. Credential variants are bounded, retained opaquely without adoption, and become reusable only after root verification. Omitted credentials require one unique matching candidate; included credentials authenticate that packet's explicit choice. Eviction removes unverified entries before verified LRU; expiry is the earliest signed expiry or 15 minutes unused.

Internal event-root records use existing kind 3: full-root key32 and value `01 || revision:u64_be || signed_bundle101 || confirmed_name` (name at most 32 UTF-8 bytes). The internal kind-5 `mc021.root.counter` changes on explicit adoption/removal, preventing held jobs or authority tokens from surviving remove/re-adopt/renewal. Expired anchors confer no authority; subsequent adoption prunes expired persisted anchors before the 16-active-root cap. Public imported staff credentials use existing kind 4 under full-root32/full-staff32. No schema version change, private seed record or automatic identity/staff-key generation is introduced.

Local adoption/import shares the node work bucket and the same two concurrent slots as radio jobs. Ingress generation zero is reserved for these local jobs; admitted transport generations are strictly positive. Adoption costs one verification; staff import costs one public derivation plus one root verification. Incoming uncached organizer CHAT reserves two verifications, identical validated credential reuse reserves one, and CRED_OFFER reserves one root verification. Failures retain the charge. HELLO/directional admission, original pending deadlines, byte-identity rejection and real verifier completion are mandatory; neither a pending record nor structural organizer classification grants authority.

Recovery stays within the existing protocol: direct CRED_REQ, initial attempt plus at most two retries separated by five seconds, one request per link per five seconds, and the original 30-second pending deadline. An immediate peer lacking a credential does not invent a forwarded request protocol; a later flooded CRED_OFFER can supply it and the late joiner. Outbound intents still pass the existing relay/frame/byte queues. `OfferSchedule` covers launch, first post and five-minute active announcements. First signing with a protected staff session requires an included credential; subsequent posts may omit it. Offer/cache/recovery state is ephemeral and cannot create root adoption after restart.

Protected `StaffSession` consumes an explicitly confirmed staff proposal inside the native-provider boundary, matches seed-derived public key and credential, and holds only a zeroizing library key behind a mutex. It has no private export. Native adapters must protect the original import before releasing their QR buffers and invalidate the temporary session on lock/background/forget; application feature wiring remains in its owning native tickets. This core implementation does not claim native staff provisioning or hardware storage has shipped. Public schedule state is separate from temporary private sessions. Signing binds the separate ordinary sender claim, full adopted root/credential choice, exact transcript, cosmetics and pin metadata; staff authority never establishes friend/device identity.

Authenticated incoming and outgoing posts commit persistent subject `03 || full_root32 || full_staff32`, immutable-byte digest, history and replay/conflict tombstones before returning an effect. Provenance retains the full root bundle and credential for historical interpretation. Current display/egress must recheck the opaque `Authority` token against generation, adoption revision and the persistent clock. Root/credential validity is strict; unavailable or rolled-back clocks grant no authority, and a pin cannot exceed credential/root expiry. Expired pins do not suppress otherwise valid historical content. Replay/conflict outcomes do not repeat accepted effects or refresh presence; stored signatures alone are historical evidence.

### Validation evidence

Windows host: Rust/cargo 1.85.1, Python 3.14.4, cargo-deny 0.20.2, Node 24.15.0/OpenSSL 3.5.5. Final commands and results:

- `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`: pass.
- `cargo test --workspace --all-features --locked`, debug and `--release`: 143 tests each, including 18 organizer cases.
- `cargo build --workspace --all-features --locked --release`: pass.
- `python -B tests/integration/storage/run_policy.py`: 11 existing encrypted-store policy/transaction regressions pass.
- `cargo test --manifest-path tests/simulator/Cargo.toml --locked`, debug and `--release`: 35 tests each.
- Refreshed `cargo-deny --all-features --locked --config src/core/deny.toml check`: advisory/bans/license/source gates pass with only existing unmatched-license-allowance warnings. Exact nine-pair dependency guard passes in the normal suite.
- Node reference fixture generation reproduces published RFC8032 TEST 1 before generating the exact MC-008 root, credential and staff-message bytes; the production provider and verifier match those fixtures. See the crypto-vector README for provenance/reproduction.
- Board regeneration/default validation, 12 validator regressions and `git diff --check` pass before publishing.

The organizer cases include all immutable bytes/TTL, multiple staff, wrong domain, forged/unadopted chains, ambiguous credential variants, synthetic root collision buckets, expiry/pin/clock rules, changed adoption during a held job, invalid-first/valid-second, exact replay after database reopen/history deletion, conflict, transaction rollback, two concurrent jobs, expiry/eviction and budget recovery. A 100,000-entry live ledger refuses a new organizer effect. Eight admitted links fill the opaque cache to 128 entries without exceeding its bound; allocated organizer state remains below 64 KiB. The late-joiner scenario traverses real admitted bytes via an initially empty non-adopting relay and a later flooded credential, then runs actual root/staff verification. It is deterministic model evidence, not measured radio availability under attack.

An initial simulator test-wrapper compile exposed missing direct test dependencies; the exact existing versions were added within the permitted simulator paths. Final full simulator checks pass. Local logs are in `.work/mc021/`; the reviewed source hash will be recorded in the PR rather than self-referentially here.

### Native-check applicability

This ticket adds internal shared-core protocol behavior and tests. It changes no UniFFI exports, build script, native adapter, package or production dependency, and it only adds owner-specific storage methods without changing existing native storage operations. Generated all-feature Kotlin and Swift bindings are byte-for-byte identical to the previously reviewed all-feature baseline: SHA-256 Kotlin `08a05881acbc7700ea54ef6aeac237a618f2b5f5aa195ffee1b1b40b01ad7330`, Swift `3d168c928f237815a514a2345caf021c3792e4addc6f58f3e8159c5e9307d19e`. Reproduction: build the host library with `--all-features --locked`, then run `uniffi-bindgen generate --library target/debug/meshchat_core.dll --language kotlin` / `swift` with `--no-format`. Comparing a default-feature build to the all-feature baseline initially showed only the intentionally omitted security-probe functions; using identical feature sets confirms exact parity.

Under the approved local-validation policy, the host protocol/security/storage/simulator checks plus required Terra review can satisfy this internal change's merge gate. The reviewer must confirm this applicability assessment. MC-020's immediately preceding native CI run 35141497235 passed all platform jobs, but is not presented as execution of MC-021. Hosted jobs on this PR are supplemental unless review identifies a native effect. Later native organizer feature integration, physical acceptance and MC-022 independent construction assessment remain mandatory.

### Automated peer review and correction

Terra medium reviewed `46ec19838870cb79b5c81bea8be12c9fcf041c23` in [review 5227968925](https://github.com/wickesjon/meshChat/pull/25#pullrequestreview-5227968925), finding one P1: a root-signed credential could bypass display-character validation through radio ingress. The shared credential-authority check now requires `text::validate` with `CredentialLabel`, covering offers, included/cached credentials, imported sessions and current-authority checks. This enforces the existing text contract without changing signed bytes or the wire format.

The added regression first failed on the reviewed implementation, then passed after the fix. Correctly signed bidi, invisible, newline and NUL labels are rejected through QR parsing, offered/omitted and included message paths; they produce no history effect and do not poison later valid same-ID acceptance. Full core debug/release suites now pass 144 tests each (19 organizer cases); full simulator debug/release suites pass 36 each. Release build, clippy, formatting, ticketboard validation and its 12 regressions, and diff checks pass after the fix. Logs: `.work/mc021/review-fix-*.log`. Dependency graph, native exports and build behavior remain unchanged.

The reviewer explicitly confirmed that the internal-Rust native-check exception applies. Follow-up review of the correction is pending; no independent construction assessment is claimed.
