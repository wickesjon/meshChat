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

Permitted paths (relative to repository root): `src/core/**`, `tests/vectors/crypto/**`, `tests/integration/dm/**`, `tests/simulator/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Implement the selected authenticated encryption construction with exact plaintext/padding, suite parameters and immutable-header bindings.
- Implement rotating tag lookup with collision handling, friend-key validation, clock-boundary behavior and encrypted reactions.
- Integrate blind relay/SYNC and encrypted local persistence; authenticate UI identity from the pin/key proof.

## Exit criteria

- [ ] Integrated ingress and real production verification of encrypted CHAT/REACTION pass invalid-first/cryptographically-valid-second same-ID and identical-authenticated-replay cases. Later valid acceptance remains possible once budget is available; replay repeats no effect or trust refresh. Include failure, eviction, concurrent arrivals and a budget-available recovery phase; no test-verifier substitute.
- [ ] Reference/KAT, sender-forgery, all-zero X25519, malformed-key and header/ciphertext tamper vectors pass.
- [ ] Relays cannot decrypt; valid DMs and reactions round-trip across the simulator and restart safely in encrypted history.
- [ ] Epoch skew/collision, replay and changed-key tests fail closed without plaintext downgrade.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If the crypto review rejects the construction, block the feature and revise MC-008 before proceeding.
- If recipient key/tag resolution fails, keep bounded opaque relay behavior and do not display a guessed sender or plaintext.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

The user-approved 2026-09-15 MC-013 sequencing correction assigns the real ingress/crypto replay gate here. MC-013 covers admission and pending/unverified state tests only; MC-022 requires these integrated results before full-wire freeze. No criterion is marked passed by this scheduling change.

Not implemented. Record commands, versions, reproducible inputs and results here. For manual/hardware checks include device/OS, duration and report paths. No test or review is claimed yet.

## Review and merge

- Branch: `ticket/MC-020-authenticated-encrypted-dms-and-reactions`.
- Review/PR: pending.
- Squash commit title: `MC-020: Authenticated encrypted DMs and reactions`.
- Completion becomes effective only when the reviewed squash commit lands on main.
