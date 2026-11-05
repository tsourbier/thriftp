# Quickstart: Dependency and Toolchain Upgrade

**Feature**: `005-dependency-upgrade` | **Date**: 2026-11-05

This guide validates the upgrade end to end. It records what to capture before changing versions, how to verify each batch, and what evidence proves the final baseline is acceptable. It does not contain implementation code or the full task breakdown; those belong in `tasks.md` and the implementation work.

## Prerequisites

- Rust 1.99.1 stable with `rustfmt` and `clippy` (the pinned toolchain in `rust-toolchain.toml`).
- Node.js 26.13.0 LTS and its bundled npm.
- Platform prerequisites for the desktop shell (the same ones documented in the contributor guide).
- A Docker-compatible runtime only for the optional protocol integration suite.
- Network access to official package sources.
- Clean working tree or a dedicated upgrade branch.

Record the starting state:

```bash
git status --short
git rev-parse HEAD
rustc --version
cargo --version
node --version
npm --version
```

Expected: a clean tree, the starting commit, and the toolchain versions used for the baseline.

## 1. Capture the baseline before changing anything (Batch 1)

Run the existing project checks and save the results. Use `--locked` where the repository already does so the baseline reflects the committed resolution records.

```bash
cargo test --workspace --locked
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked
cargo build --workspace --release --locked
```

```bash
cd apps/desktop
npm ci
npm run lint
npm run format
npm test
npm run build
cd ../..
```

```bash
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked
```

Record the advisory and compliance baseline:

```bash
npm --prefix apps/desktop audit
cargo install cargo-audit --locked
cargo audit
scripts/check-licenses.sh
scripts/generate-sbom.sh target/baseline-sbom.json
scripts/verify-reproducible-build.sh
```

Record benchmark and startup measurements with the project's existing benchmark targets. Use the same machine class and command form for the post-upgrade comparison. Capture artifact sizes for the desktop bundle and installer.

Expected baseline evidence:

- Every existing suite passes.
- The current advisory counts are recorded by severity and ecosystem.
- The current SBOM, license report, benchmark values, startup timings, and artifact sizes are saved.
- The current CLI/MCP contract outputs and saved-data fixtures are recorded.
- `cargo audit` and `npm audit` output is stored as baseline evidence.

Store the baseline using the fields in `data-model.md` and the `verification-evidence.schema.json` contract.

## 2. Apply and verify Batch 2: toolchain, CI, and containers

Upgrade the Rust toolchain, Node.js version, CI action versions, and container base images. Then verify the pipeline before changing application dependencies.

```bash
cargo --version
node --version
npm --version
scripts/test-integration.sh
```

Expected:

- Rust reports 1.99.1 or the selected latest stable version.
- Node reports 26.13.0 LTS or the selected latest stable LTS.
- `actions/checkout`, `actions/setup-node`, and `actions/upload-artifact` use the selected latest stable majors.
- The FTP image builds on Alpine 3.23 and the SFTP image on Debian 13 slim.
- Containerized protocol tests pass, or report the documented skip when no container runtime is available.

## 3. Apply and verify Batch 3: Rust dependency majors

Upgrade the manifests in compatibility groups, regenerate the workspace resolution record with the official tooling, and verify after each group.

```bash
cargo update
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --locked
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked
cargo audit
```

Expected:

- `Cargo.lock` and the desktop lockfile resolve deterministically.
- `russh`, `russh-sftp`, `ed25519-dalek`, `rand_core`, `sha2`, `keyring`, `base64`, and `criterion` are on the selected stable versions.
- No compile errors from trait, digest, credential-store, or benchmark API changes.
- Security tests, protocol tests, and credential tests pass.
- No critical or high advisory remains in the Rust graph.

## 4. Apply and verify Batch 4: frontend and tooling majors

Upgrade the UI toolchain, regenerate `package-lock.json`, and verify the frontend and desktop shell.

```bash
cd apps/desktop
npm install
npm ci
npm run lint
npm run format
npm test
npm run build
npm ls typescript typescript-eslint vite vitest
cd ../..
```

Expected:

- Vite 8, Vitest 5, ESLint 10, `@sveltejs/vite-plugin-svelte` 7, `eslint-plugin-svelte` 3, and jsdom 31 resolve without force flags.
- TypeScript resolves to 6.0.3, the latest version supported by `typescript-eslint` 8.72.1.
- `npm ls` shows no invalid peer dependency and no `--legacy-peer-deps` or force-resolution requirement.
- Lint, format, tests, and production build pass.
- The TypeScript 7 deferral is recorded as an exception with an expiry date.

## 5. Apply and verify Batch 5: Tauri alignment

Align the Rust and JavaScript Tauri packages on the current stable Tauri 2 line.

```bash
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked
cd apps/desktop
npm run build
npm test
cd ../..
```

Expected:

- `tauri`, `tauri-build`, `tauri-plugin-dialog`, `@tauri-apps/api`, and `@tauri-apps/cli` are on the selected stable Tauri 2 versions.
- The desktop shell builds and its tests pass.
- No Tauri 3 pre-release is present.

## 6. Run the full verification suite (Batch 6)

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --locked
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked
cargo build --workspace --release --locked
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked
```

```bash
cd apps/desktop
npm ci
npm run lint
npm run format
npm test
npm run build
npm audit
cd ../..
```

```bash
cargo audit
scripts/check-licenses.sh
scripts/generate-sbom.sh target/sbom.json
scripts/verify-reproducible-build.sh
scripts/test-integration.sh
```

Run the benchmark suite and compare every result with the baseline. Re-run the startup measurement and artifact-size comparison.

Expected final evidence:

- Zero open critical or high advisories.
- At least 90% of the initial alert set resolved.
- All existing tests pass with no skipped or deleted coverage.
- Formatting, linting, documentation, and warning-denied checks pass.
- Reproducible-build verification passes.
- SBOM and license checks pass.
- Containerized protocol tests pass where a runtime is available.
- Performance is within 5% of baseline.
- No secrets appear in output, logs, SBOM, or generated artifacts.

## 7. Verify compatibility contracts

Run the existing contract and security suites directly:

```bash
cargo test -p thriftp-cli --tests --locked
cargo test -p thriftp-mcp --tests --locked
cargo test -p thriftp-core --test automation --test audit_fail_closed --test approval_confirmations --locked
cargo test -p thriftp-storage-json --test audit_retention --locked
cargo test -p thriftp-tests --test security_secret_redaction --test security_security_parity --test security_panic_safety --locked
cargo test -p thriftp-core --test server_favorites --test server_favorites_create --test server_favorites_organize --test server_favorites_connect --test server_favorites_credentials --test server_favorites_edit --test server_favorites_performance --locked
```

Load the saved-data fixtures for profiles, favorites, tabs, preferences, queue state, transfer history, and audit records. Confirm no loss, no unintended migration, and no change to CLI/MCP outputs or exit codes.

Expected: every reference contract in [`contracts/compatibility-contract.md`](./contracts/compatibility-contract.md) remains satisfied.

## 8. Verify the exception register

```bash
rg "EX-[0-9]{3,}" specs/005-dependency-upgrade/exceptions.md
```

Expected:

- Every exception has a reason, risk, mitigation, owner, approver, creation date, expiry date, and review date.
- The TypeScript 7 deferral names the `typescript-eslint` peer constraint.
- No expired exception is treated as active.
- No alert is muted, snoozed indefinitely, or excluded to hide a finding.

## 9. Verify cross-platform CI

Push the upgrade branch and confirm the full matrix:

- Linux unit, frontend, contract, security, multi-process, protocol integration, audit, SBOM, build, and reproducible jobs.
- macOS and Windows desktop, workspace, and frontend jobs.
- Packaging and installer jobs for every supported platform.

Expected: every required job passes on a clean run, with no retries used to mask a failure. A platform-specific failure blocks completion until fixed.

## 10. Rollback validation

For each batch, confirm the rollback note is executable:

```bash
git revert <batch-commit>
```

or, for a single dependency:

```bash
cargo update -p <crate> --precise <previous-version>
npm --prefix apps/desktop install <package>@<previous-version>
```

After a simulated rollback, the previous lock records, tests, and build must still pass. Restore the upgraded state before continuing.

## Final acceptance checklist

- [ ] Baseline captured before the first change.
- [ ] Direct dependencies are on latest stable, or have an approved exception.
- [ ] Zero critical/high advisories; at least 90% of initial alerts resolved.
- [ ] TypeScript 7 exception documented and time-bound.
- [ ] Lock/resolution records regenerated and committed.
- [ ] All existing tests pass with no skipped or weakened coverage.
- [ ] Formatting, linting, documentation, and warning-denied checks pass.
- [ ] Performance within 5% of baseline.
- [ ] Reproducible build passes.
- [ ] SBOM and license checks pass.
- [ ] Protocol integration tests pass.
- [ ] CLI, MCP, desktop IPC, credential, transfer, and favorites contracts remain compatible.
- [ ] macOS, Linux, and Windows CI jobs pass.
- [ ] Installers and bundles build and launch.
- [ ] Inventory, exceptions, and verification evidence are complete.
- [ ] No secrets in manifests, lock records, logs, SBOM, or artifacts.
