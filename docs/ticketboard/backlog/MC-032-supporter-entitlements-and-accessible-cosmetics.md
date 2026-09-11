---
id: "MC-032"
title: "Supporter entitlements and accessible cosmetics"
depends_on: ["MC-028","MC-006"]
kind: "product"
branch: "ticket/MC-032-supporter-entitlements-and-accessible-cosmetics"
---

# MC-032 — Supporter entitlements and accessible cosmetics

## Objective

Implement free/paid theme tokens, subscription-slot convenience and permitted local/remote cosmetics under the canonical wire decision.

## Dependencies

`MC-028`, `MC-006` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/android/billing/**`, `src/android/ui/**`, `src/ios/Billing/**`, `src/ios/UI/**`, `tests/integration/billing/**`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Implement free/paid theme tokens, subscription-slot convenience and permitted local/remote cosmetics under the canonical wire decision.
- Integrate platform purchase/restore adapters with cached offline entitlement and documented refund/revocation behavior.
- Keep mesh participation, trust, friend verification and DMs independent of payment.

## Exit criteria

- [ ] Store sandbox purchase/restore and cached offline use pass for each shipping platform.
- [ ] Every theme passes the selected contrast checks and cannot mimic verified/staff chrome.
- [ ] Payment failure never disables message relay or encryption; unverifiable paid flair remains cosmetic.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If store configuration is unavailable, use test adapters and keep production purchase activation blocked.
- Apply the documented grace behavior to previously validated entitlements; do not grant trust based on paid status.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Not implemented. Record commands, versions, reproducible inputs and results here. For manual/hardware checks include device/OS, duration and report paths. No test or review is claimed yet.

## Review and merge

- Branch: `ticket/MC-032-supporter-entitlements-and-accessible-cosmetics`.
- Review/PR: pending.
- Squash commit title: `MC-032: Supporter entitlements and accessible cosmetics`.
- Completion becomes effective only when the reviewed squash commit lands on main.
