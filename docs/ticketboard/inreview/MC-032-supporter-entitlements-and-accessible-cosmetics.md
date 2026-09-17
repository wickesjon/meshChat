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

Implementation is reviewable; final native checks and review remain pending. Durable behavior, reproduction and activation prerequisites: [MC-032 evidence](../../billing/MC-032-supporter.md).

- Source `36eb017`: Rust 1.85.1 formatting, strict clippy, 189 debug + 189 release tests, release build, fourteen protected-storage policy checks, cargo-deny 0.20.2 advisory/license/source/version gates pass. Three new core checks cover canonical fields, hostile anonymous suppression and untrusted flair. Wire/cryptographic formats are unchanged.
- Android normal bindings/arm64+x86_64 libraries, app Debug/Release, test APK, lint and all sixteen JVM tests pass with committed wiring. Four billing JVM checks cover entitlement transitions, slots, five palettes and nickname fallback, and altered/wrong-key/oversized/malformed synthetic receipts. JDK 17.0.15+6, Gradle 8.13, Kotlin 2.2.0, AGP 8.11.1, API36/build-tools35.0.0. Billing 9.1.0's old Fragment transitive dependency initially failed ActivityResult lint; pinning Fragment1.8.9 resolves it without suppressing the check.
- API29 x86_64 emulator37.1.11 on WHPX: channel3, friend/DM3, sharing2 and billing purchase/cache-restart/refund2 phases pass. Real SQLCipher/provider adapters are used. All thirty private slots survive downgrade, a thirty-first addition refuses, all themes render, and unverified remote flair retains unverified trust. Six synthetic screenshots inspected. Native ELF/ZIP16KiB alignment passes; runtime pages are4KiB, not16KiB certification.
- Mac job [35202498634](https://github.com/wickesjon/meshChat/actions/runs/35202498634) on `36eb017` has passed new Swift cache/disabled-StoreKit/theme checks, shared FFI cosmetics and app Debug/Release compilation; remaining probe steps were running when recorded. Later verified-revocation/SwiftUI edge handling requires a new native check before merge.
- Supplemental hosted Android on `36eb017` failed before app compilation on HTTP503 while downloading a native build dependency; it is unavailable evidence, not a passed check. Local equivalent relevant builds/tests pass.
- Prior main run35199522003 failed a real friend-test timeout at `FriendUiTest.kt:84`: its retained320×640 screenshot showed the confirmed friend below the unscrolled LazyColumn viewport. The fixture now explicitly scrolls after the pin exists in model state. Small-viewport rerun and final link-cap regression remain pending when recorded.
- Ticketboard default validation, twelve board tests, native palette parity and whitespace checks pass. Logs: `.work/mc032/`. No real store sandbox, physical certification, publication or independent-security-assessment result is claimed.

## Review and merge

- Branch: `ticket/MC-032-supporter-entitlements-and-accessible-cosmetics`.
- Review/PR: pending.
- Squash commit title: `MC-032: Supporter entitlements and accessible cosmetics`.
- Completion becomes effective only when the reviewed squash commit lands on main.
