# MC-035 native acceptance

`native.rs` exercises the canonical native SYNC requester with actual frames,
bounded sessions, stored TTL-zero parsing, signed friend CHAT and encrypted DM.
Run `cargo test -p meshchat-core --test native_catchup --locked`.

On a Mac with the repository's pinned Xcode/Rust/SQLCipher inputs, run the iOS CI
steps through the unsigned production application build, then run
`python3 -B tests/integration/ios-ui/run_apple.py`.

The runner derives a separate test application project from the production
target under `.work/ios-ui/`. It replaces only the entry point and adds test
fixtures. Production SwiftUI, model, encrypted SQLCipher storage, Rust core,
driver, framing and cryptography remain in the test application. Synthetic
wrapping and the framed radio port are confined to this test directory and are
never compiled into the shipping app. The test bundle identifier is separate.

`FeatureChecks.swift` tests confirmation, persistence, signed/plaintext/DM trust
boundaries, encrypted reactions, native egress invalidation, replacement and
read-only history, QR decoding, event/staff lifecycle, offline entitlement cache,
reset and key loss. `UITests.swift` uses XCUITest to operate the shipping views
and retains screenshots containing only disposable public test data. Generated
staff credentials are not screenshots or uploaded fixtures.

The result bundle and build/test log stay under `.work/ios-ui/`; a passing result
requires both tests to pass. Compilation alone is not acceptance. These checks
do not certify physical Secure Enclave operation, camera scanning, Bluetooth
interoperability, background restoration or battery performance. MC-027/044
retain those real-device gates.
