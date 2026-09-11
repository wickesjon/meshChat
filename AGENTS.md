# Repository instructions

## Scope and layout

These instructions apply to the entire meshChat repository.

- Production code belongs under `src/`: shared Rust in `src/core/`, Android in `src/android/`, iOS in `src/ios/`; approved supporting products may use named subdirectories.
- Tests, simulator, adversarial harnesses, fuzz targets and golden vectors belong under `tests/`. Framework-required source-adjacent tests may be used only when documented in the relevant ticket.
- Design, decisions, operating guides, evidence and tickets belong under `docs/`.
- Root manifests, lockfiles, toolchain files, `AGENTS.md` and conventional CI configuration may remain at their required locations.
- Generated files, build outputs and temporary work belong in ignored paths inside this repository. Configure tool output/cache/temp paths accordingly when necessary. Do not commit secrets, private keys, provisioning QR images or message content.

## Request and repository boundaries

- Modify only files necessary for the approved request and the active ticket's permitted paths. No unrelated cleanup, refactoring, dependency updates, formatting sweeps or opportunistic fixes.
- Do not widen the request or change a product/security requirement through an implementation shortcut. If necessary work exceeds the ticket or request, record a blocker and obtain an explicit scope decision before doing that work.
- Do not create, edit, move or delete files outside the resolved repository root. Do not use symlinks, junctions, external worktrees or generated output paths to evade this boundary. Reading reference documentation is allowed.
- Preserve unrelated user changes. Never overwrite, discard or stage them incidentally.
- Do not change global Git configuration, external projects, machine security settings or remote services as a side effect of ticket work.
- Deployment, store submission, distribution, payment, and messages to other people require explicit authorization for that action.

## Ticket and branch workflow

1. Inspect Git status, this file, the design and the active ticket before editing.
2. Every ticket, including documentation and follow-up fixes, is worked on a dedicated `ticket/MC-NNN-short-description` branch created from current `main`. Never implement a ticket directly on `main`.
3. Start implementation only when hard dependencies are complete on `main`. Drafting/reviewing a blocked ticket is allowed; dependent production implementation is not.
4. Move the ticket through `backlog/` → `inprogress/` → `inreview/` → `complete/`. The folder is the status source of truth; never keep duplicate copies.
5. Keep `depends_on` authoritative. After a move or dependency change, run `python tests/ticketboard/validate.py --write`, then the default validator. It checks cycles, IDs, missing dependencies and generated index/diagram consistency.
6. Keep changes within the ticket's permitted paths. Record implementation choices, validation evidence, deviations and the branch/review reference in the ticket.
7. A fallback is conditional, not automatic permission to weaken an invariant or expand scope. Record its trigger and evidence; obtain approval for any changed scope or acceptance requirement.

## Review and squash merge to main

- A ticket is ready for merge only when implementation details and measurable exit criteria are satisfied, all relevant checks pass, applicable security gates pass, dependencies are complete and required review is recorded.
- Run `git diff --check` and ticketboard validation for documentation changes. For code, run the relevant component tests, formatting/lint/build checks and the active security gates. Do not invent passing hardware or external-review evidence.
- The author/agent must not fabricate independent review. Use a review artifact or explicit user review; independently required security assessments remain separate gates.
- Update affected normative design documentation and vectors in the same change. A changed wire contract needs its decision and compatibility consequences documented.
- Squash merge each finished ticket branch into `main` as one commit, with `MC-NNN` in the commit title. Do not use ordinary merge commits or a rebase merge for ticket completion.
- Do not force-push or rewrite `main`. Do not use broad staging that captures unrelated files.
- The final reviewed branch may stage its ticket in `complete/` for the squash commit. That completion becomes effective only when the squash commit lands on `main`; before then the ticket remains pending merge. Record a review/PR reference in the ticket; the squash commit is discoverable by its ticket ID, avoiding a self-referential commit hash.
- A failed check, unmet exit criterion or unresolved blocking review prevents merge. Do not mark an incomplete ticket complete to unblock another.
- Branch deletion after merge is optional and requires ensuring it contains no unmerged work.

## Specification and evidence

- `docs/mesh-chat-design.md` is the normative design, including its approved correction section. Explicit unresolved decisions block their dependent gates.
- `docs/ticketboard/implementation-plan.md` is the active build sequence. The original implementation plan is historical.
- Every ticket must contain implementation details, measurable exit criteria and potential fallbacks.
- Use evidence states: specified, implemented, tested and independently reviewed where applicable. A documented mitigation is not proof of implementation or security.
- Keep approved v1 scope intact. ESP32 firmware and optional backbone transport are follow-on work, not hidden prerequisites of the phone-based v1 release.

## Bootstrap record

The user established the initial baseline on `main` from the two original documents (commit `dfc1a3f`) and created `ticket/MC-001-project-planning` before these planning edits. This is the one-time repository bootstrap; subsequent ticket work follows the branch-and-squash rules above.
