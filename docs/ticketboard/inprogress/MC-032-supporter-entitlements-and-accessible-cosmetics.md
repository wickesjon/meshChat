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

Permitted paths (relative to repository root): `src/android/billing/**`, `src/android/ui/**`, `src/ios/Billing/**`, `src/ios/UI/**`, `tests/integration/billing/**`. User-approved extension (2026-09-17): Android app build/manifest/activity wiring, iOS project build wiring, `src/core/src/native_channels.rs`, `src/core/Cargo.toml` test registration, native integration tests, `docs/billing/**`, and `.github/workflows/ci.yml` validation wiring.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Implement free/paid theme tokens, subscription-slot convenience and permitted local/remote cosmetics under the canonical wire decision.
- Integrate platform purchase/restore adapters with cached offline entitlement and documented refund/revocation behavior.
- Keep mesh participation, trust, friend verification and DMs independent of payment.

### Implementation choices and pending integration

- Select a one-time, non-consumable Supporter unlock, permitted by design §19.2. Free profiles may add up to five private subscriptions; Supporter profiles may add thirty, in addition to the three public channels. Downgrade retains existing subscriptions and their messaging; only adding another slot uses the cap.
- Unknown, pending, disabled and failed verification results preserve previously validated local entitlement but cannot grant a first entitlement. A successful store reconciliation reporting no current purchase removes paid access. Offline grace has no clock deadline for this one-time unlock; refund recognition waits for a store reconciliation. No entitlement state enters relay, encryption, friend or organizer verification.
- Android verifies the store-signed purchase JSON with the configured RSA public key before accepting PURCHASED state for the configured product and package. The store acknowledgement follows successful protected-cache persistence. iOS accepts StoreKit verified, non-revoked non-consumable transactions and finishes them after saving. Both adapters are inactive without product configuration. Native adapters follow the [Google integration API](https://developer.android.com/google/play/billing/integrate) and [StoreKit transaction API](https://developer.apple.com/documentation/storekit/transaction).
- Define two free and three original paid palettes. Text, secondary text, accent and error colors must meet 4.5:1 against both message surfaces; button text must meet 4.5:1 against its accent. Unreadable custom nickname colors fall back to the theme text color. Sender suffix and trust labels remain outside sender-controlled styling.
- **Explicit scope approval received 2026-09-17:** the user answered “Yes” to the requested integration and Mac validation extension. This approval covers the paths above; production store activation remains disabled pending real configuration and sandbox evidence.

### Store-configuration fallback

No real store product IDs, Android licensing public key, or Apple product configuration have been supplied. The ticket's unavailable-store-configuration fallback is triggered: empty configuration blocks production store contact and purchase activation. Synthetic receipt, policy and adapter checks are development evidence only. Actual Google/Apple sandbox purchase, restore, refund and offline restart checks remain outstanding; MC-039's store-configuration gate must retain them before activation/distribution. MC-035 owns integration of the iOS billing cache and controls into its feature UI. No live store setup, payment or publication is authorized or performed here.

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

Preliminary implementation in progress; no PR, review or completion is claimed.

- Android: four JUnit checks cover entitlement transitions, new-slot limits, all five palettes and nickname fallbacks, and rejection of altered, wrong-key, oversized and malformed synthetic receipts. The Google Play adapter compiles against pinned Billing 9.1.0. JDK 17.0.15+6, Gradle 8.13, Kotlin 2.2.0 and AGP 8.11.1; local log `.work/mc032/policy.log`. Temporary init-script test wiring stays in ignored `.work/mc032/` because app build scope is still pending. Reproduction after scope approval must use committed build/test wiring before review.
- iOS: entitlement policy, StoreKit adapter and policy regression source are written but not yet compiled or run. A Mac native check is required and remains blocked on validation wiring approval.
- Remaining: app integration and protected persistence, canonical CHAT/ANNOUNCE cosmetic adapters, anonymous suppression, UI/slot/restore regression, native builds/lint/emulator checks, evidence documentation, actual Terra review and squash merge.

## Review and merge

- Branch: `ticket/MC-032-supporter-entitlements-and-accessible-cosmetics`.
- Review/PR: pending.
- Squash commit title: `MC-032: Supporter entitlements and accessible cosmetics`.
- Completion becomes effective only when the reviewed squash commit lands on main.
