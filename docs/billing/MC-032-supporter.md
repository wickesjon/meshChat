# MC-032 Supporter implementation and activation gate

## Behavior

Supporter is a device-local, one-time non-consumable unlock. Free users get Afterhours and Daylight; Supporter adds Ember, Lagoon and Violet, custom nickname text color, and thirty instead of five private subscription slots. The three public channels are additional. Existing subscriptions survive downgrade, including subscriptions saved by older builds. Payment does not select routes, change send budgets, authorize signatures, affect encryption, or grant friend/organizer trust.

The Android feature owner keeps the validated entitlement in encrypted `supporter-v1` settings and appearance choices in encrypted `ui-cosmetics-v1`. Unknown, pending or unavailable store evidence cannot grant initial access and preserves the last validated cache. A successful reconciliation reporting no ownership removes paid access. One-time offline grace has no clock expiry; a refund is recognized when store state becomes available. Identity/data reset clears this local cache; a configured store can restore ownership afterward. Purchase acknowledgement/finish occurs only after saving local entitlement. A failed billing save reports an error without deliberately disconnecting the mesh.

The Google adapter accepts PURCHASED state only for the configured product/package with a valid store-signed JSON receipt; it neither consumes purchases nor logs receipts/tokens. It queries fresh product details and supports one permanent purchase offer, with the actual store price. The StoreKit adapter uses verified, non-revoked non-consumable transactions, transaction updates and explicit `AppStore.sync()` for restore. MC-035 connects the compiled iOS adapter and settings component to its protected feature owner and complete UI. This ticket does not present the iOS skeleton as shipping feature parity.

The implementation follows the [Google Billing integration API](https://developer.android.com/google/play/billing/integrate), [StoreKit transactions](https://developer.apple.com/documentation/storekit/transaction), and [explicit restore behavior](https://developer.apple.com/documentation/storekit/appstore/sync()). Android pins Billing 9.1.0. Its dependency graph introduces an old Fragment library; the app pins Fragment 1.8.9 to satisfy ActivityResult compatibility checks rather than disabling lint. On-device payment checks are the design's cosmetic-only policy, not a backend fraud-prevention guarantee.

## Wire and presentation

`NativeChannels.set_cosmetics` fills the existing four-byte CHAT/ANNOUNCE fields from MC-006: bit0 means RGB present, bit1 is a self-asserted Supporter hint, and reserved bits are sent zero. Clearing color sends zero RGB. There is no wire revision, new entitlement packet, receipt propagation or new trust meaning. Existing signature code authenticates the payload bytes, which establishes authorship of a hint, never payment. Receivers ignore RGB without bit0. Confessions always sends zero cosmetics and suppresses hostile incoming avatar/color/flair.

Remote flair reads “Supporter flair · unverified”, using ordinary text separate from trust labels. Custom color changes nickname text only. Four-character sender suffixes and verified labels are unaffected. Native palette checks require 4.5:1 for message/secondary/accent/error text on both background and surface, and button labels on accent. Receivers fall back to the theme text color if a nickname color misses that floor. Python checks that both platforms use identical original tokens.

## Configuration fallback and release prerequisites

No store product IDs, Google licensing public key or App Store configuration have been supplied. The ticket's explicit unavailable-store fallback applies. Android's default `PlayConfiguration` is empty and the iOS product ID defaults to nil; both block store contact and purchase activation. There is no production test-grant button, intent or preference override. Test adapters exist only under `tests/integration/billing/`; they deliver synthetic store evidence to the same protected-cache callback used by the native adapter.

Before activation, MC-039 must record actual store identifiers and signing/package/bundle matches, a reviewed permanent non-consumable product and localized price, licensing key/receipt verification, sandbox purchase/pending/cancel/acknowledgement/restore, refund/revocation, account changes, and offline restart behavior on each shipping platform. No sandbox purchase is claimed here. iOS protected-cache and full UI acceptance also remains part of MC-035/027; device certification remains with the existing physical gates. Store setup, submission, publication and actual payments require separate authorization.

## Validation record

The final ticket/PR records tested and reviewed revisions. Reproduction from the repository root (with the workspace tool/cache environment from the Windows build guide):

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo test --workspace --all-features --locked --release
cargo build --workspace --all-features --locked --release
python -B tests/integration/storage/run_policy.py
cargo-deny --all-features --locked --config src/core/deny.toml check
python -B src/core/build_bindings.py android
src/android/gradlew.bat -p src/android --no-daemon --max-workers=2 :app:assembleDebug :app:assembleRelease :app:assembleDebugAndroidTest :app:lintDebug :app:testDebugUnitTest
python -B tests/integration/android-ui/check_apk.py src/android/app/build/outputs/apk/debug/app-debug.apk src/android/app/build/outputs/apk/release/app-release-unsigned.apk
python -B tests/integration/billing/test_themes.py
python -B tests/integration/android-ui/run_emulator.py --serial emulator-5580
python -B tests/integration/android-ui/run_friends_emulator.py --serial emulator-5580
python -B tests/integration/sharing/run_android.py --serial emulator-5580
python -B tests/integration/billing/run_android.py --serial emulator-5580
python -B tests/ticketboard/validate.py
python -B -m unittest discover -s tests/ticketboard -q
git diff --check
```

`zipalign -c -P 16 4` also passes for both app APKs. Rust1.85.1 passes189 debug+189 release checks, strict lint/build and fourteen storage-policy checks; cargo-deny0.20.2 passes with only the two existing unused-license allowances. Android uses JDK17.0.15+6, Gradle8.13, Kotlin2.2.0, AGP8.11.1, SDK36/build-tools35.0.0; all sixteen JVM tests and app variants/lint pass. Reused native SQLCipher4.17.0 and Compose graphics builds are unchanged inputs with passing packaging/alignment; dependency reconstruction is not claimed as a new local build.

The named emulator is API29 x86_64 revision8, emulator37.1.11 on WHPX, adb37.0.1. Baseline channel3, friend3, sharing2 and billing purchase+reopen/refund flows pass at normal and320×640/density160 viewports. The explicit link-confirmation cap phase also passes against the downgraded thirty-channel cache. SQLCipher and provider-backed settings remain active. Six synthetic theme/flair screenshots were inspected; they stay ignored under `.work/mc032/screenshots/`. The image/runtime page size is4KiB; static16KiB ELF/ZIP alignment is not runtime16KiB certification. No physical camera, radio, OEM/background, endurance or hardware-protection result is claimed.

Debug APK SHA-256: `c35bb2e49b1aa85b361c0b7ee88ca9c394af3c085a0ca7c2088eb54b24ef26bc`. Release unsigned: `3a7253671e7d0d133e1e77125ac0bcf04603017f242fa4ae40f5df7b98b71081`. Later fixture/evidence changes do not change these production APKs.

The iOS workflow uses Xcode16.4(16F6), Swift6.1.2 on macOS15 arm64. It executes `swiftc -swift-version 6 -warnings-as-errors` for the production cache, disabled StoreKit adapter and theme regression, native Swift FFI consumer including cosmetic/trust separation and anonymous suppression, and app Debug/Release simulator builds. The initial full [Mac job](https://github.com/wickesjon/meshChat/actions/runs/35202498634/job/105140343010) passes at `36eb017`. The [final production-source job](https://github.com/wickesjon/meshChat/actions/runs/35203433056/job/105143386605) at `dd07139` passes binding generation, Swift FFI, Supporter regression, app Debug/Release and BLE builds. The only subsequent production changes were the iOS billing/UI files covered by those passing checks; security-probe/core inputs match the earlier full pass. Remaining repeated probe steps are supplemental, not falsely reported complete. StoreKit live purchase/restore/refund sandbox behavior remains untested under the declared fallback.

Prior main run35199522003 failed `FriendUiTest.kt:84` because a confirmed friend's LazyColumn row was below a320×640 viewport and the test waited without scrolling. Its retained failure screenshot showed the actual pinned state and controls. The fixture now scrolls to the already-confirmed row; the three friend phases pass at that exact local viewport. MC-032's initial hosted Android job105140343019 failed before app compilation on an HTTP503 native-dependency download. That is unavailable hosted provisioning evidence, not a passed build; the relevant local equivalent checks pass under the approved local-validation policy. Physical and independent-assessment gates are unchanged.
