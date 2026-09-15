---
id: "MC-018"
title: "Encrypted persistence and retention"
depends_on: ["MC-017","MC-011"]
kind: "security"
branch: "ticket/MC-018-encrypted-persistence-and-retention"
---

# MC-018 — Encrypted persistence and retention

## Objective

Define and implement migrations for messages, subscriptions, friend pins, DM conversations, trust provenance, event roots/credentials, settings and reaction state where persistent.

## Dependencies

`MC-017`, `MC-011` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/core/**`, `src/android/security/**`, `src/ios/Security/**`, `tests/integration/storage/**`, `.github/workflows/ci.yml` (user-approved storage test wiring).

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Define and implement migrations for messages, subscriptions, friend pins, DM conversations, trust provenance, event roots/credentials, settings and reaction state where persistent.
- Use SQLCipher through the selected key provider, parameterized SQL and explicit platform backup exclusions. Under the user-approved MC-005 deferral, physical restore/exclusion verification belongs to MC-043/044.
- Implement continuous/launch pruning, deletion and identity-reset coordination; distinguish local history from bounded forward caches.

## Exit criteria

- [ ] Wrong-key reads fail; plaintext message/key markers are absent from the database and auxiliary files in controlled tests.
- [ ] Restart preserves appropriate history and trust state; rotating DM tags do not split a conversation.
- [ ] Synthetic migration rollback/failure, retention, deletion and backup-configuration tests pass for both platform integrations. Physical backup/transfer/restore tests remain mandatory at MC-043/044 before real sensitive-data use.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If migration fails, preserve the encrypted original and present a recoverable error; do not recreate silently.
- If encrypted storage is unavailable, disable persistence-dependent features instead of writing plaintext.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Scheduling decision: the user deferred MC-005 physical verification on 2026-09-11. This implementation ticket uses synthetic data and automated tests; MC-043/044 retest the production provider/storage on actual devices. Its completion does not certify hardware, backup or lock behavior.

Implemented candidate; native integration checks and Terra review are pending. This ticket is not complete.

### Started 2026-09-15; CI scope approved

Dependencies are complete on main at `e9974acb7fcc47d1e8c503342bf568db780d0d05` (MC-017, PR #21). Dedicated MC-018 branch created from that revision. Design, MC-005/008 contracts and MC-017 reset integration were inspected before implementation.

The user approved the proposed CI scope addition on 2026-09-15. `.github/workflows/ci.yml` is permitted for executing new storage integration tests on the existing isolated Android emulator and Mac runners. The scripts and test-runner registration remain within the original ticket paths. This changes no runner/account settings or physical gates. The reviewable proposal is `.work/mc018/ci-scope-proposal.md`.

### MC-018 storage model and lifecycle decision

Shared Rust `EncryptedStore` owns the schema and parameterized SQL, migrations, transactions, bounded history, replay ledger and clock high-water state. Native `CipherConnection` adapters execute only bound SQL against the already pinned SQLCipher libraries. No dependency versions or wire formats change. The generated database callback and Rust store object are trusted provider-internal objects: application wrappers expose typed operations and never return a connection, passphrase or unlocked store object. Protocol owners MC-019/020/021 must authenticate before `accept_authenticated`; storage provenance alone does not verify signatures, establish presence or authorize root adoption.

Schema version 1 contains a single nonzero 16-byte local identity generation, bounded records and message history. Version 2 transactionally adds the replay ledger and persisted clock state. Fresh creation is explicit; existing version 0, unknown/future versions or a mismatched identity generation refuse. The version-1 upgrade is a defined migration fixture, not a claim that an earlier released database exists. Failed schema work rolls back without deleting/recreating the encrypted original.

| Persistent data | Representation and limits |
|---|---|
| Settings | Record kind 0, nonempty key up to 64 bytes; owning feature supplies bounded serialized value |
| Subscriptions | Kind 1, canonical 4-byte channel ID, at most 32 |
| Friends / petnames / verification provenance | Kind 2, full ordered Ed25519/X25519 tuple (64 bytes), at most 128; value preserves the owning friend feature's canonical pin/petname/provenance record |
| Explicitly adopted event roots | Kind 3, full 32-byte root key, at most 64; adoption metadata in value |
| Staff credentials | Kind 4, full root/staff key pair (64 bytes), at most 256; canonical credential/provenance in value |
| Record values | Opaque bytes interpreted by their feature owner; at most 2048 bytes each, parameterized BLOB storage; settings at most 64 |
| History and reaction state | CHAT type 1 / REACTION type 6, immutable 8-byte message ID, timestamp, direction, body up to 1024 bytes, provenance up to 512 bytes; channel key is canonical 4-byte ID; DM conversation is the full peer tuple within the database's local identity generation, independent of rotating network tags |
| History retention | Age 48h; at most 5000 per channel, 1000 per DM conversation; global history at most 20000 rows / 32 MiB body+provenance; DMs independently at most 5000 rows / 8 MiB. Age pruning on open and operations; per-conversation pruning on insertion. Global saturation refuses the transaction until deletion/pruning frees room |
| Replay ledger | Domain plus full signer/peer/root-staff identity, direction, logical type and message ID; digest of authenticated immutable bytes, timestamp and conflict bit. At most 100000 live records. Exact replay has no effect; conflict retains the first result and marks ambiguity. History deletion leaves the ledger intact; only expired entries are pruned |
| Clock | Local high-water timestamp and uncertainty flag; missing/invalid time or backwards jump beyond 300s persists uncertainty, refusing time-dependent effects. After uncertainty, recovery requires reaching the stored high-water mark. Pruning uses accepted monotonic high-water time, never peer timestamps |

The forward cache, reassembly, pending verification, orphan reactions, credential-recovery requests and live/session trust remain transient under their existing core budgets; none are reconstructed as authenticated presence from history. Reaction target ambiguity checks the retained ledger across logical types/directions. The calling protocol owner performs authentication and target validation before storage; no unverified DM/provenance write is allowed through `append_unverified`.

Each native operation loads the existing unlocked identity, checks the database envelope's bound generation, unwraps an independent 64-byte SQLCipher passphrase, opens the connection, performs shared-core work, closes it and rechecks lock state before returning. The passphrase is independent of both curve seeds and is protected with its own native wrapping-key alias/tag. Envelopes reuse MC-017's 64-byte authenticated wrapping format and are bounded. SQLCipher is required by `PRAGMA cipher_version`; database mode uses DELETE journals, FULL synchronization, in-memory temporary storage, secure deletion and a 16384-page ceiling. Managed/native runtime copies remain a best-effort cleanup limitation; no full memory-erasure claim is made.

`StorageVault` implements MC-017's mandatory durable/idempotent reset callback. Identity reset writes its recovery marker before the vault writes a storage marker, deletes the separate wrapping key and known database/auxiliary files, syncs directory entries and retires the storage directory; only then can both identity keys rotate. No connection outlives an operation. Android shares the identity operation lock across provider instances in the single application process; Swift uses MainActor serialization. Unknown retained files refuse reset completion rather than triggering recursive deletion. Missing/lost keys preserve ciphertext until explicit reset. The callback owns no silent storage recreation. It also requires a provider-internal reset-in-progress guard, enabled only after the identity recovery marker is written; directly invoking storage clearing refuses before changing data. This prevents retiring replay records while retaining the old identity keys.

Android uses `noBackupFilesDir`, existing backup-disabled/extraction-exclusion configuration and AndroidKeyStore unlocked-device AES-GCM. iOS uses its protected/excluded application-support directory, complete database protection and the required Secure Enclave wrapping policy. The filesystem primitive can now exercise actual exclusion attributes on the Mac host with injected test wrapping; the production Apple key provider still refuses host operations and has no software wrapping fallback. Physical backup/transfer/lock behavior remains MC-043/044.

### Candidate validation, 2026-09-15

Local Windows Rust 1.85.1/cargo 1.85.1 formatting, clippy, 84 debug and 84 release checks, release build and cargo-deny 0.20.2 pass; lockfile unchanged. Python 3.14.4 `tests/integration/storage/run_policy.py` generates host-only Python UniFFI bindings and passes 11 shared-policy tests using an explicitly unencrypted SQLite double. They cover restart, record and conversation limits, reaction ambiguity, 100000-record ledger saturation without eviction, exact replay/conflict after history deletion, atomic effect rollback, five migration-failure points, strict tombstone expiry and persisted clock uncertainty/recovery. This does not prove encryption or native behavior.

Both actual Android source adapters and the new instrumentation runner compile with Kotlin 2.2.0/JDK 17.0.15+6/API 36 stubs and the pinned SQLCipher 4.17.0/SQLite 2.5.2/JNA 5.17.0 classes. Shared Rust and MC-017 Kotlin identity regression remain applicable. Local commands/logs:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo test --workspace --all-features --locked --release
cargo build --workspace --all-features --locked --release
cargo-deny --all-features --locked --config src/core/deny.toml check
python -B tests/integration/storage/run_policy.py
python -B tests/ticketboard/validate.py --write
python -B tests/ticketboard/validate.py
python -B -m unittest discover -s tests/ticketboard -v
git diff --check
```

Local logs are `.work/mc018/rust-debug.log` and `rust-release.log`; 46-ticket/127-dependency validation and 12 board tests pass. The final review-source hash will be recorded in PR evidence.

Initial hosted run `34969204506` passed Rust/ticketboard and iOS native device/simulator builds plus existing Swift regressions. The new standalone Swift storage compilation failed because its command omitted `-DSQLITE_HAS_CODEC`; the test runner now includes the same flag as the Xcode project. Native storage tests also inspect the rollback journal during an open transaction, requiring its presence and checking plaintext/passphrase absence. Android test failures include a fixed stage label and exception class without error text or data. These test-only corrections are pending native rerun; they are not passing evidence.

A further lifecycle check identified that the public reset callback could otherwise be invoked directly. Both native providers now guard that callback within the journaled identity-reset window; native tests assert direct calls refuse and preserve stored pins. The Android compile and existing MC-017 Kotlin lifecycle regression pass after this source fix. It requires native rerun and follow-up review before merge.

Required pending native gates: existing full Android builds/lint/ABI/alignment and security-emulator checks plus MC-018's separately registered instrumentation runner; pinned Xcode device/simulator builds, existing Swift regressions and the new process-separated Swift/SQLCipher suite. New native fixtures test create/reopen, actual wrong-key read refusal, plaintext-marker/passphrase absence in database/auxiliary files, retained identity/trust, replay/conflict, rollback, pruning, key-loss retention and coordinated reset. Swift uses the pinned SQLCipher macOS framework with test-only wrapping and the actual filesystem backup attributes; Android uses the actual provider in the isolated emulator. No native test or physical result is claimed until these checks run successfully.


## Review and merge

- Branch: `ticket/MC-018-encrypted-persistence-and-retention`.
- Review/PR: [PR #22](https://github.com/wickesjon/meshChat/pull/22). Terra medium reviewed `4f2f95569617a2b1c49b2165463d9e097cfffc99` with no findings. The worker posting attempt was rejected by automatic approval review; its completed review was accurately transcribed by the primary agent in COMMENT review #5209979292. Follow-up test fixes and final evidence require follow-up review.
- Squash commit title: `MC-018: Encrypted persistence and retention`.
- Completion becomes effective only when the reviewed squash commit lands on main.
