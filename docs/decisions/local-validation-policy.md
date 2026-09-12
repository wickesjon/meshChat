# Local validation and merge gates

Approved by the user on 2026-09-11 after hosted Actions could not provision runners. This replaces the requirement for a successful full hosted matrix on every PR, including MC-007. Required checks are selected by the change's effects and ticket acceptance criteria; they may execute locally. Separate Terra medium review remains mandatory. The approval extends MC-007's documentation scope to this policy and the corresponding AGENTS.md rule.

## Selecting checks

| Change | Required validation |
|---|---|
| Every PR | Ticketboard default validation, ticketboard unit tests, `git diff --check`, ticket-specific checks and Terra review |
| Ticket move/dependency change | Regenerate the board with `python -B tests/ticketboard/validate.py --write`, then run default validation |
| Documentation, decision or fixture definitions | Check internal references and affected specification/fixture validators; changing a specification alone does not claim its implementation or platform acceptance passed |
| Rust implementation or dependency changes | Pinned-toolchain formatting, clippy, debug/release tests, release build, lockfile consistency and cargo-deny gates from `.github/workflows/ci.yml`, plus affected vectors/fuzz/security checks |
| Android code, packaging or native dependencies | Affected Android build, lint, tests, APK/ABI/alignment checks and emulator/security checks from the workflow |
| iOS code, packaging or native dependencies | Affected Xcode/Swift build and regression checks from the workflow on a Mac with the pinned Xcode version |
| Shared FFI, exported API, build scripts or dependencies used by both platforms | Relevant checks on both native consumers as well as host Rust checks; host success alone is insufficient |
| CI or validation infrastructure | Validate changed workflow/scripts and exercise their affected checks; a policy-only change does not demonstrate runner execution |

Assess transitive effects, not just file extensions. An internal Rust change with no native interface/build effect can use host component checks; the reviewer must confirm the rationale for omitted platform jobs. An unavailable required platform check remains a blocker for that change. Unrelated platform jobs are not prerequisites for documentation-only PRs.

For MC-007, the changed runtime artifacts are Python arithmetic/definition validators and JSON scenario inputs, not application code, native packages or dependencies. Required checks are:

```text
python -B tests/simulator/scenarios/validate_definitions.py
python -B tests/simulator/scenarios/budget_worksheet.py
python -B tests/ticketboard/validate.py
python -B -m unittest discover -s tests/ticketboard -v
git diff --check
```

These verify scenario definitions, arithmetic and repository consistency. They do not execute the production simulator or establish physical delivery, latency, battery or cryptographic results.

## Execution and evidence

Use the workflow's pinned versions and equivalent commands for applicable component checks. Keep generated outputs, toolchains, caches and temporary files inside the repository. Existing repo-local Windows Rust tooling supports host checks; the Android native build scripts currently require Linux/macOS tooling, and iOS requires macOS/Xcode. Windows host tests are not evidence of Linux, Android or iOS execution. Do not automatically install WSL or change virtualization/security settings.

Do not run `tests/bench/security/prepare_emulator_disk.py` on a developer checkout: it is a guarded Linux CI disk-reclamation step that removes build/tool caches. It is not a correctness check. Local execution should retain caches and provide adequate disk space for required emulator tests.

Record the exact source revision, host/tool versions, commands, outcomes, unavailable checks and why each omitted job is irrelevant in the ticket or PR. Review must cover the final revision. Repeat affected checks after substantive fixes; a passing earlier revision does not cover changed code. Local logs may remain in ignored `.work/`, with a concise durable evidence record in the PR/ticket.

Hosted Actions may remain supplemental. A runner that never started is unavailable evidence, not a passing or failing source test. A real failure in a relevant hosted or local check must be investigated and resolved before merge. This policy does not authorize ignoring test failures, overriding branch protection, or modifying account billing/settings. If remote protection demands unavailable hosted statuses, report that separate blocker.

Hardware acceptance, MC-043/044 physical storage verification, independent construction/transcript review, integrated security assessments, and release gates are unchanged. Terra review is an actual separate-agent code/document review, not a substitute for those independent security assessments. A shared author account may record COMMENT review evidence; do not fabricate formal self-approval.
