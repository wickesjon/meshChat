---
id: "MC-030"
title: "Android organizer UI and staff provisioning"
depends_on: ["MC-028","MC-021","MC-041"]
kind: "android"
branch: "ticket/MC-030-android-organizer-ui-and-staff-provisioning"
---

# MC-030 — Android organizer UI and staff provisioning

## Objective

Implement event discovery prompts, canonical root adoption and staff-only credential import confirmations.

## Dependencies

`MC-028`, `MC-021`, `MC-041` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/android/ui/**`, `tests/integration/android-ui/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Implement event discovery prompts, canonical root adoption and staff-only credential import confirmations.
- Build verified updates, bounded unverified/pending drawer, signed pin controls and credential-expiry states.
- Protect sensitive provisioning screens from accidental sharing/logging and use platform-supported obscured-touch protections.

## Exit criteria

- [ ] Only adopted, valid credential chains produce staff badges and pinned authority.
- [ ] Root/staff expiry, invalid provisioning and credential recovery are reflected correctly without hidden trust changes.
- [ ] A provisioned staff device posts a signed update received through ordinary non-adopting relays.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If provisioning validation fails, preserve current trust and discard the candidate secret safely.
- If credentials expire offline, disable authoritative posting until explicitly reprovisioned; do not extend expiry.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Not implemented. Record commands, versions, reproducible inputs and results here. For manual/hardware checks include device/OS, duration and report paths. No test or review is claimed yet.

## Review and merge

- Branch: `ticket/MC-030-android-organizer-ui-and-staff-provisioning`.
- Review/PR: pending.
- Squash commit title: `MC-030: Android organizer UI and staff provisioning`.
- Completion becomes effective only when the reviewed squash commit lands on main.
