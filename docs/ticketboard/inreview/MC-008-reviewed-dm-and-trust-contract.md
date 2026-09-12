---
id: "MC-008"
title: "Reviewed DM and trust contract"
depends_on: ["MC-005"]
kind: "decision"
branch: "ticket/MC-008-reviewed-dm-and-trust-contract"
---

# MC-008 — Reviewed DM and trust contract

## Objective

Evaluate a vetted RFC 9180 authenticated-mode implementation against the current custom analogue; record suite, dependency review and interoperability implications.

## Dependencies

`MC-005` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `docs/mesh-chat-design.md`, `docs/decisions/**`, `tests/vectors/crypto/README.md`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Evaluate a vetted RFC 9180 authenticated-mode implementation against the current custom analogue; record suite, dependency review and interoperability implications.
- Define authentication of immutable header fields, mutable TTL exclusion, replay policy, sender/pin binding, key-compromise limitations and domain-separated encodings.
- Define signed-presence freshness and key-change behavior; distinguish a verified historical signature from evidence of a live direct peer.

## Exit criteria

- [x] An explicit crypto decision selects the construction and specifies exact inputs, encodings, rejection rules and evidence required for MC-022.
- [x] Friend identity, X25519 binding, stale presence, unknown keys and key reset have unambiguous state transitions.
- [x] The decision includes independent review requirements and reference-vector sources.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If no suitable vetted HPKE dependency meets platform constraints, keep DMs blocked pending review of the alternative construction.
- If live presence authentication cannot be implemented within scope, display authenticated last-seen evidence rather than asserting current proximity.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Started from main `edb3201b6f4b498ca521bcbe02b7406f875aa228`, with MC-005 complete. The [dependency compatibility proposal](../../decisions/MC-008-dependency-scope-proposal.md) compares HPKE 0.12.0, 0.13.0 and 0.14.1 against the existing pinned core graph in disposable repo-local experiments. All pass advisory/license/source checks but fail the repository's duplicate-version ban. The recommended 0.14.1 graph also passes host `cargo check` with pinned Rust 1.85.1; no native, crypto-vector or independent-security result is claimed.

The user approved the nine exact duplicate-version pairs for later integration, retaining other security gates. An isolated all-target/all-feature active graph check confirms exactly those pairs; a copied deny configuration with the approved exact skips passes advisories/bans/licenses/sources. Production manifests/lockfile/security configuration remain unchanged. This decision records selection and future integration requirements; it does not claim application crypto or platform builds.

The [DM and trust contract](../../decisions/MC-008-crypto-contract.md) selects RFC 9180 Auth/X25519/HKDF-SHA256/ChaCha20Poly1305 via HPKE 0.14.1. It defines full tuple/pin binding, exact 41-byte envelope prefix, 64/144/304-byte minimal plaintext padding (147/227/387 logical bytes), all immutable-header AAD, signature domains, persistent replay and clock/overflow rules, provider-owned HPKE with entropy failure handling, one-use link proof, and full friend lifecycle transitions. A 71-byte proof and cold receive work of eight units fit MC-007's conservative reservations; no periodic renewal traffic is added. The 304-byte bucket resolves the old incomplete 280-byte bucket without reducing the 280-byte text allowance. Long-lived links use the ticket's conservative last-authenticated-observation fallback rather than asserting unproved current proximity; this does not weaken message authentication or change independent/physical gates.

The [crypto vector plan](../../../tests/vectors/crypto/README.md) includes fixed public transcript hashes, domain bytes, every immutable-header-byte mutation and size boundaries, plus reference sources and required positive/negative implementation vectors. The embedded Python check passes on Python 3.14.4. Ticketboard write/default validation, 12 ticketboard tests, both MC-007 scenario/worksheet checks and `git diff --check` pass locally. These are documentation/definition checks plus the separately recorded dependency experiment; application/native code and dependencies are unchanged, so native CI jobs are not applicable under the approved local policy. Terra review and squash merge remain required; MC-022/037 independent assessments and MC-043/044 physical verification remain separate gates.

## Review and merge

- Branch: `ticket/MC-008-reviewed-dm-and-trust-contract`.
- Review/PR: pending.
- Squash commit title: `MC-008: Reviewed DM and trust contract`.
- Completion becomes effective only when the reviewed squash commit lands on main.
