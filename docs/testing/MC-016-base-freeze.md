# MC-016 base transport freeze

Status: accepted by Terra medium review5209133847 at `a2ff0714f3faddecc62dd30617a65e05c70a549f`; effective only when MC-016 is squash merged to main. The stable base contract is MC-006 plus MC-007's approved budgets and the explicitly isolated MC-008 structural extension allocations. This freezes base encoding/decoding and transport behavior for subsequent implementation. It does not freeze or certify cryptographic constructions, native radio behavior, persistent storage, UI admission, or the full wire/security contract.

## Frozen boundary and compatibility

The source evaluated is `30ba33debac4530db60d6614df49506ab1c18bc8`, the merged MC-015 revision. MC-016 changes documentation and evidence only. The normative [MC-006 wire contract](../decisions/MC-006-wire-contract.md), [MC-007 budgets](../decisions/MC-007-budgets-and-acceptance.md) and [MC-008 crypto contract](../decisions/MC-008-crypto-contract.md) remain authoritative; this record does not redefine their bytes or limits.

| Boundary | Stable behavior |
|---|---|
| Discovery/bootstrap | Advertise only the service UUID; use the specified INFO/characteristic UUIDs and exact HELLO envelope. Determine each native direction's capacity and exchange compatible HELLOs before ordinary traffic. Proof syntax is structural only here. |
| Frames | Exactly one outer frame per native value; no batching prefix. Directional capacity 146 through 512, maximum logical packet 1024 and SYNC transport 1035 bytes. HELLO/LINK_PROOF remain whole-only. Capacity changes invalidate the link context; re-admission uses a fresh generation. |
| Logical header and clear forms | Version 1 is a complete byte; exact 26-byte header/declared payload length, type/flag/channel matrix, clear CHAT/REACTION/control field offsets, cosmetics/pins, text limits and standalone credential syntax. Unknown high flag bits are ignored for layout but preserved; known semantic constraints still apply. |
| Compatibility rejection | Reject unknown versions and reserved types 00/04. Types 09–ff are bounded opaque flood-only envelopes with no display/history/trust meaning. Unknown flags cannot create a direct control or bypass limits. No fallback to earlier omitted-field layouts. |
| TTL | Preserve immutable bytes; only permitted live forwarding clamps/decrements TTL once. Live TTL 0 rejects; stored CHAT TTL 0 can be received locally through SYNC but cannot be forwarded. Reads, replay and pagination do not reset stored TTL or original arrival. |
| SYNC | Exact request/filter and response/sequence/marker layouts; fixed-Bloom best effort; newest-first snapshot references; one-use link/session/filter-bound cursors; four items/page, eight items/8192 encoded bytes/session, 120-second absolute deadline. Ordered terminal metadata is not authentication or application display. |
| Resource and lifecycle policy | MC-007 link/node frame, byte, work, sender, queue and memory limits; bounded expiry/dedup/pending states and disconnect cleanup; actual native results drive pacing/retries/failure. Public SYNC dispatch requires consuming an opaque outer-ingress admission token. |

Future incompatible wire changes require an explicit versioned decision and updated compatibility/negative vectors before implementation. Do not silently reinterpret version 1, reuse reserved types as a compatibility shortcut, alter authenticated immutable bytes, or accept an old grammar. A future extension within an existing opaque allocation still needs its owning decision and security/compatibility review. If a base defect or native capacity assumption fails later, reopen the affected decision and dependent gates before claiming the corrected freeze.

## Corpus identity and evidence states

The 170 logical and 15 outer framing vectors reproduce byte-for-byte with their independent Python generators. File SHA-256 (committed Git blob bytes, LF):

- `tests/vectors/base/logical.txt`: `3a50dea72f1bd9c8f671c0743efdf8a50bbef67a52dea6b079d682a073ea600f`.
- `tests/vectors/base/frames.txt`: `deed264ac5f08e77110023430c12eafa0f28f28b0ddd416b134f82bf0dfda343`.

The [vector guide](../../tests/vectors/base/README.md) explains fields, expected errors and independent assembly. Structural signatures, credentials, encrypted envelopes and LINK_PROOF vectors contain synthetic bytes; successful decoding is never successful authentication. Public link/channel/text behavior and unknown-type preservation are covered by the existing Rust regressions. Seeded codec/framing/link mutation campaigns each run 100,000 iterations, plus corpus truncations/negative cases; ingress lifecycle/intake runs 12,000 steps over 600 simulated seconds. These are deterministic mutation tests, not coverage-guided fuzzing or proof that no defects exist.

Evidence states: base contract **specified**, components **implemented**, host base checks **tested**; separate Terra PR review is required before this freeze is effective. Later independently required security assessments are not supplied by this record.

## Reproduced gates on the merged source

Host: Windows x86_64 MSVC, rustc 1.85.1 (`4eb161250`, LLVM 19.1.7), Cargo 1.85.1, Python 3.14.4, cargo-deny 0.20.2. All runtime evidence below was collected with clean tracked source at the revision above. Logs/JSONL traces reside in ignored `.work/mc016/` and can be regenerated using the commands below.

| Gate | Result and limit of the evidence |
|---|---|
| Core | 82 passing checks in each of debug/release: 80 runtime tests and 2 compile-fail admission-capability tests. Formatting, all-target/all-feature clippy, release build and dependency advisory/bans/licenses/sources pass. Both lockfiles unchanged. |
| Ingress | 14 component tests plus mutation smoke; included 600-second sender rotation at 100 values/second/link: one link 60,000 offered/614 admitted; eight links 480,000 offered/4,859 admitted. Component tests cover malformed/oversized/unknown/reserved input, staging/work/concurrency/session limits, pending overflow, invalid-first structural recovery, reconnect credit retention and separated accepted/rejected state. No actual crypto operation is claimed by these attacks. |
| Live relay | 18 scenarios × seeds 7/19/43 × two policies × exact rerun = 216 executions, 108 retained reports. All required lossless TTL-reachable pairs arrive; maximum p95 is 18,599ms versus 30,000ms. Dense K6 forwarded attempts 720 versus 900 baseline: 0.80 ratio. Stress loss/outage keeps the static denominator and reports missed pairs; it does not claim lossless recovery. Every report asserts resource limits, no beyond-TTL delivery and exact rerun metrics/trace. |
| Driver scenarios | Late join without historical live delivery, reconnect/mobility, version/identity/clock/power changes, asymmetric capacities,145-byte refusal, and backpressure/recovery all reproduce. These are simulated native outcomes, not a real-device connection guarantee. |
| SYNC | 18 paced direction/capacity/fixture cases repeated twice: 36 executions, identical metrics. Both directions deliver all eight selected packets and ordered terminal within 120s; maxima C146 71.02s, C182 52.02s, C512 25.02s. Actual production cache/session/ingress/relay/framing handles each value with 20ms native completion/transit, one frame/second and concurrent 421-byte ANNOUNCE / 36-byte REACTION. |
| SYNC adversarial/selection | 12 session and 4 cache tests, plus the original fixed JSON Bloom/mixed-channel cases, pass. Covered: empty/truncated walks, cache insertion/eviction/expiry, fixed arrival/TTL, stale tokens/cursors, one-use admissions, session and sender caps, cancellation/native failure, reordered/duplicate/conflicting/gapped responses, disconnect and 30s gap / 120s absolute deadlines. Unchanged-filter retries retain their declared false-positive omission; subscription membership does not drive selection. |
| Harness/spec consistency | Standalone simulator fmt/clippy/build and fixed JSON tests in debug/release; 15 Python simulator regressions; scenario definitions and sizing worksheet; ticketboard default plus 12 unit tests and whitespace checks pass. |

The relay driver uses production ingress/relay but intentionally reports no SYNC session outcomes; the separate Rust exchange supplies the actual cache/session result. `coverage-fixture` and `unsuppressed-fixture` are historical policy labels; with `--core-relay` they select the production relay's suppression setting. Lower latency or equal per-node mixed-tier work does not establish battery savings or load redistribution.

The SYNC gate starts with pre-admitted ready links, full buckets and a preloaded source cache. HELLO plus eight proof frames reserve nine setup frames / 1227 bytes per direction before the initial request; this is separate capacity accounting, not a valid proof exchange. Unsigned 338-byte, organizer-structure 556-byte and encrypted-structure 387-byte fixtures cover actual codec envelopes. Signed/encrypted data stays Pending. The codec's current maximum valid CHAT is 556 bytes, so eight valid items cannot fill 8192 bytes. Production byte-budget arithmetic tests 8192/8193 and overflow; the unchanged 1024-byte sizing worksheet completes at 102.02 seconds, explicitly arithmetic-only. No invalid 1024-byte envelope is declared a valid packet to satisfy that capacity witness.

## Transport assumptions and physical scope

The approved [MC-004 online evidence and permission decision](../decisions/MC-004-online-feasibility.md) exists and records source methods, limitations, conditional Android/iOS roles, RSSI use and recovery behavior. That acceptance was user-approved in place of the early physical gate. Published phone-to-board throughput is not meshChat throughput and does not establish a guaranteed minimum capacity.

The current host harness starts after link admission; it does not execute a production native HELLO/duplicate-link arbitration exchange. Those driver/lifecycle integrations remain MC-023/024/026 obligations. The frozen bootstrap syntax and admission requirements below are their contract, not a claim that those drivers already implement it.

Production native implementations must query successful/runtime directional limits, stop on backpressure and resume on native readiness, reject values exceeding the specific direction's limit, refuse below 146, and discard link work on disconnect/permission loss. The separate 512-byte protocol ceiling is not an MTU guarantee. iOS background discovery constraints, Android permission/service restrictions, process death and user force-stop/quit remain conditional or unsupported as recorded in MC-004. No device pairing, background lifetime, radio range, latency, battery or hardware key protection is certified here. MC-025/027 and MC-043/044 retain their transferred physical scenarios; beta/release descendants retain those dependencies.

## Explicitly unfinished cryptographic/full-wire work

MC-008 specifies the profile/transcripts; structural allocation does not freeze their security result. MC-019/020/021 must implement real key binding/signature/AEAD and organizer validation, including actual ingress invalid-first/valid-second, authenticated replay, eviction/concurrency/budget recovery and failure cases. MC-022 requires these results and the separately required independent construction/transcript assessment before full-wire freeze. Proof authenticity, replay/session proof binding, credential trust/pins, encrypted failure recovery and actual crypto-unit charging remain security-owner gates. MC-018 owns encrypted persistence/provenance/lifecycle; this base freeze creates no sensitive-data use authorization.

The approved sequencing removes a dependency cycle; it does not replace missing verification with a mock verifier, structural signature or authenticated status. Later security tickets may integrate through the base ingress admission/work APIs, but cannot bypass native frame limits or silently change this base grammar. Native platform builds still apply when shared exports, platform code/builds or dependencies change.

## Reproduction

Use repository-local toolchain/cache/temp paths as in the CI workflow and validation policy. On Windows set `RUSTC` and `RUSTDOC` to the pinned executables' absolute paths for fresh standalone builds; use a distinct `CARGO_TARGET_DIR` for the standalone simulator to avoid aliasing its feature set with the root library artifacts. No dependency download/update is needed after the pinned caches are populated.

```text
python -B tests/vectors/base/generate.py
python -B tests/vectors/base/generate_frames.py
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo test --workspace --all-features --locked --offline
cargo test --workspace --all-features --locked --offline --release
cargo build --workspace --all-features --locked --offline --release
cargo-deny --all-features --locked --config src/core/deny.toml check
cargo fmt --manifest-path tests/simulator/Cargo.toml -- --check
cargo clippy --manifest-path tests/simulator/Cargo.toml --all-targets --locked --offline -- -D warnings
cargo test --manifest-path tests/simulator/Cargo.toml --locked --offline
cargo test --manifest-path tests/simulator/Cargo.toml --locked --offline --release
cargo build --manifest-path tests/simulator/Cargo.toml --locked --offline --release
python -B tests/simulator/scenarios/validate_definitions.py
python -B tests/simulator/scenarios/budget_worksheet.py
python -B -m unittest discover -s tests/integration/simulator
python -B -m unittest discover -s tests/ticketboard
python -B tests/ticketboard/validate.py
git diff --check
git diff --exit-code -- Cargo.lock tests/simulator/Cargo.lock
```

Run the relay driver using the executable from that standalone target directory:

```text
python -B tests/simulator/runner.py --core PATH_TO_SIMULATOR_EXE --output .work/mc016/reproduce --core-relay --extra tests/simulator/scenarios/MC-012-driver-cases.json
cargo test -p meshchat-core --all-features --test sync_exchange --locked --offline --release -- --nocapture
```

The relay command includes all 18 cases and reruns every seed/policy automatically, requiring exact equality before writing 108 reports. The recorded execution used 72 main-suite reports plus six individually selected driver cases producing 36 reports; their union is the same 18-scenario set. Repeat the SYNC command and compare all 18 `SYNC` output rows. Record source revision, clean/dirty state and tool versions with the regenerated artifacts. Source changes invalidate previous-revision runtime claims; documentation-only changes may cite the unchanged measured source explicitly under the local validation policy.
