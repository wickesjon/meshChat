# Offline sharing and link activation

Android channel info offers a white QR card, three large words, PNG export, share sheet and explicit copy. Friends use the same QR/export controls. QR uses the canonical `meshfest://j/...` or `meshfest://friend/...` route and four-module quiet zone; there is no network permission in the app. Both scanner results and incoming links lead to native confirmation. Friend codes remain public key proposals; the complete fingerprint and private petname confirmation establish the pin. A link received online carries the stronger source warning; friend replacement still needs a fresh scan.

Channel parsing uses the existing MC-006 parser through the `channel_link` native export. There is no second channel-word or URI parser on Android. The fallback page only constructs an untrusted proposal; the native core validates it again. Native friend handling validates the full keys, including cryptographic checks that the fallback page deliberately does not claim to perform. Organizer/staff provisioning is outside this sharing surface, and no staff URL or private material can be exported.

## What works without internet

| Situation | Behavior |
|---|---|
| Both apps installed; in-app QR scan | Exact channel/key proposal and explicit confirmation, fully offline |
| Channel words spoken aloud | Select the same three words; channel is readable by anyone with the words |
| Custom-scheme link opened by installed Android app | Same confirmation; other apps can register the scheme, so prefer the in-app scanner |
| HTTPS handed to the app | Same native confirmation; this does not prove website association |
| App absent, page already loaded | Page retains the proposal and explains installation; no automatic redirect or installed-app inference |
| App absent, online, configured store listing | User follows store link, then returns to the original link or QR; no deferred-link transfer through the store |
| Fresh page/install offline | Unavailable; ask someone with the installed app to keep the QR/words |

HTTPS automatic handling is **not activated or certified**. The current app uses unverified filters and honest share copy. The unavailable-identifiers fallback in MC-031 is active; no production domain, signing certificate, Apple app identifier or store listing has been supplied. No domain purchase, association upload, deployment or store publication is authorized by this implementation.

## Local build and checks

From the repository root:

```text
python -B src/share-site/build.py
python -B -m unittest discover -s tests/integration/sharing -p "test_*.py" -v
node tests/integration/sharing/site.test.mjs
python -B tests/integration/sharing/serve.py
```

The preview binds only `127.0.0.1:8765`; open `/j/melodic-techno-valley`. Outputs stay in `.work/mc031/site`. Missing IDs produce empty association grants and no store links. The native templates under its `native-templates/` directory are review inputs, never automatically installed into a project. Fixture IDs in tests are synthetic and must never be published. `serve.py` is only a development fixture, not a production web server.

To configure reviewed fixtures, copy `src/share-site/config.example.json` into an ignored repository path and pass `--config PATH`. Supply `domain`, Android package and SHA-256 signing certificate fingerprints, Apple app prefix plus bundle ID, and actual HTTPS store URLs. Store URLs never include channel words or friend names. The builder emits `.well-known/assetlinks.json`, `.well-known/apple-app-site-association`, Android intent-filter and Apple entitlement templates. Word lists come directly from the frozen core.

The frozen native parser recognizes **only `meshfest.app` for HTTPS**. A different configurable domain can be rendered for association review, but is not a supported app URL until a separately approved compatibility decision updates and tests the native contract. Do not mistake template configurability for a parser change or domain ownership.

## Production activation checklist — blocked

1. Obtain explicit domain/store/deployment authorization and verified ownership; provide production identifiers, including the Play app-signing certificate rather than an unrelated upload/debug certificate.
2. Serve the reviewed static page for `/j/*` and `/friend/*` while preserving the original browser path. Serve the two association files over HTTPS with `application/json`, no redirect/authentication, and correct host. Do not serve `native-templates/` as public content.
3. Keep all assets same-origin. Preserve the page CSP and no-referrer policy; add response `frame-ancestors 'none'`, `X-Content-Type-Options: nosniff`, and no-store for proposal pages. Disable access/analytics/crash logs containing paths or names. The initial HTTPS request exposes its path to the hosting service; private channel words are a sharing secret, not encrypted data. The page uses no analytics, cookies, storage, fetch, or third-party assets.
4. Review/merge the matching verified Android filter and iOS associated-domain entitlement into the actual signed applications. iOS scene URL/Universal Link receipt, QR camera UI, confirmation and sensitive pasteboard handling integrate in MC-035; do not enable associations for a skeleton that cannot complete the flow. iOS copy must be local-only with a bounded expiry, with no background pasteboard reads.
5. On the supported signed native/device matrix, record OS/app build, domain, certificate/app IDs, association retrieval and OS verification result. Test installed app on/offline, no app online/offline, post-install return, invalid/extra components, user-disabled association, camera denied, canceled confirmation and scanned key equality. Android: inspect `pm get-app-links` after actual verification; never manually force a success state. iOS: inspect real association and Universal Link behavior, including same-domain Safari behavior and CDN caching.
6. Record the real production verification before changing share copy. Real camera optics and native/iOS acceptance remain MC-025/027 and MC-035; protected device evidence remains MC-043/044. Local templates and emulator checks do not satisfy those gates.

Android QR exports use only `cache/share-qr/` through a non-exported FileProvider with temporary read grants. Each export uses a distinct filename so a later share cannot replace an earlier image. On export, files older than 24 hours are removed; at 32 pending files export refuses and on-screen QR remains available. Cache files are not backed up. Clipboard entries carry Android's sensitive-content flag, which hides supported clipboard previews; it does not prevent other apps from reading the clipboard. UI explains this and favors QR/share sheets.

Platform references checked 2026-09-17: [Android association configuration](https://developer.android.com/training/app-links/configure-assetlinks), [Android verification](https://developer.android.com/training/app-links/verify-applinks), [Android clipboard flag](https://developer.android.com/develop/ui/compose/touch-input/copy-and-paste), [Apple associated domains](https://developer.apple.com/documentation/xcode/supporting-associated-domains), and [Apple Universal Link troubleshooting](https://developer.apple.com/documentation/technotes/tn3155-debugging-universal-links).
