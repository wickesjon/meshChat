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

Not implemented. Record commands, versions, reproducible inputs and results here. For manual/hardware checks include device/OS, duration and report paths. No test or review is claimed yet.

## Review and merge

- Branch: `ticket/MC-011-channels-text-and-deep-link-parsing`.
- Review/PR: pending.
- Squash commit title: `MC-011: Channels, text and deep-link parsing`.
- Completion becomes effective only when the reviewed squash commit lands on main.
