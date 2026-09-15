# MC-012 deterministic GATT simulator

Evidence: implemented/tested simulator and core primitives. Production ingress,
relay-policy and SYNC acceptance remain **not run** here, owned by MC-013/014/015/016.
Physical delivery, radio overhead, battery, native connection capacity and security
assessments remain separate. Every machine-readable report carries that distinction.

## Architecture and scope

`tests/simulator` is an independent, locked Rust test workspace plus a standard-library
Python event driver. This keeps the harness inside MC-012's permitted paths without
adding test tooling to the production dependency graph. Its Rust dependencies are
the actual `src/core` crate and the same pinned serde_json version already present
in the root lockfile; it does not reimplement the wire encoder or reassembler.

One persistent JSON-line subprocess owns a real `Core` and `Reassembler` per node.
Connect/disconnect/time/power events and every native send/receive pass through
those APIs. Frames come from `framing::Encoder`; complete logical objects are parsed
by `codec::parse`, and relay TTL bytes come from `Packet::forward_to`. A structural
SYNC marker can traverse the adapter but never completes a simulated SYNC session.
The adapter is an internal synthetic test interface, not an untrusted network service.

The existing core does not yet emit production relay/scheduler/dedup/SYNC effects.
`Simulation` therefore contains an explicitly named **fixture policy** that supplies
offered traffic and invokes the implemented primitives. MC-007's manifest already
assigns runtime production gates to MC-013/015/016, and MC-014 owns live relay results.
Those tickets must replace the corresponding fixture decisions with production
effects and run their gates; passing this harness cannot complete those tickets.

Both `coverage-fixture` and `unsuppressed-fixture` use the same event engine, traffic,
directed links, queue limits, atomic integer link/node budgets, native readiness and
loss hash. Only the latter disables holdoff and peer-coverage cancellation. Coverage
evidence comes solely from a complete packet received from that peer; the simulator
does not inspect another node's cache to decide suppression. Baseline forwarding
includes the arrival peer, since that cancellation is coverage suppression too.
Neither mode changes production source or claims authentication or UI acceptance.

## Reproduce

Use Rust 1.85.1, Python 3 and the repository's workspace-local tool/cache environment.
From the repository root (PowerShell uses `python`; Unix may use `python3`):

```text
cargo fmt --manifest-path tests/simulator/Cargo.toml -- --check
cargo clippy --manifest-path tests/simulator/Cargo.toml --locked --all-targets -- -D warnings
cargo build --manifest-path tests/simulator/Cargo.toml --locked
python -B tests/integration/simulator/test_simulator.py
python -B tests/simulator/scenarios/validate_definitions.py
python -B tests/simulator/scenarios/budget_worksheet.py
python -B tests/simulator/runner.py --core target/debug/meshchat-simulator-core.exe --output .work/mc012/results --extra tests/simulator/scenarios/MC-012-driver-cases.json
```

On Unix use the executable without `.exe` and set `MESHCHAT_SIM_CORE` to its absolute
path for the integration tests. For release verification build with `--release` and
point both commands at `target/release/meshchat-simulator-core[.exe]`. Set
`CARGO_TARGET_DIR` explicitly to the repository's `target` directory on all hosts.
The auxiliary workspace uses its own Cargo.lock and must always use `--locked`.
Run cargo-deny with `--manifest-path tests/simulator/Cargo.toml --locked --config
src/core/deny.toml check` using the existing pinned cargo-deny tool.

The runner writes `metrics.json` and one canonical JSONL trace per scenario/seed/policy
under `.work` only. It immediately repeats each run and requires complete metric and
trace-digest equality. `--scenario chain10 --seed 7` selects a small development run.
No simulator hardware prerequisite exists. Local simulator checks are explicit here;
the existing hosted workflow does not yet discover this independent workspace.
Wiring a hosted job is outside this ticket's permitted `.github` paths; these local
commands satisfy the approved local validation policy for this test-only change.

## Event and measurement contract

- Events order by monotonic milliseconds, completion/arrival (0), timer (1),
  scheduling (2), node ID, then that node's insertion sequence. Link initialization
  precedes traffic at time zero. HELLO admission is an explicit initial assumption;
  no setup frames or valid proofs are invented.
- SHA-256 tuple encoding is exactly MC-007's big-endian seed/domain/source/destination/
  object/fragment/attempt format. Loss uses integer parts-per-million thresholds;
  the committed 20% case is exact. No wall clock, Python hash order or platform PRNG
  affects decisions. Synthetic sender IDs use a separate traffic-tuple object range.
- Each individual core send command is one GATT attempt; fragments include all actual
  encoded header bytes. Categories and directed-egress counters sum to the totals.
  Fragmented/whole frame counts, loss, native completions and received objects are
  separate. Native completion is not peer receipt. Loss here is unacknowledged;
  retries are explicitly zero, not free retransmissions.
- Every scheduled CHAT and static graph recipient within TTL 7 stays in the denominator,
  including codec/queue/link refusals. Beyond-TTL recipients are separate. Outage cohorts
  count origins scheduled during the outage whose static reachable recipient loses its
  TTL-bounded path when the named outage edges are removed. Latency uses first synthetic
  delivery minus scheduled time; p95 is reported with the delivery ratio, never alone.
- A complete object counts as synthetic delivery only if it matches a predeclared
  unsigned CHAT's exact immutable bytes. Structural parse success, announcements and
  transport markers are not delivery/authentication/session completion evidence.
- Fixture pacing permits one frame/second/direction, 20 ms completion/transit, contiguous
  fragments, bounded deficit scheduling, aggregate MC-007 frame/byte buckets and Normal/
  Saver forwarding credit. Announcements are explicitly unsigned 316-byte fixtures;
  the signed 421-byte **SYNC** workload remains with its owning acceptance suite.
- Queues cap at 32 objects/link and 128/node, retaining at most 1024 encoded bytes/object;
  own overflow is recorded explicitly, never silently evicted. Unstarted and started
  objects have separate absolute 30-second deadlines. The fixture driver rejects new
  overflow rather than implementing MC-014's full production eviction policy. Native
  readiness recovery does not create a catch-up burst. Reconnect drops queued/fragment
  state, assigns fresh core generations and retains node bucket credit.
- Reported queue peaks are **harness object counts**, not Python allocator measurements
  or proof of production MC-007 allocated-byte caps. Real reassembly reservations and
  peaks come directly from the core. Fixture dedup is limited to 512 scheduled origins
  per node and peer evidence to 4096 entries; no trusted dedup, crypto, sender admission,
  credential/orphan/cache/SYNC acceptance is asserted. Resource gates must use production
  counters once the owning core state exists.

## Committed cases and observed evidence

The unchanged MC-007 manifest supplies chain10, cycle8, barbell8, bridge9, dense6,
the Saver bridge repeat and five 20%-loss stress cases. The MC-012 driver manifest
adds late join, mobility/reconnect, mixed version/identity/clock/power changes,
asymmetric 146/512 capacities, C145 refusal and backpressure recovery. Version injection
currently tests real codec refusal at origin encoding; it is not HELLO interoperability.
Wall-clock offsets affect synthetic payload timestamps only, never the monotonic clock.
Identity rotation changes future synthetic sender bytes without claiming key continuity.

On Windows x86_64 with Rust 1.85.1 and Python 3.14.4, the initial full suite executed
17 cases × three seeds × two policies, with each repeated: **204 deterministic runs**.
All repeats had identical full metrics and trace hashes. Lossless coverage-fixture
deliveries were chain 99/99, cycle 84/84, barbell 84/84, bridge and Saver bridge 96/96,
and dense 60/60, with no beyond-TTL deliveries. Seed-7 p95 ranged from 5.02 seconds
(dense) to 16.487 seconds (chain). These are fixture-policy results, not MC-014 acceptance.
Late join was 3/4, backpressure recovery 2/3 and capacity refusal 0/1, preserving misses.

The suite's nominal graph queue peaks were at most ten objects/node and two/link;
the dedicated overflow test reaches the 32/link ceiling and observes refusals/deadlines.
Core reassembly reports 126,072 reserved bytes/node on this host. None of these numbers
are a total production memory or battery claim. Machine reports include source revision,
dirty state, source digest, executable digest, scenario/manifest hashes, duration,
denominators, per-egress counters and the trace digest for review and reproduction.

The integration suite checks exact frame lengths and sums, directional costs, atomic
budget admission, deterministic hashing/replay, packet loss, stale generations, late
join, queue pressure, clocks/versions/power, chain TTL, dense/reference pairing,
malformed wire rejection, structural-only transport markers and outage denominators.
MC-007 definition/worksheet checks retain their original specified/arithmetic evidence.
