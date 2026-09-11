# Mesh acceptance scenarios

[MC-007](../../../docs/decisions/MC-007-budgets-and-acceptance.md) defines normative budgets, state limits and measurement rules. [MC-007-acceptance.json](MC-007-acceptance.json) is the predeclared scenario manifest. It specifies graph edges, seeds, traffic, loss/churn, denominators and pass thresholds. The graph is a collection of bidirectional point-to-point GATT links, with separate directional frame accounting.

Run these definition and arithmetic checks from the repository root:

```text
python -B tests/simulator/scenarios/validate_definitions.py
python -B tests/simulator/scenarios/budget_worksheet.py
```

The first checks scenario consistency, graph/link ceilings and TTL-reachable denominators. The second produces a feasible paced deficit-round-robin trace for two simultaneous SYNC walks at capacity 146, checking link/node frame, byte, forwarding and reserved crypto allowances. It accounts for setup, reverse requests, page/terminal markers, signed ANNOUNCE and reactions. The result is 104 frames and 13,060 value bytes per direction, with ordered completion at 102.02 seconds. Setup reserves another nine frames/1,227 bytes per direction. This calculation does not run production code or establish radio throughput.

The 1,024-byte logical item is a sizing placeholder. MC-009/010 supply codec and reassembly fixtures; MC-008/020–022 supply valid encrypted/signed/proof fixtures and exact work counts. The reserved proof and crypto allowance is capacity headroom, not a fabricated valid proof or passed cryptographic test. Maximum signed organizer CHAT is currently 556 bytes; the capacity witness deliberately reserves the full logical ceiling.

The same paced witness checks eight maximum unsigned (338-byte), friend-signed (443-byte) and organizer-signed (556-byte) records. Their per-direction totals are respectively 64/72/80 frames and 6,932/7,900/8,932 value bytes, including the full window's background traffic; ordered completion is 48.02/57.02/71.02 seconds. These are size/work reservations, not signature verification results.

MC-012 builds the actual event runner against the shared core, using the manifest's event order and hash encoding. Live topology runs use Normal Android core limits unless the manifest requests a Saver repeat; they do not certify a native connection count. Compare the same traffic, seed, directed links and loss decisions against the unsuppressed baseline. Preserve origin/relay/SYNC/control frame and byte counters independently. Report both delivery and GATT cost; zero suppression benefit is a valid measured outcome, not a battery-saving claim.

The live chain's scheduled workload contains 99 TTL-reachable recipient pairs and nine beyond TTL. The cycle and direct barbell each contain 84 reachable pairs, the bridge topology 96 and dense graph 60. Refused scheduled origins remain in the denominator. Stress cases retain those denominators and report outage cohorts separately, with no lossless delivery guarantee.

Adversarial variants are separate runs with the common declared duration, seeds and offered rate. Use maximum-sized valid unsigned CHAT with a fresh seeded sender/msg_id for sender rotation; complete known fragment envelopes with invalid count followed by repeats for rejected-group tracking; valid clear REACTION with an absent seeded target for orphan pressure; and native egress readiness held false for queue pressure. Malformed cases mutate every fixed-envelope length/boundary and reserved field of the codec fixtures. Key/credential and cryptographic invalid-first/valid-later variants require the owning security fixtures before they can pass. The valid-later check must include a budget-available recovery phase; do not call an ingress-budget drop a dedup failure. MC-012/013 document precise generated fixture IDs in their reports.

No simulator is implemented by this decision ticket. Physical radio/battery tests, protected-storage verification and independent security assessments remain separate gates. Reports must include the source revision, scenario ID/seed, fixture revision, duration, resource peaks, all denominators and observed outcomes; a manifest or worksheet result cannot stand in for them.
