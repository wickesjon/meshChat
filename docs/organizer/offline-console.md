# Offline organizer console (MC-041)

Use a dedicated offline organizer computer. The tool creates a fresh event root, exports the public adoption URI, and issues a separate random Ed25519 staff key and credential per phone. It never sends network traffic or puts a root key in a QR code. Relay beacons receive neither root nor staff private keys. This console is not a radio client or a mobile trust-adoption shortcut.

## Build and run

Use the repository's pinned Rust 1.85.1 toolchain and Python 3 with Tk support. Build while dependencies can be fetched, then disconnect before real issuance:

```text
cargo build --locked --release -p meshchat-organizer
python -B src/organizer-tools/console.py
```

The console launches the compiled helper through private local pipes. Do not invoke its internal pipe protocol from shell history, redirect its response to a file, or enable subprocess/input tracing. It has no secret command-line options. Errors contain no input data. All agent development uses disposable synthetic inputs and ignored repository-local `.work/` outputs.

1. **Create event:** enter the event name (at most 32 UTF-8 bytes) and explicit UTC event-end and root-expiry dates in `YYYY-MM-DD HH:MM` form. Root expiry cannot exceed event end plus 24 hours. Check inputs without generating a key, then confirm the actual name and expiry. Choose a new encrypted `.mcvault` file; an existing file is never overwritten.
2. **Record the unlock code once:** store the displayed 64-character random hexadecimal code securely and separately from the vault, for example on offline paper in controlled storage. It is a random 256-bit key, not a human password. The console does not save it, copy it to the clipboard, put it in a QR, or offer recovery. Close the display after recording it. A lost code requires a new root and explicit adoption of the replacement.
3. **Distribute public adoption data:** save the public `meshfest://event/...` URI from the creation dialog, or reopen the vault using **Export public adoption link**. Encode that public URI with an approved offline QR workflow for event posters. Compare the event/root identity through the organizer's trusted channel before distribution. A self-signature proves integrity, not that a root belongs to the real event.
4. **Provision staff:** select the encrypted root, enter its unlock code, a nonempty staff label (at most 16 UTF-8 bytes), and explicit UTC shift start/end. The end must be in the future and no later than the root expiry or event end plus 24 hours. Prefer short shifts (the design recommends at most 12 hours). **Check without issuing** validates the request and creates no staff key. The unlock input is cleared after use; re-enter it to issue.
5. Confirm the event, label and dates. The private staff QR is generated in memory and shown once for at most 60 seconds. Scan it directly with the intended staff phone, then select **Scanned · erase code now**. No private URI, QR image or staff seed is saved or copied. A closed or missed display cannot be reopened; issue a new key if needed. The staff phone must separately confirm the root and provisioning through its canonical protected importer. Shipping mobile UI/lifecycle integration belongs to MC-030/035; native synthetic import fixtures are not device certification.

The desktop display cannot prevent cameras, OS screenshots, screen sharing, debugger access or a compromised computer. Tk/Python and FFI may retain memory copies after widgets close; the console does not claim complete process-memory erasure or hardware-backed key storage. Keep provisioning local and physically controlled, disable recording through the operator's normal process, close the console after use, and protect the organizer computer. No machine setting is changed by the tool. This limitation does not permit keys on beacons.

## Local vault format and boundaries

The vault is a hex encoding of `meshchat/offline-root/v1\0`, a fresh 12-byte nonce, and ChaCha20-Poly1305 ciphertext/tag. The exact magic is authenticated as associated data. Plaintext is a 32-byte root seed, four-byte big-endian root expiry, four-byte big-endian event end and the validated UTF-8 event name. Every creation uses independent operating-system randomness for the root seed, 32-byte unlock key and nonce. Opening checks authentication and expiry before signing. The root private seed never crosses the Rust-to-console boundary. Mutable metadata and staff issuance use the existing MC-006/008 wire grammar and domains unchanged.

Rust secret buffers use zeroizing ownership and the existing pinned Ed25519/ChaCha20-Poly1305 implementation. The new [qrcode 0.14.1 encoder](https://github.com/kennytm/qrcode-rust) uses its matrix-only configuration; no image encoder or network dependency is added. Native fixtures decode the generated matrix using the same ZXing version used by the Android scanner. Public names and labels are validated by the core text policy; a missing/blank staff label is additionally refused by this issuance UI.

## Rotation, expiry and rehearsal

- **Staff rotation:** issue a fresh staff key with the next explicitly chosen validity window. Expired or not-yet-valid credentials grant no authority. Overlap is an operator decision; validity is never silently extended. Close/forget the old mobile session under the mobile lifecycle workflow. Issuing a new credential does not remotely revoke an older unexpired one.
- **Root rotation or compromise:** create a new root, authenticate and distribute its new public adoption data, and explicitly replace/adopt it on phones. Reissue each staff credential under the new root. Old roots remain valid until removed or expired; there is no implicit mesh revocation or automatic replacement. Destroying the organizer's vault does not revoke copies or already-issued credentials.
- **Lost staff QR:** each new issuance is a different key; the earlier credential remains potentially valid through its stated end if someone scanned it. Treat unwanted exposure as a credential compromise. The protocol has no hidden remote revocation switch.
- **Offline rehearsal:** generated synthetic root/staff bundles go through the real parser, confirmed adoption, `Organizer::import_staff`, and `Organizer::sign`. Two staff identities send signed updates through a non-adopting node and the ordinary bounded relay scheduler, including TTL decrement and fragmentation. The receiving node adopts the root and verifies both staff labels; the relay acquires neither adopted root nor authenticated history. Negative fixtures cover mismatched root/seed, malformed input, future and expired shifts, tampered/wrong-key vaults, and invalid labels/validity.

Run `cargo test --locked -p meshchat-organizer`, then the console protocol checks with `python -B tests/integration/organizer-tools/check_console.py` after a release build. The Rust tests generate disposable, random native fixtures in `.work/mc041/fixtures.tsv`; they contain synthetic staff seeds and must never be committed, logged, reused for real provisioning or uploaded as artifacts. Kotlin/Swift fixtures invoke the test-only canonical import adapter, not a duplicate mobile verifier. Detailed final commands, revisions and outcomes are recorded in the ticket/PR. Independent release security assessments and physical storage/provisioning gates remain mandatory.
