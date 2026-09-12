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

- [ ] An explicit crypto decision selects the construction and specifies exact inputs, encodings, rejection rules and evidence required for MC-022.
- [ ] Friend identity, X25519 binding, stale presence, unknown keys and key reset have unambiguous state transitions.
- [ ] The decision includes independent review requirements and reference-vector sources.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If no suitable vetted HPKE dependency meets platform constraints, keep DMs blocked pending review of the alternative construction.
- If live presence authentication cannot be implemented within scope, display authenticated last-seen evidence rather than asserting current proximity.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Started from main `edb3201b6f4b498ca521bcbe02b7406f875aa228`, with MC-005 complete. The [dependency compatibility proposal](../../decisions/MC-008-dependency-scope-proposal.md) compares HPKE 0.12.0, 0.13.0 and 0.14.1 against the existing pinned core graph in disposable repo-local experiments. All pass advisory/license/source checks but fail the repository's duplicate-version ban. The recommended 0.14.1 graph also passes host `cargo check` with pinned Rust 1.85.1; no native, crypto-vector or independent-security result is claimed.

Blocked on an explicit scope decision for the proposal's nine exact duplicate-version pairs, to be applied only when integrating the selected dependency. Production manifests/lockfile/security configuration are unchanged. No construction selection is frozen yet, no acceptance checkbox is complete, and this ticket must not merge or unblock dependent implementation. Older versions also require exceptions; a coordinated existing-dependency upgrade is a separate alternative scope. The proposal records reproducible manifests, commands, retained logs, graph hash and tradeoffs. Ticketboard regeneration/default validation and documentation checks must pass before publishing the eventual ready PR; no PR or Terra review is claimed at this stage.

## Review and merge

- Branch: `ticket/MC-008-reviewed-dm-and-trust-contract`.
- Review/PR: pending.
- Squash commit title: `MC-008: Reviewed DM and trust contract`.
- Completion becomes effective only when the reviewed squash commit lands on main.
