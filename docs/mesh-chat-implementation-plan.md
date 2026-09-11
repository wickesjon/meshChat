# Meshfest v1 — Implementation Plan

Companion to the design doc (`mesh-chat-design.md`). Section references like §3.4 point there. This document is the *build sequence*: what to build, in what order, how to know each piece works, and where the risk lives.

---

## 0. Scope & shape of v1

### 0.1 What ships in v1

Offline BLE-mesh group chat with: public channels (#General, #Event Updates, #Confessions) and semi-private word-triple channels; custom-mesh relaying with battery-minimizing suppression; per-sender rate limiting; nicknames + animal/color avatars (§10.6); verified friends via in-person QR (§7.4); end-to-end encrypted 1:1 DMs (§7.5); message reactions (§2.7); organizer-verified #Event Updates (§17); shareable channel links + QR (§5); the Afterhours dark theme (§10.0); Auto power management (§11.4); Android Beacon Mode (§15.7); and the Supporter tier (§19).

### 0.2 What is explicitly NOT in v1 (see §20)

Encrypted group chats; DM forward secrecy / Double Ratchet; global message signing for all public traffic; pre-distributed event keys; Wi-Fi Aware transport; iOS Beacon Mode. Designed-for, deferred.

### 0.3 Platform sequencing

**Android first, iOS second.** Android's foreground-service model lets the mesh work fully, making it the platform to prove the protocol on; iOS is a port against a frozen protocol core plus a from-scratch CoreBluetooth driver with background constraints (§8.4). Ship a public Android beta before iOS feature-complete.

### 0.4 Guiding build principles

- **Protocol core is radio-agnostic and ships first.** Everything testable without Bluetooth (codec, mesh logic, rate limiting, crypto, storage) lives in the Rust core (§12) and is proven in the simulator before any radio work.
- **Simulator before silicon.** Mesh behavior is validated on 200 virtual nodes (§13) before it ever touches a phone; radios only add real-world noise to already-correct logic.
- **Security gates are non-negotiable.** The fuzzing, crypto, and hardening CI gates (§12.1, §13) block merges from the moment the relevant code exists — not bolted on at the end.
- **Honest-by-construction UI.** Every "this isn't private / not verified / rate-limited" affordance (§7, §18) is built with the feature it qualifies, never deferred to a polish pass.

---

## 1. Team & tooling assumptions

Plan is written for a **small team (3–5 engineers)**: 1–2 on the Rust core + crypto, 1–2 on Android, and **iOS capacity (with Xcode tooling) present from M0 (not M4)** to run the two-platform BLE spike — not deferred to M4, since the spike's whole purpose is to test the iOS half of the transport before M1 freezes. Design/UX shared. A solo developer can follow the same milestone order at roughly 2.5–3× the calendar. Estimates below are engineer-weeks of focused work, not wall-clock, and deliberately ranged.

Baseline tooling stood up in M0: monorepo (§12 layout), Rust stable + `cargo-fuzz`/`cargo-audit`/`cargo-deny`, UniFFI for bindings, Android Studio + Kotlin, Xcode + Swift (from M4), CI (GitHub Actions or similar) running the full gate suite on every PR.

---

## 2. Milestones overview

| # | Milestone | Focus | Est. (eng-weeks) | Exit gate |
|---|---|---|---|---|
| M0 | Foundations + feasibility spike | Repo, CI, skeleton core + bindings, **two-platform BLE spike, permission ruling** | 1–2 | CI green + **recorded spike results + permission decision** (gates M1 freeze) |
| M1 | Protocol core | Codec, mesh, rate limit, storage, sim | 4–6 | **Base transport frozen**: sim targets + base-transport golden vectors |
| M2 | Crypto core | Identity, friends, authenticated DMs, org creds | 3–4 | **Full v1 wire frozen**: crypto suite green + crypto golden vectors |
| M3 | Android BLE | Real radios, 2-device → N-device mesh | 4–6 | 10-device bench mesh relays reliably |
| M4 | Android app | Full UI, all v1 features on Android | 6–8 | Feature-complete Android beta |
| M5 | iOS port | CoreBluetooth driver + SwiftUI app | 6–9 | iOS↔Android interop matrix passes |
| M6 | Beacons & Beacon Mode | Fixed relays + in-app beacon mode | 2–3 | Beacon extends coverage in field test |
| M7 | Hardening & field | Security audit closure, scale test, launch prep | 4–6 | External pen-test clean; 30+ device field test |

Total rough order: **30–44 engineer-weeks** to v1 launch — a *planning estimate, not a commitment* (the adversarial review rightly flags it as unvalidated). It excludes the two hardest-to-bound unknowns: iOS background-BLE reliability work (M5) and field-test iteration (M7), either of which can expand materially. Treat the number as a lower-bound sizing input; re-baseline after the M0 BLE spike gives real data. M1–M2 and parts of M3–M4 parallelize across the core/Android split.

---

## 3. M0 — Foundations + feasibility spike (1–2 eng-weeks)

Stand up the skeleton everything else hangs on — **and de-risk the two things an adversarial review flagged as unprovable on paper** (design review findings 4, 7). M1's protocol must not be frozen until these clear.

**Feasibility spike (must complete before M1 protocol freeze):**
- **Two-platform BLE spike:** a throwaway Android + iOS build that establishes a GATT link both directions and measures the *actual* achievable behaviors — background discovery reality (confirm Android cannot discover a backgrounded iOS peripheral; §8.4), write-without-response throughput and backpressure, negotiated MTU floor, and held-connection survival across backgrounding. The protocol's transport assumptions are ratified against silicon here, not assumed.
- **Android permission/privacy ruling:** decide the manifest before it locks (finding 7) — whether RSSI friend-proximity and beacon telemetry ship, and therefore whether `neverForLocation` is claimed or the location permission is requested and disclosed. Get a platform-policy read; the choice affects discovery reliability and store review.

**Build:**
- Monorepo with the §12 structure: `core/` (Rust), `android/`, `ios/` (stub), `tools/simulator/`, `tools/flooder/`.
- Rust core crate with `#![forbid(unsafe_code)]`, `overflow-checks = true` in release, the §12.1 lint/gate config.
- UniFFI wired end to end: a trivial `core_version() -> String` callable from a throwaway Android activity, proving the FFI boundary and `catch_unwind` wrapper work.
- CI pipeline running: `cargo test`, `cargo clippy -D warnings`, `cargo audit`, `cargo deny`, `cargo fmt --check`, and a placeholder fuzz job. Android lint + build.

**Exit gate (revised — finding R7, now actually gates the feasibility work):**
1. Green CI on the structured repo; an Android debug build calls into the Rust core.
2. **Recorded BLE spike results** (a committed short report): measured MTU floor, write-without-response throughput and the observed backpressure behavior on both platforms (§8.2 state machine validated), confirmation of the Android↔backgrounded-iOS discovery limit (§8.4), and held-connection survival across backgrounding.
3. **Recorded permission/privacy decision** (finding 7): the manifest choice (RSSI-proximity in or out; `neverForLocation` vs. location permission) written down with its rationale and any platform-policy read.

**M1 protocol MUST NOT be frozen until items 2 and 3 are recorded.** These are what M0 exists to retire; a green-CI-only gate would let M1 freeze on the very assumptions the spike was meant to test.

**Risk:** UniFFI binding friction across Rust↔Kotlin types. Mitigation: keep the FFI surface tiny and event-based (§12 sans-IO design) — resolve binding quirks now on a trivial call, not later on a complex one.

---

## 4. M1 — Protocol core (4–6 eng-weeks)

The heart of the system, entirely radio-free and simulator-validated. Nothing here needs a phone.

**Build, in order:**
1. **Packet codec (§2).** Header + CHAT/ANNOUNCE/SYNC/REACTION/EVENT_INFO payloads; **fragmentation as a transport envelope reassembled before the mesh pipeline (§2.4, finding 1)** with hard reassembly bounds (§18-M1); **version-tolerant relay (§2.8, mixed-version review item)**. All length math `checked_*`; parse returns `Result`, never panics. *Immediately* wire `cargo-fuzz` at the parser.
2. **Channel derivation (§4.2).** Normalization + hashing; the 8,000-combo word lists and KAT vectors; public-channel IDs; glyph derivation (§10.6).
3. **Seen-cache + dedup (§3.2)** operating on **reassembled logical packets** (finding 1), per-source partitioning (§18-M7).
4. **Relay engine (§3.1–3.4):** TTL clamp; **per-egress-peer suppression driven by explicit per-link knowledge, with bridge/sole-path protection (§3.4, finding 2)** — not global cancel-on-two-duplicates; hold-off jitter; TX batching.
5. **Rate limiter (§6):** per-sender × channel-class buckets, global bucket, **and the per-link aggregate ingress bucket that bounds `sender_id`-rotation Sybil traffic (§6.5, finding 5)**; receiver-side drop-before-relay.
6. **SYNC (§2.6, R3/R2.2):** cursor-based multi-packet framing (batch_id/seq/last); SYNC_REQ is a ~550-byte **logical packet** (Bloom k=6, m=4096 bits) fragmented per the frame/logical split (§2.1) — the 512-byte figure is a *frame* limit, not a logical-packet limit; amplification quotas (§18-M2).
7. **Storage (§9):** `rusqlite`, four tables (messages/subscriptions/settings/friends); retention pruning. (SQLCipher key wiring in M2.)
8. **Sans-IO API surface (§12):** events in, commands out, injectable clock.

**Build the simulator (§13) in parallel** — acceptance harness for all the above: N virtual nodes, configurable topology/loss/mobility/**churn/mixed-version/identity-rotation**, no radios. **Must include the two-cluster-single-bridge cut-vertex scenario (finding 2)** and an **identity-rotating flooder (finding 5)**.

**Exit gate — "base transport frozen" (revised per review; finding R6 splits the freeze into two):**
- Density: at 100+ nodes, mean relays/node/message ≤ 0.3 with delivery ≥ 95%.
- Sparse: 10-node chains deliver ≥ 99%.
- **Bridge: two-cluster-one-bridge cross-cluster delivery ≥ 95% — suppression must not sever the bridge.**
- Battery-tier: tier-3 nodes carry ≥ 3× tier-0 relay load.
- **Flood: a single physical flooder rotating `sender_id` every packet is bounded by the per-link ingress budget; measured, not "zero beyond one hop."**
- **SYNC (corrected, finding R5.2): a late joiner recovers ≥ 95% of the *newest 100* messages relevant to its subscriptions within one session (≤30s), using cursor pagination; Bloom FPR ≤ ~2% at 500-item load; SYNC_ITEM transport-object fragmentation never exceeds the live link write length. (The old '95% of a 500-message cache with a 25-item quota' target was arithmetically impossible.)**
- **Golden vectors for the base transport:** outer frame header in all four kinds (`0x00`–`0x03`), CHAT, **ANNOUNCE (full §2.5 layout)**, SYNC_REQ (with cursor), **SYNC_ITEM transport object (whole + fragmented)**, REACTION, EVENT_INFO, an unknown-type frame — byte-exact, committed, generated at the 182-byte minimum link length *and* a larger MTU.
- Parser + deep-link fuzzing clean (zero panics/OOM).

This gate freezes the **base transport + mesh** (frame grammar, fragmentation, dedup/TTL, relay, rate limits, SYNC container). It does **not** freeze the crypto envelopes — those aren't implemented until M2 (finding R6: a wire freeze cannot precede the format's implementation).

**The full v1 wire protocol is frozen only at the end of M2**, gated on the M2 crypto suite passing *and* **cross-language golden vectors for the DM authenticated-encryption envelope, the friend signature block, and the organizer credential chain** (§13). Second implementations (iOS Swift-via-UniFFI is the same core, but any independent parser — e.g. ESP32 — ) validate against these vectors before shipping. Until then, only the base transport is stable; crypto field layouts may still change.

Passing the base-transport gate means the mesh logic is correct against the *actual* GATT transport model before radios are added; the full-wire freeze additionally means the security envelopes are implemented and byte-agreed.

**Risk:** suppression tuning (§3.4) may not hit both the density and sparse targets with one parameter set. Mitigation: the simulator exists precisely to sweep parameters cheaply; budget time for a tuning pass, and keep the thresholds as config, not constants.

---

## 5. M2 — Crypto core (3–4 eng-weeks)

All cryptography, still radio-free, behind a hard CI gate. Uses vetted libraries only (`ed25519-dalek`, `x25519-dalek`, an AEAD, `hkdf`) — never hand-rolled (§12.1).

**Build:**
1. **Device identity (§7.1/§7.5):** **both** an Ed25519 signing keypair *and* an X25519 DM keypair generated at first launch, private keys to the OS keystore (Android Keystore now; iOS Keychain in M5); `sender_id = SHA-256(ed25519_pub)[:8]`; friend-code QR carries the 65-byte two-key bundle.
2. **Encryption-at-rest (§9, §18-B3):** SQLCipher keyed from the keystore; backup-exclusion flags. Retrofit the M1 storage layer.
3. **Message signing (§7.4/§17.1):** the `signed` flag + signature block; signed ANNOUNCE; signed private-channel messages where a pinned friend exists.
4. **Verified friends (§7.4):** friend-code encoding/decoding (`meshfest://friend/...`), pin storage, key-change detection + the mandatory re-scan-not-auto-trust rule.
5. **Authenticated DMs (§7.5, R4/R5.4):** two-DH (`dh_ephemeral ‖ dh_static`) → HKDF-SHA256 (salt/info per §7.5) → AES-256-GCM with the **canonical AAD binding header, msg_id, recipient tag, ephemeral key, and nonce** (authenticated-replay closed); ephemeral-key packet format; rotating recipient tags; length-bucket padding; encrypt→relay→decrypt path asserting relays cannot decrypt; **replay-with-modified-header negative vectors**.
6. **Organizer keys (§17, R3/finding 6):** event root + root-signed **staff credential wire object** (§17.1 binary format, carried in QR and inline in first signed message); root→credential→message verification chain; signed `not_before`/`not_after` (local expiry from the credential); canonical staff-sig transcript; adopt-root-via-QR bootstrap; EVENT_INFO discovery; raised relay budget for verified bursts across non-key nodes.

**Exit gate — the crypto CI suite (must match design §13 item 7 exactly; finding R5.4):**
- KATs for Ed25519 and X25519 against reference vectors.
- **DM sender-forgery negative vector (the test that proves the round-four vulnerability is closed):** an attacker holding every public value — both static public keys, a current recipient tag, valid ephemeral keys — but **not** the sender's static X25519 private key must be unable to produce a message the recipient authenticates as that sender.
- **X25519 validation:** all-zero shared-secret rejection (`pair_secret`, `dh_ephemeral`, `dh_static`), non-32-byte key rejection.
- **Recipient-tag vectors** across epoch boundaries (`epoch−1/epoch/epoch+1`) and with reversed peer ordering (must agree, since `pair_secret` is symmetric).
- DM round-trip through the simulator asserting in-path relays cannot decrypt; AEAD tamper tests (flipped byte ⇒ auth failure, never silent plaintext); replay-with-modified-header negative vectors.
- **Organizer chain vectors:** root→credential→message; multi-staff cache keyed by `(event_root_id, staff_key_id)`; the two binding checks; credential-omitted message + **multi-hop CRED_OFFER/CRED_REQ recovery where the immediate peer lacks the credential**.
- **Signed ANNOUNCE vectors** (§2.5 layout + friend transcript).
- Key lifecycle: both Ed25519 **and X25519** keypairs provisioned at first launch into the keystore; no message key ever written to disk.

All green before any crypto-bearing build ships. **This gate is also the full v1 wire freeze** (see M1).

**Risk:** deanonymization or key-continuity bugs are high-severity and easy to introduce. Mitigation: the crypto suite is written *alongside* each primitive, not after; the recipient-tag epoch boundary and the anonymous-post single-use-key path get dedicated tests because they're the subtle ones (§18-B1).

---

## 6. Dependencies between milestones

M0 → M1 → (M2 ∥ start of M3). M2 and M3 can run concurrently once the M1 **base transport** is frozen: M3 radios feed the frozen transport, M2 adds crypto envelopes on top (the full-wire freeze lands at end of M2). M3 only depends on the base transport, not the crypto layout. M4 needs M1+M2+M3. M5 needs a frozen protocol (M1–M2) and the M3 driver as a reference implementation. M6 needs M3 (Android) for beacon-mode; hardware beacons need only the frozen protocol. M7 spans everything and gates launch.

The critical path is **M0→M1→M3→M4→M7** for an Android-only beta, with M2 folded into M4's feature set and M5/M6 as parallel/follow-on tracks. iOS (M5) is the longest single risk and should start its BLE spike early (during M3) even if the app work waits.

---

## 7. M3 — Android BLE driver (4–6 eng-weeks)

First contact with real radios. The frozen M1 core is the brain; this milestone is the nervous system. Feed the core real bytes; add only real-world noise to already-correct logic.

**Build, staged:**
1. **Two-device direct link.** Foreground service (§8.3); GATT server (peripheral) + client (central) with the §8.2 service/characteristics; advertise + scan by service UUID; connect, negotiate MTU (request 517), exchange one CHAT packet. This is the "hello over Bluetooth" proof.
2. **Connection manager.** Maintain 3–6 concurrent links; the §8.1 peer-selection (bridge preference, RSSI diversity), role tiebreak, and the slot-exhaustion defenses (§18-M3: reserved slots, idle-peer eviction, inbound-rate cap). Feed connect/disconnect/bytes events into the core.
3. **Mesh over radios.** Let the core's relay engine drive real transmissions across 3, then 5, then 10 devices. Verify TTL hops, dedup, suppression, and SYNC catch-up behave on silicon as they did in the simulator.
4. **Power integration.** Wire Auto mode (§11.4), the relay budget ceiling, duty-cycled scanning, and the lone-wanderer backoff (§11.3) to the real radio and battery APIs. First real `batterystats` drain profiling.
5. **Permissions & OEM survival.** `BLUETOOTH_SCAN/ADVERTISE/CONNECT` (API 31+) per the **M0 permission ruling** (§8.3) — `neverForLocation` only if that ruling chose it, else the disclosed location permission; the battery-optimization-exemption onboarding; test against aggressive OEM killers (Samsung, Xiaomi).

**Exit gate:** a 10-device mixed-OEM bench mesh relays messages reliably across multiple hops, SYNC delivers recent history to a late joiner, measured drain lands in the §11.2 range (~2–5%/hr normal), and behavior matches simulator predictions. Embed hop-telemetry (`7 - ttl_at_receipt`) in debug builds for M7 field validation.

**Risk (high):** per-OEM BLE quirks and background/foreground reliability. Mitigation: buy the test-device matrix early (Pixel, Samsung, Xiaomi at minimum); treat OEM battery-killer survival as a first-class acceptance item, not an afterthought.

---

## 8. M4 — Android app (6–8 eng-weeks)

Everything the core and driver enable, wrapped in the full §10 UI in the Afterhours theme. This is the largest UI milestone and folds in the M2 crypto features on the surface.

**Build (screen inventory §10.1):**
- Onboarding (nickname + avatar/color §10.6, permissions walkthrough, mesh-expectations + airplane-mode education §16.1).
- Bottom tab bar: **Channels / Messages / Friends** + Settings gear (§10.1).
- Channels tab + chat screen: message list, animal avatars + channel glyphs (§10.6), reactions (§2.7), byte-aware composer with rate-limit countdown, mesh status strip (§10.4), semi-private open-lock affordance.
- Join private channel (three word pickers + randomize dice, §4.4), channel info / share page + QR (§5, §10.1).
- Messages tab + DM threads: encrypted 1:1 with closed-lock, key-change block state (§7.5).
- Friends tab: my-friend-code QR + scanner, add-friend confirmation sheet with fingerprint + in-person warning, pinned list, Friends-nearby strip (§7.4).
- #Event Updates verified rendering: staff badge as separate chrome, unverified drawer, pinned messages (§17.3); the "claims to be X · not verified" impersonation chip.
- Settings: theme picker, power mode, manage friends, Supporter tier + IAP entitlement (§19), reset identity, delete data.
- The message-rendering security rules (§10.2): native text only, no WebView, Unicode sanitizer, no auto-linkify.

**Exit gate:** feature-complete Android beta — every v1 feature usable end to end on a real device, all honesty/trust affordances present, tapjacking guards on confirmation sheets (§18-m4), no content in logs (§18-m7).

**Risk:** UI scope creep and the trust-chrome-vs-decoration separation (§18-B2) getting blurred under deadline. Mitigation: the separation is an assertion-tested invariant (§13 item 6) — a nickname/avatar can never render as a verified badge — so regressions fail CI, not just review.

---

## 9. M5 — iOS port (6–9 eng-weeks)

Port against the *frozen* protocol core (M1–M2, same Rust via UniFFI-generated Swift), with a from-scratch CoreBluetooth driver and SwiftUI app. The longest-risk milestone; start its BLE spike during M3.

**Build:**
1. **CoreBluetooth driver (§8.4):** central + peripheral, `bluetooth-central`/`bluetooth-peripheral` background modes, state restoration; the "connect while foregrounded, then hold the connection" strategy that keeps GATT links alive in background.
2. **Interop hardening (§8.5):** the **split** cross-platform matrix — the achievable *discovery* pairings (all except Android-central↔backgrounded-iOS-peripheral, an Apple limit) plus all four *established-connection* pairings surviving backgrounding — verifying discovery, MTU, fragment reassembly, SYNC. Not an "all directions" matrix; the impossible discovery case is documented and out of gate.
3. **SwiftUI app:** parity with the M4 Android UI; Keychain for the identity key; the iOS honest-degradation banner ("backgrounded iPhones relay less").
4. **Beacon Mode disabled on iOS** with the explanatory settings entry (§15.7).

**Exit gate (revised, finding 4):** the §8.5 interop matrix passes *as split into achievable cases* — all four discovery pairings **except** Android-central↔backgrounded-iOS-peripheral (an Apple platform limit, explicitly out of gate and documented), plus all four established-connection pairings surviving backgrounding. An iOS device participates in a mixed-platform mesh; background degrades gracefully with instant foreground SYNC catch-up.

**Risk (highest single item):** iOS background BLE unpredictability. Mitigation: the early M3-era spike de-risks the driver before the app work commits; the product already frames iOS background as best-effort (§8.4), so the bar is "graceful degradation," not "parity."

---

## 10. M6 — Beacons & Beacon Mode (2–3 eng-weeks)

Coverage infrastructure. Two tracks, both speaking the frozen protocol.

**Build:**
- **In-app Beacon Mode (Android, §15.7):** the config profile (min hold-off, relay prob 1.0, uncapped budget, 8 connections, 60-min cache, continuous scan), the `infra` bit tied to charging state, the dim burn-in-safe status screen with hold-to-exit, the 30%-battery auto-downgrade and auto-beacon-while-charging toggle.
- **Hardware beacon (§15.1):** start with the zero-code path — retired Android phones in Beacon Mode. Then an ESP32-S3 firmware port of the mesh core's relay path (C/Rust) for the production unit; signed firmware, no listening services (§18-m6). Optional LoRa/Ethernet backbone (§15.3) as a stretch item.

**Exit gate:** a beacon (phone-based first) measurably extends coverage across a sparse gap in a field test; Beacon Mode runs 6+ hours on external power without intervention.

**Risk:** ESP32 firmware is a second implementation of the relay path — divergence risk. Mitigation: ship v1 on Beacon-Mode phones (single codebase); treat ESP32 as a fast-follow that reuses the core's relay logic, tested against the same simulator vectors.

---

## 11. M7 — Hardening, scale & launch (4–6 eng-weeks)

Close the security findings, prove it at scale, prepare release. Spans the whole system.

**Build/validate:**
- **Security closure (§18):** confirm every B/M/m finding's fix is present *and tested* — not just documented. Run the full §13 security regression suite as a release gate. Commission an **external penetration test / crypto review** of the DM and organizer-signing paths — the one place to pay for outside eyes.
- **Adversarial pass:** `flooder` and new attack cases (fragment exhaustion M1, SYNC amplification M2, slot exhaustion M3, cache pollution M7) against a real bench mesh, not just the simulator.
- **Scale/field test (§13 item 4, §14):** 30–50 real devices at an actual gathering — hop counts, latency distribution, delivery ratio vs. density, iOS background degradation in vivo, battery across the device matrix. This is the test that catches what the simulator can't (RF congestion, human-body absorption, real mobility).
- **Store readiness:** privacy nutrition labels (honest about the plaintext-on-air channel layer and the local-only data model), permission-rationale copy, the §19 IAP configuration, crash/ANR telemetry (content-free).
- **Docs:** organizer onboarding guide (key generation, QR distribution, beacon deployment §15.4), a support FAQ pre-empting the battery and iOS-reliability questions (§11.2).

**Exit gate:** external review clean or all findings remediated; scale test hits acceptable delivery at realistic density; store submissions accepted; rollback/hotfix path in place.

**Risk:** the field test reveals a density/battery/iOS problem too late to fix pre-launch. Mitigation: run a *smaller* field test at the end of M3 (10 devices) and again mid-M4 — don't let M7 be the first time real crowds touch the mesh.

---

## 12. Cross-cutting workstreams

Run continuously, not as milestones:

- **Security gates (§12.1, §13):** parser + deep-link fuzzing, crypto KATs, supply-chain audit, the trust-chrome assertion tests, no-content-logging grep gate — all in CI from the moment the relevant code exists. A red gate blocks merge.
- **Simulator maintenance:** as protocol details evolve, the simulator stays the source of truth for mesh behavior; every relay/suppression change re-runs the §13 acceptance sweep.
- **Design-doc sync:** the design doc is the spec; any implementation-forced change (a tuned threshold, a packet-layout tweak) updates the doc in the same PR so the two never drift.
- **Device lab:** the physical test matrix (mixed Android OEMs + iOS generations) grows across M3–M7 and is the ground truth for battery and interop claims.

---

## 13. Definition of done for v1

- All §0.1 features usable end-to-end on Android and iOS, in a mixed-platform mesh.
- Every §18 security finding fixed and covered by an automated test or CI gate.
- Simulator acceptance targets (§13) met; a real ≥30-device field test hits acceptable delivery at festival-realistic density.
- External crypto/pen review of DM + organizer paths remediated.
- Honest-by-construction UI: every not-private / not-verified / rate-limited / iOS-degraded state is present and correct.
- Store-accepted builds with content-free telemetry and truthful privacy labeling.
- Organizer + support documentation shipped.

---

## 14. Biggest risks, ranked

1. **iOS background BLE (M5).** The single hardest technical unknown. De-risk with an early spike; frame as graceful degradation. Beacons (M6) are the structural mitigation.
2. **Real-crowd mesh behavior at scale (M7).** Simulator can't fully model RF congestion + human absorption + mobility. Mitigation: incremental field tests from M3 onward, not one big test at the end.
3. **OEM battery-killers & BLE quirks (M3).** Can silently break the mesh on specific popular phones. Mitigation: broad device matrix early, OEM survival as an acceptance item.
4. **Crypto correctness (M2).** Deanonymization / key-continuity bugs are severe. Mitigation: KAT + tamper + tag-boundary tests written alongside the code; external review in M7.
5. **Suppression tuning (M1).** Hitting dense-and-sparse targets with one parameter set. Mitigation: cheap simulator sweeps; thresholds as config.
6. **UI trust/decoration blur (M4).** A spoofable avatar/color reading as verified. Mitigation: assertion-tested invariant that fails CI.
