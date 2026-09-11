---
id: "MC-002"
title: "Build layout and continuous integration"
depends_on: ["MC-001"]
kind: "foundation"
branch: "ticket/MC-002-build-layout-and-continuous-integration"
---

# MC-002 — Build layout and continuous integration

## Objective

Create the Rust workspace under src/core and native project skeletons under src/android and src/ios; retain conventional platform source nesting.

## Dependencies

`MC-001` See the [current ticket index](../README.md#ticket-index). Dependencies must be complete on main before implementation starts.

## Scope

Permitted paths (relative to repository root): `src/core/**`, `src/android/**`, `src/ios/**`, `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `.github/**`, `.gitignore`.

Also permitted: this ticket and generated ticketboard index/diagram changes required by its workflow. No unrelated file changes or work outside the repository. Read the [design](../../mesh-chat-design.md), its §0 corrections, and the [active plan](../implementation-plan.md). A necessary change outside these paths needs an explicit scope decision.

## Implementation details

- Create the Rust workspace under src/core and native project skeletons under src/android and src/ios; retain conventional platform source nesting.
- Pin toolchains and dependency versions; enable forbidden unsafe code in owned core code, checked release overflow, formatting and lint gates.
- Configure Linux Rust and Android checks plus macOS Swift builds; run ticketboard validation on every PR and add dependency audit/license checks.

## Exit criteria

- [ ] Rust tests, formatting, strict linting and dependency checks pass from a fresh checkout.
- [ ] Android skeleton builds; the macOS job builds the iOS skeleton with recorded SDK/toolchain versions.
- [ ] No generated build output, credentials, or signing material is tracked.
- [ ] Relevant checks pass, evidence is recorded, required review is complete, and the ticket is squash merged to main.

## Potential fallbacks

- If macOS runners are unavailable, record the blocker and use a documented local Mac build for development; do not label the cross-platform gate passed.
- Document any audited dependency exception with scope and expiry instead of disabling the entire check.

A triggered fallback must be recorded with evidence. It does not authorize weaker security, invented validation or expanded scope.

## Evidence

Not implemented. Record commands, versions, reproducible inputs and results here. For manual/hardware checks include device/OS, duration and report paths. No test or review is claimed yet.

## Review and merge

- Branch: `ticket/MC-002-build-layout-and-continuous-integration`.
- Review/PR: pending.
- Squash commit title: `MC-002: Build layout and continuous integration`.
- Completion becomes effective only when the reviewed squash commit lands on main.
