---
id: "MC-011"
title: "Channels, text and deep-link parsing"
depends_on: ["MC-009"]
kind: "core"
branch: "ticket/MC-011-channels-text-and-deep-link-parsing"
---

# MC-011 — Channels, text and deep-link parsing

## Objective

Implement canonical channel normalization, fixed word lists, public IDs and glyph mappings with committed vectors.

## Dependencies

`MC-009` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/core/**`, `tests/vectors/channels/**`, `tests/integration/links/**`, `tests/fuzz/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Implement canonical channel normalization, fixed word lists, public IDs and glyph mappings with committed vectors.
- Parse channel, friend, event and staff links into inert typed proposals; validate host/path, encoding, sizes and bundle versions without changing trust/subscriptions.
- Separate wire validation from display sanitization; define byte-aware nickname/text handling including emoji and accessibility-relevant Unicode behavior.

## Exit criteria

- [ ] All 8,000 triples have deterministic IDs; collisions are detected and resolved by an explicit decision rather than assumed absent.
- [ ] Malformed URLs/bundles are rejected and every valid link requires a native confirmation before state mutation.
- [ ] Boundary tests cover multi-byte lengths, confusables, bidi controls and normalization without altering signed bytes.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If a newer word or bundle version is unknown, return an update-needed state rather than joining a different channel.
- If sanitization would alter authenticated content, retain authenticated original bytes and render a safe separate representation.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Started from main `bee682ae62b04cc360cf44fbe878fa7404178bf5` after MC-010 squash; MC-009 is complete. Production implementation has not begun. A read-only standard-library enumeration of the design's 20×20×20 fixed triples found 8,000 unique SHA-256 prefix IDs, no private/private collisions and no collisions with the three public IDs. This is preliminary arithmetic evidence, not committed implementation/vector coverage.

### Blocker: root lockfile scope approval

The permitted paths exclude root `Cargo.lock`, but proper SHA-256 channel hashing and the specified NFC/TR39 behavior need direct dependency declarations and a resolved lockfile update. Do not copy a hash/Unicode implementation to evade this boundary or replace full normalization/confusable handling with partial tables. Proposed scope addition: **`Cargo.lock`**, only for MC-011's exact dependency integration. `src/core/Cargo.toml` is already permitted. No dependency/security-policy exception is proposed.

Proposed direct dependencies are `sha2 = "=0.10.9"`, `unicode-normalization = "=0.1.24"`, `unicode-security = "=0.1.2"`, and normal-build availability of the already pinned `zeroize = "=1.8.1"` for private staff-provisioning proposal lifetime. SHA-256 and zeroize already exist in the locked graph. The [normalization crate](https://docs.rs/unicode-normalization/0.1.24/unicode_normalization/) supplies NFC and combining-mark tables; [unicode-security](https://docs.rs/unicode-security/0.1.2/unicode_security/) supplies the TR39 skeleton operation. Both chosen NFC/confusable tables report Unicode 16.0.0. An initial normalization 0.1.25 experiment reports Unicode 17.0.0; 0.1.24 is proposed to align those two tables explicitly, not silently mix data versions. A future table update requires reviewing expected text/security vectors.

An isolated repo-local experiment resolved only these five new registry packages (production manifest/lockfile untouched):

| Package | Version | Crates.io checksum |
|---|---|---|
| unicode-normalization |0.1.24|`5033c97c4262335cded6d6fc3e5c18ab755e1a3dc96376350f3d8e9f009ad956`|
| unicode-security |0.1.2|`2e4ddba1535dd35ed8b61c52166b7155d7f4e4b8847cec6f48e71dc66d8b5e50`|
| unicode-script |0.5.8|`383ad40bb927465ec0ce7720e033cb4ca06912855fc35db31b5755d0de75b1ee`|
| tinyvec |1.13.2|`4cf0ded5c4e56918d8f8a339e1bb67d038d3bc6d144ac407904015ba2e4cde9b`|
| tinyvec_macros |0.1.1|`1f3ccbac311fea05f86f61904b462b55fb3df8837a366dfc601a0161d0532f20`|

The disposable workspace used the current core with security-probe/bindgen features and those exact direct dependencies, seeded from current Cargo.lock. Rust/cargo 1.85.1 Windows MSVC compiled and ran public synthetic checks for composed-accent NFC, Cyrillic/Latin confusable skeleton equality and the literal General channel ID. `cargo-deny 0.20.2 --all-features --locked --config src/core/deny.toml check` passes advisories/bans/licenses/sources with fresh advisory data. No new duplicate-version allowances are needed. The probe is non-publishable like this private workspace; an initial missing-private-manifest setting caused only its own license gate to fail, then was corrected before the passing run. Sources and resolved checksums above are available in `.work/mc011-dependency/`; generated outputs are ignored.

Dependency changes are covered by the native-consumer row of the approved local-validation policy. This scope approval alone would not waive those checks. The current Windows environment has no Mac/Xcode for required iOS validation; hosted Actions provisioning is unavailable under the previously recorded account limitation. That remains a separate merge prerequisite to resolve through available native runners or an explicit validation-policy decision. No host probe is described as native evidence, and no hardware/independent-security gates are waived.

Awaiting the root-lockfile scope decision before production dependency changes. Ticket remains inprogress and incomplete; no PR/review/merge is claimed.

## Review and merge

- Branch: `ticket/MC-011-channels-text-and-deep-link-parsing`.
- Review/PR: pending.
- Squash commit title: `MC-011: Channels, text and deep-link parsing`.
- Completion becomes effective only when the reviewed squash commit lands on main.
