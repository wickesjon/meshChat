# MC-022 assessment remediation scope proposal

Status: **approved by the user on 2026-09-16**. This is the approved response to the completed separate automated construction assessment at `ad87cf890459cd43e1e10991085440ebd6d48cb9` (production `835a7966f4edb2dead99f5768ec24b75059a337c`). The [original assessor report](MC-022-assessment/report.md) remains unchanged; remediation and retest evidence are recorded separately.

## Findings and proposed behavior

- **F-01, organizer replay:** public signed-content replay identity must not depend on whether this device sent or received the message. Preserve direction-specific DM replay identity. Check legacy public ledger entries under both directions so existing outgoing records cannot be reaccepted after an upgrade or restart. Preserve tombstones and conflict refusal; do not delete history or existing evidence. Prefer compatibility-aware reads/canonical new writes without a schema replacement. If a schema migration proves necessary, document and test it before merge. Convert the assessor's defect-reproducing probe into a regression requiring Replay, one accepted history effect, and no renewed pin effect, including reopening, history deletion and conflicting variants.
- **F-02, invalid organizer pin:** retain correctly authenticated, otherwise valid text and suppress the pin when it exceeds the credential/root lifetime. Preserve exact signed bytes. Add boundary, expired, excessive and valid pin tests; outbound construction continues to refuse creating invalid pins.
- **A-01, unsafe signed text:** make Rust acceptance owners enforce the existing forbidden-character policy on message/nickname fields after authentication and before new verified history/authority effects. Reject without normalizing signed bytes. Apply the same policy before unsigned clear-content local acceptance; structural parsing and opaque transport remain distinct. Test NUL, C0/C1, bidi and zero-width cases, valid multibyte text, and invalid-first/valid-second same-ID recovery. Native display still owns normalization, badge-safe presentation and confusable warnings.
- **A-02, integration obligation:** retain explicit native key-loading/QR work accounting and provider-invalidation obligations. Do not mislabel unfinished native feature integration as tested.

## Requested additional MC-022 production paths

`src/core/src/organizer.rs`, `src/core/src/storage.rs`, `src/core/src/friends.rs`, `src/core/src/ingress.rs`, and `src/core/src/text.rs` only as necessary for the three dispositions above. The existing MC-022 scope already permits `tests/integration/**`, `tests/vectors/crypto/**`, `docs/testing/**`, the normative design and its ticket/board records. Add `docs/decisions/MC-008-crypto-contract.md` solely to clarify public-versus-DM replay identity and the mandatory text-acceptance boundary if needed; no weakened requirement or wire change is proposed.

No dependency/root-lockfile changes, new algorithm, wire-version replacement, physical-gate waiver or third-party certification claim. The separate AI report is accurately attributed; it does not become an external human audit.

## Work and merge plan

Preserve the approved, unfinished MC-023 work as a scoped WIP commit on its existing branch before switching to the existing MC-022 PR branch. Record the report and probe evidence in MC-022, implement authorized fixes, and obtain assessor retesting on the corrected revision. Obtain the required Terra medium PR follow-up review separately, waiting for completion without polling. Run affected Rust/storage/native checks, vector compatibility, board and whitespace checks. Keep MC-022 in review until all blocking findings and remaining applicable gates are resolved. Resume MC-023 afterward; its shared native API still requires Android and Swift/Mac validation.

AGENTS.md requires an explicit scope decision before editing necessary paths outside a ticket's permitted paths. MC-022 currently permits tests and documentation but excludes these production fixes. Approval of the MC-023 transport bridge does not silently extend MC-022.
