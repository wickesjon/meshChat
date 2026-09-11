---
id: "MC-028"
title: "Android shell and channel chat"
depends_on: ["MC-018","MC-023","MC-011"]
kind: "android"
branch: "ticket/MC-028-android-shell-and-channel-chat"
---

# MC-028 — Android shell and channel chat

## Objective

Implement onboarding, Channels/Messages/Friends navigation, Afterhours/light tokens and permission/mesh states.

## Dependencies

`MC-018`, `MC-023`, `MC-011` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/android/ui/**`, `tests/integration/android-ui/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Implement onboarding, Channels/Messages/Friends navigation, Afterhours/light tokens and permission/mesh states.
- Build public/private channel lists, chat, byte-aware composer, reactions, glyphs and anonymous Confessions behavior.
- Render safe native text with separate trust chrome and local-arrival ordering; show enqueue failure and best-effort send meaning.

## Exit criteria

- [ ] Channel join/send/receive/react flows work with the real core and retain history after restart.
- [ ] Anonymous posts use disposable identity and neutral avatar; UI never calls plaintext channels encrypted.
- [ ] Accessibility, byte-boundary, unsafe-text, disconnected and rate-limited states pass UI checks.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If radio access is unavailable during UI work, use deterministic core-driven fixtures while keeping hardware validation pending.
- If a message cannot enter the bounded queue, show not-sent/retry rather than a false sent state.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Not implemented. Record commands, versions, reproducible inputs and results here. For manual/hardware checks include device/OS, duration and report paths. No test or review is claimed yet.

## Review and merge

- Branch: `ticket/MC-028-android-shell-and-channel-chat`.
- Review/PR: pending.
- Squash commit title: `MC-028: Android shell and channel chat`.
- Completion becomes effective only when the reviewed squash commit lands on main.
