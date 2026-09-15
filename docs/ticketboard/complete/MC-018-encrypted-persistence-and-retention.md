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

- [x] Wrong-key reads fail; plaintext message/key markers are absent from the database and auxiliary files in controlled tests.
- [x] Restart preserves appropriate history and trust state; rotating DM tags do not split a conversation.
- [x] Synthetic migration rollback/failure, retention, deletion and backup-configuration tests pass for both platform integrations. Physical backup/transfer/restore tests remain mandatory at MC-043/044 before real sensitive-data use.
- [x] Relevant checks pass, evidence is recorded and the source review is complete. This is a staged merge candidate: ticket completion becomes effective only when the final reviewed squash commit lands on main.

## Potential fallbacks

- If migration fails, preserve the encrypted original and present a recoverable error; do not recreate silently.
- If encrypted storage is unavailable, disable persistence-dependent features instead of writing plaintext.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Scheduling decision: the user deferred MC-005 physical verification on 2026-09-11. This implementation ticket uses synthetic data and automated tests; MC-043/044 retest the production provider/storage on actual devices. Its completion does not certify hardware, backup or lock behavior.

Implemented and tested source `8e970761b345e4372865323b0409eb11fd784873`. Required source reviews and shared/native checks passed. Completion is staged for the final reviewed squash merge and becomes effective only on main.

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

### Native validation and resolved failures

All four jobs in [CI run 34973787716](https://github.com/wickesjon/meshChat/actions/runs/34973787716) passed on reviewed source `8e970761b345e4372865323b0409eb11fd784873`:

| Job | Evidence |
|---|---|
| Rust `104396284676` | Pinned Rust 1.85.1 formatting/clippy, debug/release tests and build, lockfile consistency, 11 shared SQLite-double policy tests and dependency gates |
| Ticketboard `104396284579` | Generated index/DAG consistency and board tests |
| Android `104396284143` | JDK 17.0.15+6, Kotlin 2.2.0, AGP 8.11.1, NDK 27.3.13750724, compile SDK 36; Debug/Release builds, lint, both native ABIs and alignment checks; existing MC-005 phases and all five MC-018 phases on isolated API 29 emulator with 4096-byte pages |
| iOS `104396284535` | Xcode 16.4, Swift 6.1.2 in Swift 6 mode, iOS SDK 18.5; unsigned device/simulator builds and existing FFI/identity/curve regressions; all five MC-018 Swift/SQLCipher phases on the Mac host |

The native suites execute `create`, `reopen`, `checks`, `key-loss` and `reset` in separate app processes. They verify retained identity/pins/history; actual wrong-key refusal; plaintext-marker/passphrase absence in the database and auxiliary files, including an active rollback journal; atomic history/ledger rollback; synthetic v1 migration rollback and upgrade; age pruning, clock uncertainty and history deletion without replay acceptance; retained ciphertext after wrapping-key loss; and coordinated identity/storage reset with stale-handle refusal. Calling the storage-reset callback directly refuses without deleting pins. Android checks its real no-backup path and disabled backup flag. Mac checks the real filesystem backup-exclusion attribute.

Android uses the actual AndroidKeyStore provider and pinned SQLCipher 4.17.0 build. Swift uses the actual pinned SQLCipher framework and production filesystem adapter with test-only wrapping; it does not exercise production Secure Enclave wrapping on the Mac. Neither emulator nor host tests establish physical backup/transfer/restore, hardware key protection or lock behavior. Those remain MC-043/044. Controlled synthetic v1 migrations are fixtures, not an upgrade claim for an earlier released app.

Reproduction commands are the existing workflow build/lint/package gates plus:

```text
python -B src/core/build_bindings.py android --security-probe
python3 -B tests/integration/storage/run_android.py --serial emulator-5554
python3 -B tests/integration/storage/run_apple.py
```

Local Windows cross-compilation passed for arm64-v8a and x86_64 after the final production correction, with logs in `.work/mc018/android-cross.log`. Hosted checks are required here because both native consumers and the FFI surface changed. The final ticket/index-only completion update changes no tested source, dependency, binding, build configuration or runtime fixture; the passing native/source results remain applicable under the local-validation policy, with final board/diff checks and Terra review required on that metadata revision.

Three earlier failures were resolved and rerun, not counted as passes: the standalone Swift command omitted `-Xcc -DSQLITE_HAS_CODEC`; the shared secure-delete pragma used a write-only callback despite returning a row (the stricter SQLite double reproduced failure before the scalar-query correction); and AGP replaced the test manifest's sole instrumentation entry with the default runner. The manifest now declares both runners, and the Android script verifies installed registration. Actual local AGP merging reproduced and verified that correction; its isolated diagnostic used the pinned upstream AAR manifest only, not a substitute for native build/alignment evidence. The final hosted Android job provides that native evidence.


## Review and merge

- Branch: `ticket/MC-018-encrypted-persistence-and-retention`.
- Review/PR: [PR #22](https://github.com/wickesjon/meshChat/pull/22). Terra medium reviewed `4f2f95569617a2b1c49b2165463d9e097cfffc99` with no findings. The worker posting attempt was rejected by automatic approval review; its completed review was accurately transcribed by the primary agent in COMMENT review #5209979292. Terra follow-ups found no issues at `cf6ef6b71b18731f5e18f2d4b4f2caeff57ba80e` (COMMENT #5210052176), `89a06b45172a33809a15e763ffec0c49564cdf56` (#5210202215) and `0db4b5ab912499282356cfd08b4bbc4f1a0e0d12` (#5210210338) and `8e970761b345e4372865323b0409eb11fd784873` (#5210435009). Each is the completed separate worker artifact, accurately transcribed and anchored to its actual revision. Final evidence/completion metadata review must be recorded on PR #22 before merge.
- Squash commit title: `MC-018: Encrypted persistence and retention`.
- Completion becomes effective only when the reviewed squash commit lands on main.
