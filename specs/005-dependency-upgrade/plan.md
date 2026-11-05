# Implementation Plan: Dependency and Toolchain Upgrade

**Branch**: `005-dependency-upgrade` | **Date**: 2026-11-05 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/005-dependency-upgrade/spec.md`

## Summary

Upgrade the entire Thriftp dependency surface to the latest stable releases, resolve the current dependency advisories, and restore a green cross-platform build without changing product behavior or security guarantees. The work is delivered as reviewable batches: baseline capture, toolchain and CI modernization, Rust dependency majors, frontend tooling majors, desktop runtime alignment, and final evidence/cleanup.

The upgrade uses current stable targets: Rust 1.99.1, Node 26.13.0 LTS, Tauri 2.12.5, Vite 8.3.6, Vitest 5.1.0, ESLint 10.14.0, jsdom 31.0.0, and the corresponding latest stable Rust crates. TypeScript 7.0.2 is the only planned exception: the latest `typescript-eslint` release still declares TypeScript `<6.1.0`, so the project will use TypeScript 6.0.3 until upstream support lands.

## Technical Context

**Language/Version**: Rust 1.99.1 stable (pinned toolchain; effective MSRV 1.90 from Tauri 2.12.5); Node.js 26.13.0 LTS; TypeScript 6.0.3 (TypeScript 7 deferred by peer-dependency constraint)

**Primary Dependencies**: Rust workspace crates and Tauri 2.12.5; Svelte 5.57.3; Vite 8.3.6; Vitest 5.1.0; ESLint 10.14.0; jsdom 31.0.0; Prettier 3.9.10; Tauri CLI 2.12.4; `@tauri-apps/api` 2.12.4

**Storage**: Existing JSON state files and operating-system credential store; no schema or storage change

**Testing**: `cargo fmt`, `cargo clippy` with warnings denied, `cargo test --workspace --locked`, `cargo doc` with warnings denied, `cargo bench`, `vitest`, ESLint, Prettier, `tsc`, Desktop shell tests, CLI/MCP contract tests, security tests, containerized protocol integration tests, reproducible-build verification, cross-platform CI matrix

**Target Platform**: macOS, Linux, and Windows desktop; Rust 1.99.1; Node.js 26.13.0 LTS; Tauri 2.12.5

**Project Type**: Multi-crate Rust workspace, Tauri desktop application, Svelte/TypeScript frontend, CLI, MCP server, and containerized protocol test harness

**Performance Goals**: Listing, transfer, queue, and startup performance remain within 5% of the pre-upgrade baseline

**Constraints**: Zero open critical/high advisories; at least 90% of initial alerts resolved; no user-visible behavior, public interface, data-format, or security regression; all required checks green on macOS, Linux, and Windows; no force-resolution or legacy-peer workarounds; every exception documented and time-bound

**Scale/Scope**: 8 Rust workspace crates, 1 desktop shell crate, 377 npm packages across production and development, 3 lock/resolution records, 2 test container images, and the full CI workflow

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **I. Product Vision**: PASS. The upgrade improves reliability and security without adding product scope.
- **II. Architecture**: PASS with conditions. No architectural change is permitted; the core remains Tauri-independent, protocol interfaces stay stable, and new dependencies are allowed only when replacing an existing one or satisfying a documented compatibility need.
- **III. Security**: PASS with required review. Zero critical/high advisories, unchanged credential-store and TLS/host-key behavior, secret redaction preserved, and explicit review for credential, cryptography, SSH, and transport upgrades.
- **IV. User Experience**: PASS. The upgrade must preserve navigation speed, native behavior, keyboard access, themes, and transfer responsiveness.
- **V. Quality**: PASS. Existing tests, new regression coverage, formatting, linting, documentation, benchmarks, and cross-platform CI are mandatory gates.
- **VI. Open Source**: PASS. License checks, SBOM regeneration, no mandatory telemetry, and no cloud dependency are required.
- **VII. AI-Assisted Development**: PASS. The spec and plan are the source of truth; architectural changes require separate approval; tests must run before completion.
- **VIII. Automation & Integration**: PASS. CLI and MCP contract tests and security parity are required; the upgrade must not create a weaker automation path.

No constitutional violations require justification. The TypeScript 7 deferral is a dependency-compatibility exception, not an architectural exception, and is recorded in the upgrade exception register.

### Post-Design Re-Evaluation

The Phase 1 design preserves every gate:

- The compatibility contract freezes desktop, CLI, MCP, protocol, credential, transfer, favorites, and data-format behavior.
- The inventory, exception, and verification schemas make security and quality evidence reviewable without introducing runtime architecture.
- The batch plan keeps the core Tauri-independent and isolates SSH, cryptographic, credential, frontend, and runtime changes.
- The verification plan retains all existing suites, adds dependency-driven regression coverage, and requires cross-platform, performance, license, SBOM, and reproducible-build evidence.
- The only planned exception, TypeScript 7, is time-bound and does not weaken lint coverage.

No gate failure or unjustified violation remains.

## Project Structure

### Documentation (this feature)

```text
specs/005-dependency-upgrade/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── baseline.md                   # pre-upgrade measurements and commit
├── contracts/
│   ├── compatibility-contract.md
│   ├── dependency-inventory.schema.json
│   ├── upgrade-exception.schema.json
│   └── verification-evidence.schema.json
├── upgrade-inventory.md          # human-readable inventory narrative
├── inventory.json                # authoritative inventory records
├── exceptions.md                 # human-readable exception narrative
├── exceptions.json               # authoritative exception records
├── evidence.json                 # authoritative verification evidence
├── security-review.md            # security-sensitive upgrade review
├── validation.md                 # human-readable evidence summary
└── tasks.md                      # created by /speckit.tasks
```

### Source Code (repository root)

```text
Cargo.toml
Cargo.lock
CHANGELOG.md
rust-toolchain.toml

crates/
├── thriftp-core/
├── thriftp-protocols/
├── thriftp-credentials-keyring/
├── thriftp-storage-json/
├── thriftp-cli/
├── thriftp-mcp/
└── thriftp-tests/

tests/integration/

apps/desktop/
├── package.json
├── package-lock.json
├── src/
└── src-tauri/
    ├── Cargo.toml
    └── Cargo.lock

.github/workflows/ci.yml

docker/
├── ftp/Dockerfile
└── sftp/Dockerfile

scripts/
docs/
```

**Structure Decision**: No new runtime modules or packages are introduced. The upgrade changes manifests, lock/resolution records, toolchain pins, CI configuration, container definitions, and only the source code required to migrate breaking APIs. Documentation and verification evidence live under `specs/005-dependency-upgrade/`, with durable contributor guidance updated under `docs/`.

## Implementation Approach

### Batch 1 — Baseline and guardrails

Capture the pre-upgrade baseline before changing versions: test results, benchmark results, audit counts, license report, SBOM, artifact sizes, startup timing, and the current dependency inventory. Record the exact starting versions and the current advisory set so improvement is measurable. Add the inventory and exception record formats defined by the contracts.

### Batch 2 — Toolchain, CI, and test infrastructure

Upgrade the pinned Rust toolchain to 1.99.1 and Node.js to 26.13.0 LTS. Publish the effective Rust MSRV as 1.90.0 and the Node engine floor as 26.x in the project metadata. Update CI actions to immutable commit pins: `actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1` (v7.0.1), `actions/setup-node@949feb2413d6458794dcd2491c4babbbce0c15c1` (v7.1.0), `actions/upload-artifact@cf430e030ddbb5b0abf93d22962f4752f3646cd9` (v7.0.2), `Swatinem/rust-cache@2201cc584a476d1b508be977bbe4b7d1c7a52e9c` (v2.9.3), and `dtolnay/rust-toolchain@d124f86f28ff42855784ff3319f48ac182c248f8` (1.99.1). Select and document explicit current GitHub-hosted runner image labels for macOS, Linux, and Windows; do not use floating `-latest` tags. Rebuild the FTP test image on Alpine 3.23 and the SFTP test image on Debian 13 slim. Verify that the pipeline still runs before touching application dependencies.

### Batch 3 — Rust dependency majors

Upgrade the Rust workspace in low-risk-to-high-risk order: general utilities, then SSH/FTP, then credential and cryptographic crates, then benchmarks. The Tauri shell and plugins are aligned in Batch 5 after the core and protocol APIs have settled. Key migrations include `russh` 0.54 → 0.65.1, `ed25519-dalek` 2 → 3.0.0 with `rand_core` 0.10.1 and `sha2` 0.11.1, `keyring` 3 → 4.2.0, `base64` 0.22 → 0.23.1, and `criterion` 0.5 → 0.8.2. Compile and test after each compatibility group, and run the security suite before moving on.

### Batch 4 — Frontend and tooling majors

Upgrade the UI toolchain to Vite 8.3.6, Vitest 5.1.0, `@sveltejs/vite-plugin-svelte` 7.3.1, ESLint 10.14.0, `eslint-plugin-svelte` 3.24.0, jsdom 31.0.0, Prettier 3.9.10, and `prettier-plugin-svelte` 4.1.1. Upgrade TypeScript to 6.0.3, the latest release supported by `typescript-eslint` 8.72.1. Keep TypeScript 7.0.2 out with a time-bound exception until `typescript-eslint` supports it. Regenerate `package-lock.json` and resolve peer-dependency changes through compatible versions, not force flags.

### Batch 5 — Desktop shell and runtime alignment

Align `tauri`, `tauri-build`, `tauri-plugin-dialog`, `@tauri-apps/api`, and `@tauri-apps/cli` on the current stable Tauri 2 line. Update the desktop shell lockfile, verify the capability and permission configuration still builds, and run the desktop tests on all platforms. Do not move to Tauri 3 pre-releases.

### Batch 6 — Cleanup, evidence, and release readiness

Remove unused dependencies, consolidate duplicate versions where compatible, refresh the SBOM and license report, run the full benchmark comparison, and complete the upgrade inventory, exception register, validation evidence, and release notes. Confirm the final alert count, verify the inventory contains no unapproved pre-release or end-of-life component, confirm the provenance and security reviews are recorded, run the complete cross-platform CI matrix, verify that every documented exception is still necessary, and confirm that no upgraded dependency adds mandatory telemetry, an account requirement, a cloud call, or a new plaintext secret path.

## Rollback Strategy

Each batch is a separate reviewable change with its own verification evidence. A destabilizing batch is reverted as a unit, or its problematic dependency is pinned with a documented exception while unrelated fixes remain. Lock/resolution records are restored together with the manifest change, and the previous benchmark and test results remain the reference. No batch is merged with a red required check.

## Complexity Tracking

No constitutional violations or added architectural complexity. The only intentional deviation from the latest-stable rule is the TypeScript 7 deferral, which is recorded as a time-bound compatibility exception rather than a complexity exception.

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| None | — | — |
