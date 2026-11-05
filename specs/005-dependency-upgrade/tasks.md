---
description: "Task list for the dependency and toolchain upgrade"
---

# Tasks: Dependency and Toolchain Upgrade

**Input**: Design documents from `/specs/005-dependency-upgrade/`

**Prerequisites**: `plan.md`, `spec.md`, `research.md`, `data-model.md`, `contracts/`

**Tests**: Included because the specification requires regression coverage, security verification, and cross-platform validation. Write or update the relevant tests before declaring a batch complete.

**Organization**: Tasks are grouped by user story. US1 and US2 are both P1; US2 is sequenced first because it produces the dependency versions that US1 verifies. The remaining P1 stories follow, then the P2/P3 stories, then final polish.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an incomplete task)
- **[Story]**: User story label (`US1`–`US7`), only in user story phases
- Every task includes an exact repository-relative file path or an evidence output path

## Path Conventions

- Rust workspace: `Cargo.toml`, `Cargo.lock`, `crates/*/`, `tests/integration/`
- Desktop app: `apps/desktop/`, `apps/desktop/src-tauri/`
- Automation: `.github/workflows/ci.yml`, `docker/`, `scripts/`
- Feature evidence: `specs/005-dependency-upgrade/`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Create the evidence and validation scaffolding used by every batch.

- [X] T001 Create `specs/005-dependency-upgrade/baseline.md`, `specs/005-dependency-upgrade/upgrade-inventory.md`, `specs/005-dependency-upgrade/exceptions.md`, and `specs/005-dependency-upgrade/validation.md` with the headings and fields required by `specs/005-dependency-upgrade/data-model.md`, and initialize `specs/005-dependency-upgrade/inventory.json`, `specs/005-dependency-upgrade/exceptions.json`, and `specs/005-dependency-upgrade/evidence.json` as empty JSON arrays.
- [X] T002 [P] Implement `scripts/validate-dependency-records.py` to validate every element of `specs/005-dependency-upgrade/inventory.json`, `specs/005-dependency-upgrade/exceptions.json`, and `specs/005-dependency-upgrade/evidence.json` against the corresponding item schema in `specs/005-dependency-upgrade/contracts/*.schema.json`; fail on missing files, duplicate IDs, and broken cross-references, and when invoked with `--release`, also fail on an empty inventory or evidence register. Use the Python 3 standard library only, or document and security-review any added JSON Schema dependency before adoption.
- [X] T003 [P] Add validator coverage in `scripts/tests/test_validate_dependency_records.py`, including valid records, missing required fields, an unapproved exception, and an expired exception; document and use `python3 -m unittest discover -s scripts/tests -p 'test_*.py'` as the test runner.
- [X] T004 Add a short usage section for the validator to `docs/testing.md` so contributors can run it against feature records.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Capture the pre-upgrade baseline and modernize shared toolchain, CI, and test infrastructure. No dependency batch may start before this phase is complete.

**⚠️ CRITICAL**: Baseline evidence and the toolchain/CI/container batch must be finished before any user story work begins.

- [X] T005 Before any upgrade edit, capture the full pre-upgrade baseline in `specs/005-dependency-upgrade/baseline.md`: Rust, Node.js, npm, desktop runtime, package-manager, CI-action, and container-image versions; the complete direct and transitive dependency inventory with previous versions; the starting commit obtained with `git rev-parse HEAD`; and the toolchain file contents. Seed `specs/005-dependency-upgrade/upgrade-inventory.md` with the pre-upgrade entries.
- [X] T006 Run the existing workspace tests, desktop tests, frontend tests, and record pass/fail counts and durations in `specs/005-dependency-upgrade/baseline.md`.
- [X] T007 Run the existing benchmark targets and record listing, transfer, queue, startup, and artifact-size measurements in `specs/005-dependency-upgrade/baseline.md`.
- [X] T008 Capture the starting `npm audit`, Dependabot, `cargo audit`, license, SBOM, and reproducible-build results in `specs/005-dependency-upgrade/baseline.md`.
- [X] T009 Capture the current CLI/MCP contract outputs, desktop IPC expectations, and saved-data fixtures in `specs/005-dependency-upgrade/baseline.md`.
- [X] T010 [P] Update `rust-toolchain.toml` to Rust 1.99.1 and set the effective MSRV to 1.90.0 in `Cargo.toml` and `apps/desktop/src-tauri/Cargo.toml`.
- [X] T011 [P] Add a Node.js 26.13.0 engine requirement to `apps/desktop/package.json` and update the contributor prerequisites in `README.md` and `docs/testing.md`.
- [X] T012 [P] Update `.github/workflows/ci.yml` to use immutable action pins: `actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1` (v7.0.1), `actions/setup-node@949feb2413d6458794dcd2491c4babbbce0c15c1` (v7.1.0), `actions/upload-artifact@cf430e030ddbb5b0abf93d22962f4752f3646cd9` (v7.0.2), `Swatinem/rust-cache@2201cc584a476d1b508be977bbe4b7d1c7a52e9c` (v2.9.3), and `dtolnay/rust-toolchain@d124f86f28ff42855784ff3319f48ac182c248f8` (1.99.1), with Node.js 26.13.0.
- [X] T013 [P] Update `docker/ftp/Dockerfile` to use `alpine:3.23`, refresh and pin all installed packages to current supported versions, preserve the existing FTP and FTPS service behavior, and record the base-image and package inventory in `specs/005-dependency-upgrade/validation.md`.
- [X] T014 [P] Update `docker/sftp/Dockerfile` to use `debian:13-slim`, refresh and pin all installed packages to current supported versions, preserve the existing SSH/SFTP service behavior, and record the base-image and package inventory in `specs/005-dependency-upgrade/validation.md`.
- [X] T015 Run the existing local checks after the toolchain, CI, and container changes and record the results in `specs/005-dependency-upgrade/validation.md`.
- [X] T016 Build the updated test containers and run `scripts/test-integration.sh`; record image digests, base-image versions, installed package versions, and test results in `specs/005-dependency-upgrade/validation.md`.
- [X] T017 Add a dependency-record validation step to `.github/workflows/ci.yml` that runs `scripts/validate-dependency-records.py` against `specs/005-dependency-upgrade/inventory.json`, `specs/005-dependency-upgrade/exceptions.json`, and `specs/005-dependency-upgrade/evidence.json`, and runs `python3 -m unittest discover -s scripts/tests -p 'test_*.py'`; fail the job when any required file is missing, malformed, fails schema validation, or any validator test fails.
- [X] T018 Verify `specs/005-dependency-upgrade/baseline.md` still contains only pre-upgrade measurements and the commit captured in T005, note the toolchain/CI/container changes in T010–T017 as post-baseline setup, and freeze the baseline before proceeding. Do not replace the baseline commit with a post-change hash.

**Checkpoint**: Baseline is captured, the pinned toolchain and CI actions are current, test containers are rebuilt, and the validation scaffolding works.

---

## Phase 3: User Story 2 - Bring every dependency to the latest stable release (Priority: P1)

**Goal**: Every direct dependency, runtime component, build tool, automation action, and test container is on its latest stable release, with deterministic resolution records.

**Independent Test**: Compare the committed inventory and resolution records against the target matrix in `specs/005-dependency-upgrade/research.md`; every direct dependency matches the latest stable release or has a validated exception, and clean installs resolve without force flags.

### Implementation for User Story 2

- [X] T019 [US2] Upgrade utility Rust crates in `Cargo.toml` (`async-trait`, `chrono`, `clap`, `serde`, `serde_json`, `tokio`, `trash`, `tracing`, `tracing-subscriber`, `uuid`, `zeroize`, `anyhow`, `tempfile`, `thiserror`) and migrate affected call sites.
- [X] T020 [US2] Upgrade SSH, FTP, and TLS crates in `Cargo.toml` and `crates/thriftp-protocols/Cargo.toml` (`russh` 0.65.1, `russh-sftp` 3.0.1, `suppaftp` 12.2.1, `native-tls` 0.2.18) and migrate the protocol adapters.
- [X] T021 [US2] Upgrade cryptographic crates in `Cargo.toml`, `crates/thriftp-core/Cargo.toml`, and `crates/thriftp-mcp/Cargo.toml` (`ed25519-dalek` 3.0.0, `rand_core` 0.10.1, `sha2` 0.11.1, `base64` 0.23.1) and migrate signing, hashing, and encoding code.
- [X] T022 [US2] Upgrade the credential-store crate in `Cargo.toml` and `crates/thriftp-credentials-keyring/Cargo.toml` (`keyring` 4.2.0) and migrate the native credential backend.
- [X] T023 [US2] Upgrade `criterion` to 0.8.2 in `Cargo.toml` and migrate every benchmark in `crates/thriftp-core/benches/`, `crates/thriftp-protocols/benches/`, `crates/thriftp-cli/benches/`, and `crates/thriftp-mcp/benches/`.
- [X] T024 [US2] Upgrade all npm dependencies and devDependencies in `apps/desktop/package.json`, including Vite 8.3.6, Vitest 5.1.0, ESLint 10.14.0, Svelte plugin 7.3.1, jsdom 31.0.0, and TypeScript 6.0.3.
- [X] T025 [US2] Update `apps/desktop/vite.config.ts`, `apps/desktop/vitest.config.ts`, `apps/desktop/eslint.config.js`, and `apps/desktop/tsconfig.json` for the upgraded Vite, Vitest, ESLint, and TypeScript versions.
- [X] T026 [US2] Align the Tauri stack in `apps/desktop/src-tauri/Cargo.toml`, `apps/desktop/src-tauri/tauri.conf.json`, and `apps/desktop/package.json` on `tauri` 2.12.5, `tauri-build` 2.7.4, `tauri-plugin-dialog` 2.8.4, `@tauri-apps/api` 2.12.4, and `@tauri-apps/cli` 2.12.4.
- [X] T027 [US2] Replace only dependencies that must be replaced for compatibility with the upgraded baseline in `Cargo.toml`, `crates/*/Cargo.toml`, and `apps/desktop/package.json`; leave unused, duplicate, and abandoned dependency cleanup to US7 T092–T094. For every new source, fork, alternate registry, or patched package, review provenance, maintenance status, license, and security, and record the decision in `specs/005-dependency-upgrade/inventory.json`, `upgrade-inventory.md`, and `security-review.md`.
- [X] T028 [US2] Regenerate `Cargo.lock` and `apps/desktop/src-tauri/Cargo.lock` with the official tooling and verify both workspaces resolve consistently.
- [X] T029 [US2] Regenerate `apps/desktop/package-lock.json` with `npm install` and verify there are no invalid peer dependencies or force-resolution requirements.
- [X] T030 [US2] Migrate Rust call sites for the upgraded APIs in `crates/thriftp-protocols/src/`, `crates/thriftp-core/src/`, `crates/thriftp-credentials-keyring/src/`, `crates/thriftp-mcp/src/`, `crates/thriftp-cli/src/`, and `apps/desktop/src-tauri/src/`.
- [X] T031 [US2] Migrate frontend call sites for the upgraded Svelte, Tauri, Vite, and testing APIs in `apps/desktop/src/`.
- [X] T032 [US2] Complete `specs/005-dependency-upgrade/inventory.json` and `upgrade-inventory.md` for every changed dependency, including libraries, runtime components, toolchains, package managers, automation actions, and container images. Record previous version, target version, update type, reason, migration notes, source/provenance, and any exception ID, and confirm that no pre-release or end-of-life component is selected without an approved exception.
- [X] T033 [US2] After each US2 sub-batch (utility crates, SSH/FTP, cryptography, credentials, benchmarks, npm tooling, and Tauri alignment), run the relevant Rust and frontend compile/test smoke checks and record a separate evidence record per sub-batch in `specs/005-dependency-upgrade/evidence.json`, summarized in `validation.md`; do not start the next sub-batch until the current one is green.
- [X] T034 [US2] Audit manifests and resolution records for manual edits, force flags, legacy-peer flags, and skipped install scripts; remove any workaround and record the clean result in `specs/005-dependency-upgrade/validation.md`.
- [X] T035 [US2] Record the TypeScript 7.0.2 deferral as a complete approved exception `EX-001` in `specs/005-dependency-upgrade/exceptions.json` and mirror it in `exceptions.md`, including `component`, `currentVersion`, `latestStableVersion`, `reason: "peer-conflict"`, `peerConstraint` naming the `typescript-eslint` `<6.1.0` constraint, `description`, `risk`, `mitigation`, `owner`, `approvedBy`, `createdAt`, `expiresAt`, `reviewDate`, and `status: "approved"`.

**Checkpoint**: The dependency inventory and resolution records reflect the latest stable baseline, with TypeScript 7 explicitly excepted.

---

## Phase 4: User Story 1 - Remove known security exposure from third-party code (Priority: P1)

**Goal**: Resolve critical and high advisories, resolve at least 90% of the initial alerts, and make every remaining finding visible and approved.

**Independent Test**: Run the dependency policy check, `npm audit`, and `cargo audit`; confirm zero critical/high findings, at least 90% of the original alerts resolved, and every remaining finding linked to a valid exception.

### Tests and Automation for User Story 1

- [X] T036 [US1] Add `scripts/check-dependency-policy.sh` to fail when a critical or high advisory remains and wire it into `.github/workflows/ci.yml`.
- [X] T037 [US1] Add policy fixtures and assertions to `scripts/tests/test_validate_dependency_records.py` covering missing approval, expired exceptions, unresolved advisory links, pre-release targets without an exception, `targetVersion` != `latestStableVersion` without `exceptionId`, duplicate IDs, missing cross-references, date-order violations, and a `releaseReady` batch with red evidence; add `scripts/tests/test_dependency_resolution.py` covering duplicate-version consolidation, pre-release rejection, yanked-package recovery, and rollback gating.

### Implementation for User Story 1

- [X] T038 [US1] Run `npm --prefix apps/desktop audit` and `cargo audit`, triage every finding, and record the before/after counts in `specs/005-dependency-upgrade/validation.md`.
- [X] T039 [US1] Resolve remaining npm advisories by upgrading or replacing affected packages in `apps/desktop/package.json` and `apps/desktop/package-lock.json`.
- [X] T040 [US1] Resolve remaining Rust advisories by upgrading or replacing affected crates in `Cargo.toml`, `crates/*/Cargo.toml`, and the workspace lockfile.
- [X] T041 [US1] Conduct the security-sensitive dependency review in `specs/005-dependency-upgrade/security-review.md` for `keyring`, `russh`, `russh-sftp`, `suppaftp`, `ed25519-dalek`, `sha2`, `base64`, `native-tls`, and `tauri-plugin-dialog`. Cover credential storage, SSH/FTP/FTPS/SFTP authentication and transport, cryptography, and secret handling.
- [X] T042 [US1] Run the security, secret-redaction, host-key, and credential suites and record the results in `specs/005-dependency-upgrade/validation.md`.
- [X] T043 [US1] Record any unresolved advisory as a complete time-bound exception in `specs/005-dependency-upgrade/exceptions.json`, mirror it in `exceptions.md`, and validate the completed record with `scripts/validate-dependency-records.py`; do not leave partial entries.
- [X] T044 [US1] Confirm that no alert, advisory, or security check is muted, snoozed indefinitely, or disabled in `.github/`, repository settings, or CI, and document the evidence in `specs/005-dependency-upgrade/validation.md`.
- [X] T045 [P] [US1] Regenerate the SBOM with `scripts/generate-sbom.sh target/sbom.json` and compare it with the resolved dependency set.
- [X] T046 [US1] Run `scripts/check-licenses.sh` and record the license result in `specs/005-dependency-upgrade/validation.md`.
- [X] T047 [US1] Confirm zero critical/high advisories and at least 90% overall alert resolution, then record the final security metrics in `specs/005-dependency-upgrade/validation.md`.

**Checkpoint**: The upgraded tree has no unapproved critical/high exposure and every residual finding is visible and time-bound.

---

## Phase 5: User Story 3 - Keep the product behaving and feeling the same (Priority: P1)

**Goal**: Preserve protocol, CLI, MCP, desktop, credential, data, accessibility, and performance behavior after the upgrades.

**Independent Test**: Run the full existing suite plus updated regression tests and compare CLI/MCP outputs, saved-data fixtures, security behavior, and performance against the baseline.

### Tests for User Story 3

- [X] T048 [P] [US3] Update protocol regression tests in `crates/thriftp-protocols/tests/connection_integration.rs`, `crates/thriftp-protocols/tests/transfer_integration.rs`, and `crates/thriftp-protocols/tests/ftp_parse.rs` for the `russh` and `suppaftp` migrations.
- [X] T049 [P] [US3] Update credential-store tests in `crates/thriftp-credentials-keyring/tests/credential_store.rs` and `crates/thriftp-credentials-keyring/tests/native_smoke.rs` for `keyring` 4.
- [X] T050 [P] [US3] Update cryptographic and automation tests in `crates/thriftp-mcp/tests/contract*.rs`, `crates/thriftp-core/tests/automation.rs`, and `crates/thriftp-core/tests/audit_fail_closed.rs` for `ed25519-dalek`, `sha2`, and `base64` changes.
- [X] T051 [P] [US3] Update CLI contract tests in `crates/thriftp-cli/tests/contract.rs`, `crates/thriftp-cli/tests/exit_codes.rs`, `crates/thriftp-cli/tests/snapshot_envelopes.rs`, and `crates/thriftp-cli/tests/streams.rs`.
- [X] T052 [P] [US3] Update MCP contract and security tests in `crates/thriftp-mcp/tests/auth.rs`, `crates/thriftp-mcp/tests/contract_authorization.rs`, `crates/thriftp-mcp/tests/contract_security.rs`, and `crates/thriftp-mcp/tests/plain_ftp_warning.rs`.
- [X] T053 [P] [US3] Update frontend component and tooling tests under `apps/desktop/src/**/*.test.ts` for the Svelte, Vite, Vitest, and jsdom upgrades.

### Implementation for User Story 3

- [X] T054 [US3] Run `cargo test --workspace --locked` and the desktop shell tests, fix every behavioral regression, and record the results in `specs/005-dependency-upgrade/validation.md`.
- [X] T055 [US3] Run the security suites in `crates/thriftp-tests/tests/security/`, fix regressions, and record the result in `specs/005-dependency-upgrade/validation.md`.
- [X] T056 [US3] Run the CLI and MCP contract suites and compare their outputs and exit codes with `specs/005-dependency-upgrade/baseline.md`.
- [X] T057 [US3] Load the saved profiles, server favorites, directory favorites, tabs, preferences, queue state, transfer history, and audit fixtures and verify there is no loss or unintended migration; record the result in `specs/005-dependency-upgrade/validation.md`.
- [ ] T058 [US3] Run the benchmark targets and compare listing, transfer, queue, startup, and artifact-size results with the baseline; any regression above 5% blocks the batch unless it is investigated and explicitly approved in a time-bound exception with evidence, per SC-009 and FR-038; record the comparison and decision in `specs/005-dependency-upgrade/validation.md`.
- [X] T059 [US3] Run the frontend keyboard, theme, focus, dialog, preview, and accessibility smoke checks and record the result in `specs/005-dependency-upgrade/validation.md`.
- [X] T060 [US3] Run manual FTP, explicit FTPS, implicit FTPS, and SFTP connect/list/upload/download/rename/delete smoke tests and record the result in `specs/005-dependency-upgrade/validation.md`.

**Checkpoint**: Existing behavior, security guarantees, interfaces, saved data, accessibility, and performance remain equivalent.

---

## Phase 6: User Story 4 - Restore a clean, reproducible build on every platform (Priority: P1)

**Goal**: A fresh checkout on macOS, Linux, or Windows installs, builds, tests, packages, and passes every required CI check without workarounds.

**Independent Test**: From a clean checkout with no caches, run the documented install/build/test commands on each supported operating system and confirm every required CI job is green.

### Tests and Automation for User Story 4

- [X] T061 [P] [US4] Add or refresh `scripts/verify-clean-install.sh` to run a clean install, build, lint, and test sequence for a fresh checkout without cached dependencies.
- [ ] T062 [US4] Run the clean-install verification on Linux and record the result in `specs/005-dependency-upgrade/validation.md`.
- [X] T063 [US4] Run the clean-install verification on macOS and record the result in `specs/005-dependency-upgrade/validation.md`.
- [ ] T064 [US4] Run the clean-install verification on Windows and record the result in `specs/005-dependency-upgrade/validation.md`.

### Implementation for User Story 4

- [ ] T065 [US4] Fix Linux desktop build and test failures in `.github/workflows/ci.yml` and platform package prerequisites, including WebKitGTK, GTK, OpenSSL, and related packages.
- [ ] T066 [US4] Fix macOS and Windows build, test, or packaging failures caused by the upgraded toolchain and dependencies, updating `.github/workflows/ci.yml` or platform-specific source under `apps/desktop/src-tauri/` as needed and recording the fix in `specs/005-dependency-upgrade/validation.md`.
- [X] T067 [US4] Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked`, and the frontend lint/format/type/build gates; fix all warnings.
- [ ] T068 [US4] Run the full `.github/workflows/ci.yml` pipeline and stabilize every required job without retries that mask failures.
- [X] T069 [US4] Run `scripts/test-integration.sh` with the rebuilt FTP and SFTP containers and record the result in `specs/005-dependency-upgrade/validation.md`.
- [X] T070 [US4] Run `scripts/verify-reproducible-build.sh` and record the result in `specs/005-dependency-upgrade/validation.md`.
- [ ] T071 [US4] Build the desktop installers and bundles for macOS, Linux, and Windows and record the artifact paths and sizes in `specs/005-dependency-upgrade/validation.md`.
- [X] T072 [US4] Verify that no test is skipped, ignored, or deleted to hide an upgrade failure, and record the evidence in `specs/005-dependency-upgrade/validation.md`.

**Checkpoint**: The upgrade builds, tests, packages, and reproduces cleanly on every supported platform.

---

## Phase 7: User Story 5 - Modernize the runtime and platform baseline safely (Priority: P2)

**Goal**: Document and verify the new minimum runtimes and platforms without silently dropping a supported operating system.

**Independent Test**: Review the documented minimum versions, confirm CI uses the same versions, and build and launch the application on every supported operating system.

### Implementation for User Story 5

- [X] T073 [US5] Document the new Rust, Node.js, npm, Tauri, platform minimums, CI runner image labels, and package-manager versions in `README.md`, `docs/desktop.md`, and `docs/testing.md`.
- [X] T074 [P] [US5] Record the macOS, Linux, and Windows support decision in `docs/compatibility.md`, including any platform requirement change and its user impact.
- [X] T075 [US5] Verify that `.github/workflows/ci.yml` uses the explicit documented runner image labels, toolchain versions, and package-manager versions, with no floating `-latest` labels.
- [X] T076 [US5] Record the rebuilt container image digests and base-image versions in `specs/005-dependency-upgrade/validation.md`.
- [X] T077 [US5] Verify the Tauri runtime and plugin compatibility on macOS and record the result in `specs/005-dependency-upgrade/validation.md`.
- [ ] T078 [US5] Verify the Tauri runtime and plugin compatibility on Linux and record the result in `specs/005-dependency-upgrade/validation.md`.
- [ ] T079 [US5] Verify the Tauri runtime and plugin compatibility on Windows and record the result in `specs/005-dependency-upgrade/validation.md`.
- [ ] T080 [US5] Build and launch the desktop bundles on every supported platform and record the result in `specs/005-dependency-upgrade/validation.md`.
- [X] T081 [US5] Re-run the protocol integration suite against the updated containers and record the result in `specs/005-dependency-upgrade/validation.md`.

**Checkpoint**: Runtime and platform support are explicit, documented, and verified on every supported operating system.

---

## Phase 8: User Story 6 - Make the upgrade auditable and repeatable (Priority: P2)

**Goal**: A reviewer can see every version change, every exception, every breaking migration, and the evidence needed to reproduce or roll back the upgrade.

**Independent Test**: Hand the inventory, exception register, validation evidence, and rollback notes to another maintainer and confirm they can reproduce the resolution and verification results or reverse a problematic batch.

### Implementation for User Story 6

- [X] T082 [P] [US6] Complete `specs/005-dependency-upgrade/inventory.json` and `upgrade-inventory.md` so every dependency, runtime, toolchain, package manager, automation action, and container image records previous version, new version, update type, reason, migration guidance, source/provenance, and evidence. Verify that no pre-release or end-of-life component remains, or link each remaining one to an approved exception, and validate the JSON before proceeding.
- [X] T083 [P] [US6] Complete `specs/005-dependency-upgrade/exceptions.json` and `exceptions.md` with every required field, approval, expiry, and review date, then validate the JSON; no partial or expired entry may remain.
- [X] T084 [US6] Complete `specs/005-dependency-upgrade/evidence.json` with every test, audit, benchmark, build, package, license, SBOM, security-review, reproducibility, and platform result, and build the human-readable index in `validation.md`.
- [X] T085 [US6] For each US2 sub-batch, add a separate rollback note to `specs/005-dependency-upgrade/validation.md` with the commands to revert the batch or pin the affected dependency, and verify the rollback is executable; do not record a single combined rollback note for all of US2.
- [X] T086 [US6] Update `docs/testing.md` and `CONTRIBUTING.md` with the repeatable upgrade and verification steps.
- [X] T087 [P] [US6] Create `CHANGELOG.md` with the upgrade scope, required user action, breaking changes, and exception summary.
- [X] T088 [US6] Confirm dependency monitoring remains enabled with no muted alerts and document the evidence in `specs/005-dependency-upgrade/validation.md`.
- [ ] T089 [US6] Re-run the documented upgrade and verification steps from a clean checkout to prove reproducibility, recording the result in `specs/005-dependency-upgrade/validation.md`.
- [X] T090 [US6] Verify the CI dependency-record validation step in `.github/workflows/ci.yml` passes for the completed `inventory.json`, `exceptions.json`, and `evidence.json` with `--release`, and that an intentionally missing or malformed record fails the job.
- [X] T091 [US6] Review every exception for expiry and approval status, close obsolete entries, and record the final register state in `specs/005-dependency-upgrade/validation.md`.

**Checkpoint**: The upgrade is fully documented, reviewable, reproducible, and reversible.

---

## Phase 9: User Story 7 - Reduce dependency debt while upgrading (Priority: P3)

**Goal**: Remove unused, duplicate, and abandoned dependencies where compatible without removing product capability.

**Independent Test**: Compare dependency counts, duplicate versions, installation size, license output, and SBOM before and after; confirm reductions are explained and all capabilities remain.

### Implementation for User Story 7

- [X] T092 [US7] Audit unused direct dependencies with `cargo tree`, `npm ls`, and code reference searches, then remove confirmed-unused entries from `Cargo.toml`, `crates/*/Cargo.toml`, and `apps/desktop/package.json`.
- [X] T093 [US7] Inspect `cargo tree -d` and `npm ls` for duplicate versions, consolidate compatible duplicates, and record unavoidable duplicates, pre-release rejections, and yanked-package decisions with their blocking constraint in `specs/005-dependency-upgrade/inventory.json` and `upgrade-inventory.md`.
- [X] T094 [US7] Replace deprecated or unmaintained dependencies where a maintained equivalent exists, update the manifests, apply the provenance and security review from T027, and record the replacement decision in `specs/005-dependency-upgrade/upgrade-inventory.md` and `specs/005-dependency-upgrade/security-review.md`.
- [X] T095 [US7] Re-run `scripts/check-licenses.sh` after cleanup and record the result in `specs/005-dependency-upgrade/validation.md`.
- [X] T096 [US7] Compare dependency counts, installation size, and artifact sizes with `specs/005-dependency-upgrade/baseline.md`; record any reduction and justify any increase above the US7 acceptance thresholds (more than 5% installation size or more than 10% dependency count).
- [X] T097 [US7] Re-run the full test suite after dependency removal, confirm no user-facing capability was lost, and record the result in `specs/005-dependency-upgrade/validation.md`.
- [X] T098 [US7] Update the status and evidence fields for removed, replaced, and consolidated entries in `specs/005-dependency-upgrade/upgrade-inventory.md`.

**Checkpoint**: Dependency debt is reduced without losing capability or reintroducing advisories.

---

## Phase 10: Polish & Cross-Cutting Concerns

**Purpose**: Final documentation, security sign-off, and release readiness.

- [X] T099 Update `README.md` status and highlights to reflect the upgraded baseline.
- [X] T100 [P] Review `docs/architecture.md` and `docs/desktop.md` for drift caused by the upgrade and update only the changed guidance.
- [X] T101 [P] Add the dependency policy, exception summary, and minimum-version guidance to `docs/security.md` and `docs/release.md`.
- [ ] T102 Run the full `specs/005-dependency-upgrade/quickstart.md` validation flow and record the outcome in `specs/005-dependency-upgrade/validation.md`.
- [ ] T103 Run the final `.github/workflows/ci.yml` matrix on all supported platforms and confirm every required job is green.
- [X] T104 [P] Obtain security review sign-off for the credential, SSH, cryptography, and transport upgrades and record it in `specs/005-dependency-upgrade/security-review.md`.
- [X] T105 Audit `specs/005-dependency-upgrade/inventory.json`, `exceptions.json`, `evidence.json`, and their Markdown narratives for missing fields, stale entries, contradictions, and unsupported claims.
- [X] T106 Clean up temporary files, generated audit artifacts, and local caches. Scan dependency manifests, resolution records, the generated SBOM, audit output, logs, error messages, the working tree, `specs/005-dependency-upgrade/inventory.json`, `exceptions.json`, `evidence.json`, and `validation.md` for secrets, credentials, tokens, or private keys, and record the clean result.
- [X] T107 Verify `specs/005-dependency-upgrade/checklists/requirements.md` remains satisfied and that no requirement or success criterion is contradicted.
- [X] T108 Perform the final release-readiness review, update `specs/005-dependency-upgrade/plan.md` and `validation.md` with the final status, and confirm no batch is marked release-ready while a required check is red.
- [X] T109 [P] Audit every changed or new dependency in `specs/005-dependency-upgrade/inventory.json` for mandatory telemetry, account creation, cloud calls, new network endpoints, or plaintext secret paths; record the result in `specs/005-dependency-upgrade/evidence.json` and summarize it in `validation.md`, blocking release if any required dependency adds one without an approved exception.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies. Creates the evidence and validator scaffolding.
- **Foundational (Phase 2)**: Depends on Setup. Captures the baseline and modernizes the toolchain, CI, and containers. Blocks every user story.
- **US2 (Phase 3)**: Depends on Foundational. Produces the target dependency versions.
- **US1 (Phase 4)**: Depends on US2 for final advisory remediation and verification. Its security review can begin once the target versions are known.
- **US3 (Phase 5)**: Depends on US2 and benefits from US1's security review.
- **US4 (Phase 6)**: Depends on US2, US1, and US3. The build can be stabilized only after behavior and security are verified.
- **US5 (Phase 7)**: Depends on Foundational and US4 for platform builds. It can start in parallel with US3 for documentation and runtime checks.
- **US6 (Phase 8)**: Depends on US1, US2, US4, and US5 for final evidence.
- **US7 (Phase 9)**: Depends on US2 and US3; can run in parallel with US5/US6 where files do not conflict.
- **Polish (Phase 10)**: Depends on all desired stories.

### User Story Dependencies

- **US2 (P1)**: Foundation for all dependency changes. No story dependency.
- **US1 (P1)**: Depends on US2 for the versions that resolve advisories.
- **US3 (P1)**: Depends on US2; consumes US1 security review.
- **US4 (P1)**: Depends on US2, US1, and US3.
- **US5 (P2)**: Depends on Foundational and US4; can be prepared alongside US3.
- **US6 (P2)**: Depends on US1, US2, US4, and US5.
- **US7 (P3)**: Depends on US2 and US3; must not delay the security baseline.

### Within Each User Story

- Write or update the regression test before declaring the related migration complete.
- Upgrade manifests and resolution records before migrating call sites.
- Complete Rust migration and tests before moving to frontend tooling.
- Run the full suite and record evidence before marking a story complete.

### Parallel Opportunities

- Setup: T002 and T003 can run together; T004 follows the validator implementation.
- Foundational: T005–T009 share `specs/005-dependency-upgrade/baseline.md` and must be sequenced; T010–T014 touch different files and can run together.
- US2: T024 can run after T011 completes and alongside the sequenced Rust manifest upgrades; the Rust batches T019–T023 share `Cargo.toml` and must be sequenced.
- US1: T036 and T037 are independent once the validator exists; T045 can run independently after remediation; T044 and T046 serialize their `specs/005-dependency-upgrade/validation.md` updates.
- US3: T048–T053 are independent test-file updates; T059 and T060 can run on separate machines if their `specs/005-dependency-upgrade/validation.md` updates are serialized.
- US4: T062–T064 can run on separate machines, but serialize their `specs/005-dependency-upgrade/validation.md` updates or capture separate evidence fragments.
- US5: T073 and T074 are independent docs; T077–T079 can run on separate machines, but serialize their `specs/005-dependency-upgrade/validation.md` updates.
- US6: T082, T083, and T087 are independent record/document files; T086 follows T011's `docs/testing.md` edit and can then run; T084 and T088 share `specs/005-dependency-upgrade/validation.md` and must be sequenced.
- US7: T095 and T096 both write `specs/005-dependency-upgrade/validation.md`; run them in sequence.
- Polish: T100 and T101 are independent docs; T099 follows T073's README update; T104 and T109 are independent; T105 follows the validation work.

---

## Parallel Example: User Story 2

```bash
# The npm manifest can move while the Rust manifests are sequenced:
Task: "Upgrade all npm dependencies and devDependencies in apps/desktop/package.json"
Task: "Upgrade utility Rust crates in Cargo.toml"
```

## Parallel Example: User Story 3

```bash
# Independent regression suites can be updated together:
Task: "Update protocol regression tests in crates/thriftp-protocols/tests/"
Task: "Update credential-store tests in crates/thriftp-credentials-keyring/tests/"
Task: "Update CLI contract tests in crates/thriftp-cli/tests/"
Task: "Update frontend tests under apps/desktop/src/**/*.test.ts"
```

## Parallel Example: User Story 4

```bash
# Clean-install validation can run on each platform independently:
Task: "Run clean-install verification on Linux"
Task: "Run clean-install verification on macOS"
Task: "Run clean-install verification on Windows"
```

## Parallel Example: User Story 5

```bash
# Platform-specific runtime checks are independent:
Task: "Verify Tauri runtime compatibility on macOS"
Task: "Verify Tauri runtime compatibility on Linux"
Task: "Verify Tauri runtime compatibility on Windows"
```

---

## Implementation Strategy

### MVP First (Security-Clean Latest Stable Baseline)

The smallest independently valuable increment is **Foundational + US2 + US1**: the toolchain and CI are current, all direct dependencies are on the latest stable baseline, critical/high advisories are resolved, and at least 90% of the initial alerts are gone. This is the security-driven MVP.

1. Complete Phase 1: Setup.
2. Complete Phase 2: Foundational.
3. Complete US2 (latest stable upgrade).
4. Complete US1 (security remediation).
5. **STOP and VALIDATE**: run the advisory checks, full test suite, and baseline comparison.
6. Deliver the security-clean baseline if the remaining stories must be sequenced later.

### Incremental Delivery

1. Setup + Foundational → baseline and modern toolchain.
2. US2 → latest stable dependency baseline.
3. US1 → security-clean alert reduction.
4. US3 → behavior and security preservation.
5. US4 → green cross-platform build and packaging.
6. US5 → documented runtime and platform baseline.
7. US6 → audit-ready evidence and rollback.
8. US7 → dependency-debt reduction.
9. Polish → final release readiness.

### Parallel Team Strategy

1. One maintainer owns the baseline and toolchain/CI batch.
2. One maintainer owns Rust manifest and API migrations; another owns the npm manifest and frontend tooling once the manifests are separated.
3. Security review runs beside the migration after the target versions are selected.
4. Platform verification runs in parallel across macOS, Linux, and Windows.
5. Evidence and documentation are completed continuously, not left to the end.

---

## Notes

- `[P]` tasks touch different files and have no incomplete-task dependency.
- The TypeScript 7 deferral is the only planned latest-stable exception; any additional exception must be approved and time-bound.
- Do not mark a batch complete while a required check is red or an exception lacks approval.
- Commit after each batch or coherent task group so a failing batch can be reverted cleanly.
- Stop at each checkpoint to validate the story independently before proceeding.

---

## Phase 11: Convergence

- [X] T110 Generate a complete dependency inventory from `cargo metadata --locked`, `apps/desktop/src-tauri/Cargo.lock`, `apps/desktop/package-lock.json`, CI action pins, and container manifests; populate `specs/005-dependency-upgrade/inventory.json` for every direct and transitive dependency with the required fields and evidence, then validate with `scripts/validate-dependency-records.py --release` per FR-001 and SC-012 (partial).
- [X] T111 Replace the fallback SBOM with a real CycloneDX SBOM: install and run `cargo-cyclonedx` in CI, generate and validate `target/sbom.json` against the resolved Rust dependency set, add npm component coverage or document the tool used, and record the result in `specs/005-dependency-upgrade/evidence.json` per FR-016 and SC-011 (partial).
- [X] T112 Install and run `cargo-license` in CI, produce and record the allowed-license inventory for the upgraded dependency set, fail on disallowed licenses, and update `specs/005-dependency-upgrade/evidence.json` and `validation.md` per FR-017 and SC-011 (partial).
- [X] T113 Audit duplicate versions in `Cargo.lock` and `apps/desktop/src-tauri/Cargo.lock` (for example `sha2` 0.10/0.11, `hashbrown`, `syn`, and `windows-sys`), consolidate compatible duplicates, and document each unavoidable duplicate with its blocking constraint in `specs/005-dependency-upgrade/inventory.json` and `validation.md` per FR-004 and US7/AC2 (partial).
- [ ] T114 Capture the final Dependabot/advisory dashboard counts and monitoring settings, prove at least 90% of the initial alert set is resolved, confirm no alerts are muted or snoozed, and record the evidence in `specs/005-dependency-upgrade/evidence.json` and `validation.md` per FR-012, FR-044, SC-002, and SC-016 (partial).

