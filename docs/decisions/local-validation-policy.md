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

## Physical acceptance scheduling — approved 2026-09-14

The user cannot test on real devices until implementation is near completion. Physical checks embedded in implementation tickets MC-023/024/026/033/035 are therefore transferred to the named acceptance gates below. Those implementation tickets may complete with their specified automated tests, applicable native builds and Terra review; their completion does not certify physical behavior. This changes timing and ownership, not supported behavior, acceptance thresholds or evidence quality. Use synthetic data during development until the corresponding MC-043/044 protection gate permits sensitive-data use.

| Implementation work | Required development evidence | Deferred physical owner and deadline |
|---|---|---|
| MC-023 Android GATT driver | Native build/lint plus automated whole/fragmented exchange, directional capacity and backpressure/callback tests | MC-025: two real Android devices, both directions, runtime limits/backpressure; before MC-034 beta |
| MC-024 connection/power policy | Automated duplicate-link, slot/churn, permission/service-stop and power-threshold tests plus native checks | MC-025: Pixel/Samsung/Xiaomi screen-off, OEM termination, permission and connection-limit matrix; before MC-034 |
| MC-026 CoreBluetooth driver | Mac/Xcode builds and automated role, readiness, capacity, lifecycle and restoration-state tests | MC-027: real-device role/frame matrix, background/suspension/disconnect/restoration/foreground and discovery/force-quit results; before MC-036/037/038 |
| MC-033 phone Beacon Mode | Native tests for bounded scheduling/cache, long-duration logical operation and power transitions | MC-025: six-hour powered-phone run, physical power removal/downgrade, sparse-gap coverage and battery/load comparison; before MC-034 |
| MC-035 iOS feature integration | Native builds and UI/integration tests with the production core and synthetic inputs | MC-027: every shipping v1 flow on the supported physical iOS matrix, including reset/restoration, offline purchase cache and catch-up; before MC-036/037/038 |
| MC-017/018 protected identity/storage | Existing MC-005 development contract and synthetic native lifecycle/negative checks | MC-043 after Android feature integration and before MC-034; MC-044 after MC-035 and before MC-037; both retain all physical backup/lock/restore scenarios |

MC-025 now waits for the Android feature implementations; MC-027 waits for MC-035, while MC-035 depends directly on MC-026 and MC-022 instead of physical interop. MC-043/044 also wait for their platform feature integration. The authoritative ticket dependencies enforce this sequencing without cycles. MC-034 keeps Android physical feature acceptance, MC-036 keeps its integrated adversarial device gate, and MC-038 keeps the 30–50-device field gate. Independent MC-022/037 assessments remain separate and unchanged.

Run available emulator/simulator checks during development and record their exact versions, inputs and limitations. Test doubles remain confined to tests; production implementations must retain their actual native adapters. Simulated timing, radio, lifecycle or security results never establish physical capacity, range, battery, OEM background behavior or hardware key protection. Transfer each deferred scenario and its procedure to its owner; do not mark it passed in the originating implementation ticket. Later failures require scoped remediation and retesting before the physical gate closes.

Native compilation does not require a physical handset and is **not deferred by this decision**. Android Studio can install the pinned SDK/NDK and provide emulator tooling, but installation alone is not a successful build. MC-046 supplies Windows Android core/app build support; the [Windows build guide](../testing/windows-android-build.md) distinguishes that coverage from the separate SQLCipher/security-emulator pipeline. iOS still needs Mac/Xcode, including unsigned device/simulator builds and relevant Swift regressions. MC-011 PR #13 requires native dependency checks on its own revision; successful tooling validation on another branch does not close that gate. Missing hardware blocks the scheduled physical gates; missing required native build tooling blocks affected implementation PRs.

Tooling reference: Android's [NDK installation guide](https://developer.android.com/studio/projects/install-ndk?hl=en) explains installing a specific side-by-side version through SDK Manager; this repository pins NDK `27.3.13750724` in its build workflow. This is setup guidance, not evidence of a completed local installation or test.

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

Use the workflow's pinned versions and equivalent commands for applicable component checks. Keep generated outputs, toolchains, caches and temporary files inside the repository. Windows supports host Rust checks and MC-046 Android cross-compilation/app packaging; the SQLCipher rebuild still requires its Unix toolchain, and iOS requires macOS/Xcode. Host JVM FFI tests, Android cross-compilation, emulator execution and physical-device results are distinct evidence. Do not automatically install WSL or change virtualization/security settings.

Do not run `tests/bench/security/prepare_emulator_disk.py` on a developer checkout: it is a guarded Linux CI disk-reclamation step that removes build/tool caches. It is not a correctness check. Local execution should retain caches and provide adequate disk space for required emulator tests.

Record the exact source revision, host/tool versions, commands, outcomes, unavailable checks and why each omitted job is irrelevant in the ticket or PR. Review must cover the final revision. Repeat affected checks after substantive fixes; a passing earlier revision does not cover changed code. Local logs may remain in ignored `.work/`, with a concise durable evidence record in the PR/ticket.

Hosted Actions may remain supplemental. A runner that never started is unavailable evidence, not a passing or failing source test. A real failure in a relevant hosted or local check must be investigated and resolved before merge. This policy does not authorize ignoring test failures, overriding branch protection, or modifying account billing/settings. If remote protection demands unavailable hosted statuses, report that separate blocker.

Hardware acceptance requirements and thresholds remain mandatory with the ownership/timing above. MC-043/044 physical storage verification, independent construction/transcript review, integrated security assessments, and release criteria are not waived. Terra review is an actual separate-agent code/document review, not a substitute for those independent security assessments. A shared author account may record COMMENT review evidence; do not fabricate formal self-approval.
