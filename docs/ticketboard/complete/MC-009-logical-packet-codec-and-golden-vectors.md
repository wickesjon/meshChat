---
id: "MC-009"
title: "Logical packet codec and golden vectors"
depends_on: ["MC-003","MC-006"]
kind: "core"
branch: "ticket/MC-009-logical-packet-codec-and-golden-vectors"
---

# MC-009 — Logical packet codec and golden vectors

## Objective

Implement bounded header/type parsing and serialization for CHAT, ANNOUNCE, SYNC_REQ, REACTION, EVENT_INFO, CRED_REQ/OFFER and opaque unknown types.

## Dependencies

`MC-003`, `MC-006` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/core/**`, `tests/vectors/base/**`, `tests/integration/codec/**`, `tests/fuzz/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Implement bounded header/type parsing and serialization for CHAT, ANNOUNCE, SYNC_REQ, REACTION, EVENT_INFO, CRED_REQ/OFFER and opaque unknown types.
- Enforce the resolved length, UTF-8, flag and control-packet rules using checked arithmetic; preserve original wire bytes for authentication.
- Add committed byte-exact vectors and parser fuzz targets from the canonical contract.

## Exit criteria

- [x] All supported types round-trip and match golden vectors; malformed lengths and flag combinations return errors.
- [x] Unknown-type forwarding preserves opaque bytes within conservative budgets; unknown flags never bypass known-type validation.
- [x] Fuzz smoke runs complete without panic or unbounded allocation and persist useful regression inputs.
- [x] Relevant checks pass and review is recorded. Final completion is staged for PR #11 and becomes effective only after the final review and squash land on main.

## Potential fallbacks

- If a vector disagrees with the design, stop and resolve the contract; do not make the parser accept both ambiguous layouts.
- Keep unimplemented future types opaque and undisplayed within the defined compatibility policy.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Started from main `f600933bcdcad9fe4c2235694a56718b76ae10f8` after MC-008 squash; hard dependencies MC-003 and MC-006 are complete on main. MC-008's resolved profile is also available. No fallback or scope expansion was required.

Implemented `src/core/src/codec.rs` as a borrowed, allocation-free structural parser with caller-buffer serialization and TTL-only forwarding. It covers all known logical types, friend/organizer signature metadata and credentials, canonical profile01 encrypted envelopes and opaque future types. It checks the 26–1,024-byte envelope, exact conditional lengths, semantic flags despite unknown high bits, raw UTF-8 byte bounds, direct-control and Event channel restrictions, stored-CHAT context and cheap identity/root equality. It preserves original header/payload/text bytes for authentication. No authentication, key hashing, trust, display sanitization, rate-limit admission or history side effect is implied. Those remain their owning later tickets; this separation does not relax their requirements. Origin ID/TTL/reserved-bit/signature policy is distinct from wire serialization.

The [base vectors](../../../tests/vectors/base/README.md) persist 170 positive/negative cases with a separate Python grammar assembler, literal MC-006 reaction comparison, all layouts/maxima, every known-type disallowed flag pair, every truncation, strict UTF-8 and field boundaries, every unknown type/flag pair and byte-preserving forward checks. The [fuzz target](../../../tests/fuzz/README.md) exercises both parse contexts and standalone credentials with seed truncations/inversions, 100,000 reproducible mutations and random sizes 0–1,100. This is mutation fuzzing, not a coverage-guided campaign. Parser code has no allocation sites; harness buffers and serialization outputs are explicitly bounded. Persisted malformed cases serve as regression seeds.

Local Windows x86_64 MSVC checks use Rust/cargo 1.85.1, Python 3.14.4 and cargo-deny 0.20.2. Required commands: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`; debug and release `cargo test --workspace --all-features --locked`; `cargo build --workspace --all-features --locked --release`; `git diff --exit-code -- Cargo.lock`; `cargo-deny --all-features --locked --config src/core/deny.toml check`; `python -B tests/vectors/base/generate.py`; ticketboard write/default and 12 unit tests; `git diff --check`. All commands pass: 14 Rust tests each in debug/release, 170 vectors, 12 board tests and all four deny gates. The final reviewed revision is recorded in the PR.

Only an internal Rust module and source-external Cargo test targets were added. No dependency, lockfile, build script, UniFFI export or native interface changed. The module is not connected to native ingress yet. Under the approved local policy, host Rust/vector/fuzz/security checks apply; Android/iOS jobs have no affected interface or packaging behavior and are omitted subject to required Terra confirmation. No native, physical, independent security or authenticated-message result is claimed.

## Review and merge

- Branch: `ticket/MC-009-logical-packet-codec-and-golden-vectors`.
- Review/PR: [PR #11](https://github.com/wickesjon/meshChat/pull/11). Separate Terra medium [COMMENT review 5185002804](https://github.com/wickesjon/meshChat/pull/11#pullrequestreview-5185002804) on `7b24a81797cbe8711d03ffa98e3ee94e5056f58a` found no actionable issues and reproduced all listed checks, using cached offline advisory data after a restricted-network refresh failed. Root refreshed the advisory data successfully for its passing run. Reviewer confirmed omitted native jobs are inapplicable; no independent security/native attestation is claimed. Final metadata-only revision and follow-up review are recorded in the PR before squash.
- Squash commit title: `MC-009: Logical packet codec and golden vectors`.
- Completion becomes effective only when the reviewed squash commit lands on main.
