# Validation Evidence

The authoritative machine-readable evidence is [evidence.json](./evidence.json).

## Local Verification

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `cargo test --workspace --locked` | PASS |
| `cargo doc --workspace --no-deps --locked` | PASS |
| `python3 scripts/validate-dependency-records.py ... --release` | PASS |
| `python3 -m unittest discover -s scripts/tests` | PASS, 13 tests |
| `npm run lint` (desktop) | PASS |
| `npm test` (desktop) | PASS, 29 files / 110 tests |
| `npm run build` (desktop) | PASS |
| `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml` | PASS |

## Dependency Audit

- `npm audit --audit-level=high` initially reported 16 package advisories. After the upgrade and the approved minimatch override, 2 low-severity `deepmerge` instances remain and are covered by `EX-002`.
- No critical, high, or moderate advisory remains in the local npm graph.
- `cargo audit` reports one medium RSA timing advisory (RUSTSEC-2023-0071, `rsa 0.10.0-rc.18`) and three unmaintained zbus warnings. They are triaged by `EX-004` and `EX-006`; no critical or high advisory remains.
- `scripts/check-dependency-policy.sh` passes with the approved `RUSTSEC-2023-0071` exception.

## Container and Reproducibility Evidence

- FTP image: `protocol-ftp` digest `sha256:4e9adaca8d048642a2a3bb3f111ed31b8beef8b30c6b7f0fe4fe83fddd14fbe5`.
- SFTP image: `protocol-sftp` digest `sha256:771611466841753511d7975634d6b7e6622fd6e8ced4e6f58ab38bc63c47e861`.
- `scripts/test-integration.sh` passed for FTP, explicit FTPS, implicit FTPS, and SFTP.
- `scripts/verify-reproducible-build.sh` passes for final release binaries and the generated SBOM.
- `scripts/verify-clean-install.sh` passed on macOS.

## Compatibility

- Existing CLI contract, MCP contract, security, host, storage, server-favorites, and transfer tests passed without skipped or removed tests.
- Saved-data fixtures and schema migration tests passed.
- No new telemetry, account, cloud dependency, or plaintext secret path was introduced.

## Platform Notes

Cross-platform CI workflows are defined for Ubuntu 24.04, macOS 15, and Windows 2025 with explicit runner labels. Local validation in this environment covered macOS. Linux and Windows runs remain CI-gated.

## Rollback

Each batch is revertible as a unit:

- Restore `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, and `apps/desktop/src-tauri/Cargo.toml` for Rust/toolchain changes.
- Restore `apps/desktop/package.json` and `apps/desktop/package-lock.json` for frontend changes.
- Restore `.github/workflows/ci.yml` and `docker/*/Dockerfile` for CI/container changes.
- Re-run the relevant tests and `scripts/verify-reproducible-build.sh` after a rollback.

## Dependency Cleanup and Size

- `cargo tree --workspace --depth 1` and `npm ls --depth=0` were reviewed; no unused direct dependency was identified and removed.
- The regenerated npm lockfile contains 249 package entries. No material installation-size increase was identified; the frontend build output remains approximately 184 kB JS and 26 kB CSS uncompressed.
- `scripts/check-licenses.sh` was run; `cargo-license` is not installed locally, so the detailed license inventory remains CI-gated. The script completed its dependency-metadata fallback successfully.

## Remaining CI-Gated Tasks

The following tasks cannot be completed in this local macOS environment and remain for CI:

- Linux and Windows clean-install verification (`T062`, `T064`).
- Platform-specific build/test fixes (`T065`, `T066`) because those jobs have not run locally.
- Full CI matrix stabilization (`T068`, `T103`).
- Desktop installers and bundles for all platforms (`T071`, `T080`).
- Linux and Windows Tauri compatibility checks (`T078`, `T079`).
- Full upgrade replay from a fresh checkout (`T089`).
- Manual cross-platform quickstart validation (`T102`).
- Benchmark baseline comparison (`T058`); benchmark targets compile, but a pre-upgrade numeric baseline was not captured in this run.

No local failure is being masked; these are environment limitations, not waived checks.

## Final Status

100 of 114 tasks are complete locally. The remaining 14 tasks are CI/platform/merge-gated: T058 (numeric benchmark comparison), T062/T064 (Linux/Windows clean installs), T065/T066/T068/T078/T079 (platform CI), T071/T080 (installers/bundles), T089 (fresh-checkout replay), T102 (full cross-platform quickstart), T103 (final CI matrix), and T114 (post-merge Dependabot dashboard proof).

Local validation passed for Rust workspace tests, desktop tests, frontend tests/build/lint/format, Clippy, rustfmt, docs, dependency-record validation, dependency-policy checks, containerized protocol integration, clean install on macOS, and reproducible final binaries/SBOM.

Residual advisories are documented: low-severity `deepmerge` (EX-002), medium RSA timing (EX-004), pre-release transitive `ssh-key` (EX-005), and unmaintained zbus stack (EX-006). No critical or high advisory remains locally.

## Complete Inventory, SBOM, and License Evidence

- `inventory.json` now records 884 items: 55 curated direct/action/container/toolchain entries plus 829 transitive entries generated from `Cargo.lock`, `apps/desktop/src-tauri/Cargo.lock`, and `apps/desktop/package-lock.json`.
- `target/sbom.json` is a CycloneDX 1.5 SBOM merged from per-crate `cargo-cyclonedx` output (222 components); `target/sbom-npm.json` is a CycloneDX 1.5 npm SBOM (214 components).
- `scripts/check-licenses.sh` now runs `cargo-license` in JSON mode and checks 302 runtime packages. All licenses are permissive or weak-copyleft compatible with the project policy.

## Duplicate Versions

The workspace lockfile has three duplicate names and the desktop lockfile has 29. They are transitive, major-version-separated dependencies of Tauri, cryptographic, and proc-macro families, so consolidating them would break peer constraints:

- Workspace: `hashbrown` 0.14.5/0.17.1, `num-bigint` 0.4.8/0.5.1, `syn` 2.0.119/3.0.9.
- Desktop: `sha2` 0.10.9/0.11.1, `base64` 0.21.7/0.22.1/0.23.1, `syn` 1.0.109/2.0.119/3.0.9, `hashbrown` 0.12.3/0.14.5/0.17.1, plus the Tauri/Windows proc-macro and encoding families.

Each resolved version is present in `inventory.json`; no avoidable duplicate was identified.

## Advisory Dashboard Snapshot

The repository has vulnerability alerts enabled and automated security fixes enabled (`paused: false`). The default branch currently reports 65 open Dependabot alerts (3 critical, 12 high, 39 medium, 11 low) because the working-tree upgrade is not committed or merged. The local upgraded tree has no critical/high findings, but the >=90% dashboard reduction cannot be proven until the change lands on the default branch. This is tracked as `T114`.
