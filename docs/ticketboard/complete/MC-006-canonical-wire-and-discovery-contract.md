---
id: "MC-006"
title: "Canonical wire and discovery contract"
depends_on: ["MC-004"]
kind: "decision"
branch: "ticket/MC-006-canonical-wire-and-discovery-contract"
---

# MC-006 — Canonical wire and discovery contract

## Objective

Freeze one outer frame per GATT value with runtime capacity per direction; define every frame/type/flag combination, exact lengths and reserved-field handling.

## Dependencies

`MC-004` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `docs/mesh-chat-design.md`, `docs/decisions/**`, `tests/vectors/README.md`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Freeze one outer frame per GATT value with runtime capacity per direction; define every frame/type/flag combination, exact lengths and reserved-field handling.
- Specify logical and transport-object reassembly, low-capacity refusal, control-packet hop scope, duplicate-link resolution after identity exchange and concrete UUIDs.
- Resolve event QR framing, organizer pin fields/transcripts, cosmetic fields, friend key distribution, DM plaintext/padding framing in coordination with MC-008, and canonical vector conventions.

## Exit criteria

- [x] Independent readers can determine exact bytes and parser behavior for every v1 packet, QR bundle and flag combination.
- [x] Every supported packet fits the selected capacity/fragment limits or has a documented explicit refusal case.
- [x] No pin/cosmetic field is described without a byte position and authentication rule; unresolved crypto choices remain blocked on MC-008 before MC-020.
- [x] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main (completion staged for the reviewed squash; effective only after final CI and merge).

## Potential fallbacks

- If optional remote cosmetic hints need a new incompatible layout, keep them local for v1 and record the approved product consequence.
- Do not guess a wire layout in implementation; reopen this decision for a versioned correction before freeze.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Started from main `05596059e779fe5db6f0c88b165f1c882627de9e`, with MC-004 complete and MC-005 squash-merged as PR #7. The [normative contract](../../decisions/MC-006-wire-contract.md), matching design updates and [vector conventions](../../../tests/vectors/README.md) specify the base formats and explicit MC-008 extension boundaries. The first criterion covers the base contract together with those declared crypto blockers; no encrypted-envelope, exact crypto-domain or fresh-proof decision is claimed complete here.

Decisions: one frame per value, 146–512-byte directional admission, explicit type/flag and reserved-field matrix, bounded conflict-aborting reassembly, four production UUIDs and a 54-byte whole-only HELLO, proof-gated duplicate consolidation preserving asymmetric links, exact QR bundles/URI rules, four cosmetic bytes and five organizer pin bytes with authentication coverage. SYNC_REQ now carries a requester-owned session ID (520-byte payload, 546 logical) so delayed responses cannot bind to a later request; explicit zero-blob page/session markers resolve empty and budget-ended responses. The decision documents pre-freeze compatibility consequences. No product fallback, physical-evidence waiver or scope expansion was used.

Evidence state is **specified**. Python 3.14 standard-library arithmetic checks pass for four structural frame lengths, reaction reassembly, all 17 size/fragment worksheet rows, the 146-byte floor and the three public-channel SHA-256 IDs. The vector README contains the reproducible structural/count check. These are document checks, not a working codec, cryptographic verification or measured device support. MC-008/MC-022 retain crypto extension/review gates; MC-007 retains budgets and measured acceptance.

Validation: `python -B tests/ticketboard/validate.py --write`, default validator, all 12 ticketboard unit tests and `git diff --check` pass. Separate Terra medium review [5183932501](https://github.com/wickesjon/meshChat/pull/8#pullrequestreview-5183932501) reviewed revision `2e083cb652fb5d3bcb1e29e32ed2947c22d5be42` and requested two corrections: the expiry anchor for a rejected first fragment and explicit unknown-type flood scope. Both are corrected in the contract, design and vector expectations. A rejected first envelope expires 30 seconds after observation; an aborted admitted group retains its initial deadline. Unknown types follow ordinary flood TTL rules and cannot introduce direct controls. Follow-up review [5183944190](https://github.com/wickesjon/meshChat/pull/8#pullrequestreview-5183944190) found no remaining blockers at `1237fe7ae6c28ed2d10e1deb59f4af8438513b08` and reproduced the local checks. This completion staging is pending final CI and squash merge; the final reviewed revision and CI outcomes are recorded in PR #8 before merge.

## Review and merge

- Branch: `ticket/MC-006-canonical-wire-and-discovery-contract`.
- Review/PR: [PR #8](https://github.com/wickesjon/meshChat/pull/8); separate Terra medium review and follow-up are recorded there. Agent review is not an independent cryptographic/security assessment.
- Squash commit title: `MC-006: Canonical wire and discovery contract`.
- Completion becomes effective only when the reviewed squash commit lands on main.
