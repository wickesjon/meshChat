# MC-025 bounded Android catch-up

## Intended behavior

Android retains one pending catch-up attempt for each newly admitted link. It lets initial presence traffic proceed and waits for the native link observation to report valid live CHAT/ANNOUNCE activity before requesting history. Closed/stopped links lose their pending attempt; the existing 20-second initial-activity rule remains unchanged. It does not start another walk when a session finishes, is limited, or is refused. Native per-node/per-link request admission, session deadlines, newest-first mixed-channel selection, eight-item/8192-byte caps, and forwarding-cache retention remain authoritative. A later newly admitted connection may make another bounded request under those same limits; this is not full-history replication.

Owned results pass through `EncryptedStorage.processCatchup`, which opens the existing protected store and short-lived identity session and closes them on return. Signed/encrypted history follows the native authentication path; a historical label grants no trust. Recovered public rows receive a session-local label, bounded to 100 identifiers. The UI always qualifies history as potentially incomplete. Lock/unavailable owner teardown clears these in-memory labels and state; persisted history is retained by the existing store policy. No wire/native API, crypto or retention contract changes.

## Paired physical procedure

Use two unlocked, already onboarded devices with synthetic profiles. Build/install the candidate app and its test APK in place. Run `org.meshchat.ui.PhysicalCatchupTest` through the normal AndroidJUnitRunner concurrently on both devices with explicit `physicalCatchup=true`, the same never-reused alphanumeric `runId` (1–16 characters), and `role=A` (source) / `role=B` (receiver). Existing emulator-only scripts must keep rejecting physical serials.

Predeclared assertions:

1. No fixture marker exists in either endpoint's current channel history before the run.
2. A starts with nine eligible, short, unverified public CHAT cache entries from three stable synthetic claimed senders (three each), plus one expired entry. This isolates the item cap from the ordinary single-sender burst allowance. The expired entry is removed by native cache aging; exactly nine remain before radio startup.
3. B starts a normal real radio connection. Production Android code requests/drains catch-up; the test never requests or processes catch-up on B's behalf.
4. B receives exactly the newest eight fixture entries. The ninth (oldest eligible) and expired entry are absent. Recovered rows have no self/verified-friend/signed claim and carry the recovered-history label.
5. After completion, keep the same native link for 30 additional seconds. No automatic extra walk recovers the ninth item. B's live forwarding cache and relayed-chat count remain zero before its confirmation message.
6. B sends one new synthetic confirmation through the normal composer. A must receive it over the physical link. The tests stop transport, clear their foreground KEEP_SCREEN_ON flags, and let the runner close the activities. Existing profile/history/keys remain intact.

The cache fixture is intentionally distinct from the physical transfer. On A only, before radio startup, the test constructs a replacement native transport bound to the existing protected store/identity. Synthetic native HELLO/admission and incoming frames seed its forwarding cache without adding entries directly to the recipient's history. An initial monotonic timestamp 17 minutes before current uptime exercises expiry. Nine recent entries, cycling among three fixed claimed senders, are then seeded at 12-second monotonic intervals starting two minutes before current uptime, respecting the normal five-per-minute sender allowance; the fixture finally advances to real current uptime. No OS clock or production budget is edited. Synthetic link state is closed before the real radio starts. This does **not** prove three real authors/devices transmitted the fixture, a 17-minute wall-clock disconnection, or actual native capacity during seeding. Native seed admission still must succeed; no cache mutation/reflection into Rust internals or budget refill bypass is used. Earlier C01–C05 used a single claimed sender and remain separate evidence: source pacing alone does not prevent receiver-side burst limits during fast historical replay.

Aggregate waiting/fixture/result events are logged without message content. Optional `traceCatchup=true` additionally observes frame lengths, framing/SYNC headers, submission booleans and closure events through test-only wrappers that preserve the original callbacks and protected egress decision. Use tracing for diagnosis, not performance certification. The exact-eight assertion fails immediately once a terminal history notice appears with fewer than eight selected rows.

Record candidate hashes, fixture/assertion events and complete runner outcomes under ignored repository-local evidence paths. Polling/model refresh and setup contribute to observation time; the test's 180-second receipt window is functional coverage, not the formal 120-second first-request-frame measurement. These small items exercise the item cap, not the 8192-byte ceiling. Full simultaneous catch-up, authenticated physical vectors, wire accounting, negotiated capacity, control/reaction contention, long-outage and MC-007 acceptance remain separate scenarios.

## Validation status

Implementation and paired physical results are recorded in the [active ticket](../ticketboard/inprogress/MC-025-android-bench-mesh-and-early-field-gate.md) and [results report](MC-025-results-report-2026-09-26.md). A pass here does not complete scenario E or MC-025.
