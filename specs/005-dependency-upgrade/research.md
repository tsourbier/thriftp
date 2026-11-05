# Research: Dependency and Toolchain Upgrade

**Feature**: `005-dependency-upgrade` | **Date**: 2026-11-05

This research resolves the technical unknowns in the plan. Version targets were checked against the official registries and release feeds on 2026-11-05. Versions must be re-checked at implementation time; the policy in the spec is "latest stable release available at implementation time", so newer stable releases may replace these targets.

## R1. Toolchain baseline and target versions

**Decision**: Pin Rust to 1.99.1 stable, Node.js to 26.13.0 LTS, and npm to the version bundled with that Node LTS. Set the effective Rust MSRV to 1.90.0, the highest requirement among the upgraded crates.

**Rationale**: Rust 1.99.1 is the current stable release and is installed in the development environment. Tauri 2.12.5 requires Rust 1.90; `russh` 0.65.1 requires Rust 1.89; `keyring` 4.2.0 and `sha2` 0.11.1 require Rust 1.88. Node 26.13.0 is the current stable LTS and satisfies Vite 8, Vitest 5, and jsdom 31 engine requirements. Pinning the toolchain keeps local, CI, and release builds identical. The workspace MSRV is 1.90.0 because the upgraded protocol stack requires it; this feature does not claim a separate lower core MSRV.

**Alternatives considered**:
- Keep Rust 1.88.0: rejected because Tauri 2.12.5 requires 1.90 and the project would be unable to build the latest stable desktop stack.
- Use Node 22 LTS: rejected because the spec calls for latest stable and Node 26 is the current LTS; Node 22 remains supported by the upgraded tooling but is no longer the latest LTS.
- Use a pre-release Node 27 build: rejected because the spec excludes pre-release runtimes.

## R2. Rust dependency target matrix

**Decision**: Upgrade the workspace to the following stable versions, re-checking each at implementation time:

| Component | Current | Target | Notes |
|---|---|---|---|
| `tauri` | 2.x | 2.12.5 | Tauri 3 is alpha; excluded by the stable-only rule |
| `tauri-build` | 2.x | 2.7.4 | Must stay aligned with `tauri` 2.12.5 |
| `tauri-plugin-dialog` | 2.x | 2.8.4 | Matches `@tauri-apps/plugin-dialog` 2.8.4 |
| `russh` | 0.54 | 0.65.1 | Major SSH API migration; requires Rust 1.89 |
| `russh-sftp` | 3.x | 3.0.1 | Current stable; verify compatibility with `russh` 0.65.1 |
| `ed25519-dalek` | 2.x | 3.0.0 | Major crypto API migration; uses `rand_core` 0.10 and `sha2` 0.11 |
| `rand_core` | 0.6 | 0.10.1 | Required by `ed25519-dalek` 3 and `russh` 0.65 |
| `sha2` | 0.10 | 0.11.1 | Required by `ed25519-dalek` 3 and `russh` 0.65 |
| `keyring` | 3.x | 4.2.0 | Major credential-store API migration |
| `base64` | 0.22 | 0.23.1 | API changes in encoding/decoding helpers |
| `criterion` | 0.5 | 0.8.2 | Benchmark API migration |
| `suppaftp` | 12.x | 12.2.1 | Same major; verify FTPS and TLS features |
| `native-tls` | 0.2 | 0.2.18 | Same major |
| `async-trait` | 0.1 | 0.1.92 | Same major |
| `chrono` | 0.4 | 0.4.45 | Same major |
| `clap` | 4.5 | 4.6.7 | Same major |
| `serde` | 1.x | 1.0.229 | Same major |
| `serde_json` | 1.x | 1.0.154 | Same major |
| `tokio` | 1.x | 1.54.1 | Same major |
| `trash` | 5.x | 5.3.0 | Same major |
| `tracing` | 0.1 | 0.1.44 | Same major |
| `tracing-subscriber` | 0.3 | 0.3.23 | Same major |
| `uuid` | 1.x | 1.29.0 | Same major |
| `zeroize` | 1.x | 1.9.1 | Same major |
| `anyhow` | 1.x | 1.0.104 | Same major |
| `tempfile` | 3.x | 3.27.0 | Same major |
| `thiserror` | 2.x | 2.0.21 | Same major |

**Rationale**: The largest risk is the clustered SSH/cryptography upgrade. `russh` 0.65.1, `ed25519-dalek` 3.0.0, `sha2` 0.11.1, and `rand_core` 0.10.1 must move together because their trait and digest versions are coupled. `keyring` 4.2.0 is a separate security-sensitive migration. Upgrade general utilities first, then the coupled SSH/cryptography group, then the credential-store migration, then benchmarks; this matches the task order and keeps failures attributable.

**Alternatives considered**:
- Upgrade every crate in one manifest change: rejected because a compile failure would be difficult to attribute across SSH, crypto, credential, and utility boundaries.
- Keep `russh` 0.54 and only update patch versions: rejected because it leaves a large version gap and does not satisfy the latest-stable requirement.
- Move to Tauri 3 alpha: rejected because the spec excludes pre-release components.

## R3. Frontend target matrix and the TypeScript exception

**Decision**: Upgrade to Vite 8.3.6, Vitest 5.1.0, `@sveltejs/vite-plugin-svelte` 7.3.1, ESLint 10.14.0, `eslint-plugin-svelte` 3.24.0, jsdom 31.0.0, Prettier 3.9.10, `prettier-plugin-svelte` 4.1.1, `@types/node` 26.6.6, `@testing-library/svelte` 5.4.2, and `typescript-eslint` 8.72.1. Use TypeScript 6.0.3, not 7.0.2, and record a time-bound exception.

**Rationale**: `typescript-eslint` 8.72.1 declares `typescript >=4.8.4 <6.1.0`. TypeScript 7.0.2 is the latest stable TypeScript, but using it would violate the peer contract and likely break lint/type-aware rules. TypeScript 6.0.3 is the latest stable release inside the supported range. The rest of the toolchain is mutually compatible: Vite 8 requires Node 20.19+ or 22.12+; Vitest 5 supports Vite 6, 7, or 8; `@sveltejs/vite-plugin-svelte` 7 requires Vite 8 and Svelte 5.46.4+; ESLint 10 is supported by `eslint-plugin-svelte` 3 and `typescript-eslint` 8; jsdom 31 supports Node 26.

**Alternatives considered**:
- Use TypeScript 7 and downgrade or drop `typescript-eslint`: rejected because it would reduce lint coverage and violate the no-weakening rule.
- Use `--legacy-peer-deps` or force resolution: rejected by the spec.
- Stay on the current Vite 6/Vitest 2 toolchain: rejected because it does not satisfy the latest-stable requirement and leaves known tooling debt.

## R4. Tauri runtime and frontend alignment

**Decision**: Use Tauri 2.12.5 with `tauri-build` 2.7.4, `tauri-plugin-dialog` 2.8.4, `@tauri-apps/api` 2.12.4, and `@tauri-apps/cli` 2.12.4. Keep the Tauri 2 capability and permission model unchanged.

**Rationale**: Tauri 2.12.5 is the latest stable release; Tauri 3 is still alpha. The selected plugin and JavaScript package versions are the current stable releases and remain on the Tauri 2 line. Aligning them avoids a mixed-version desktop shell.

**Alternatives considered**:
- Upgrade only the Rust Tauri crates and leave the JavaScript API behind: rejected because mixed versions can break IPC types and CLI behavior.
- Move to Tauri 3 alpha: rejected by the stable-only rule.

## R5. CI actions, runners, and test containers

**Decision**: Update GitHub Actions to immutable commit pins: `actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1` (v7.0.1), `actions/setup-node@949feb2413d6458794dcd2491c4babbbce0c15c1` (v7.1.0), `actions/upload-artifact@cf430e030ddbb5b0abf93d22962f4752f3646cd9` (v7.0.2), `Swatinem/rust-cache@2201cc584a476d1b508be977bbe4b7d1c7a52e9c` (v2.9.3), and `dtolnay/rust-toolchain@d124f86f28ff42855784ff3319f48ac182c248f8` (1.99.1). Resolve and record the current supported GitHub-hosted runner image labels for macOS, Linux, and Windows at implementation time; use explicit labels rather than floating `-latest` tags. Rebuild the FTP image on `alpine:3.23` and the SFTP image on `debian:13-slim` (trixie).

**Rationale**: These are the current stable releases, pinned to verified commit SHAs, and supported base images. Pinning after selecting the latest stable release satisfies the constitution's supply-chain requirement and prevents silent drift. The container upgrade is part of the dependency surface because protocol integration tests depend on those images.

**Alternatives considered**:
- Leave actions on `v4`: rejected because the upgrade scope includes automation dependencies.
- Use `alpine:latest` or `debian:latest`: rejected because floating tags make reproducible tests and rollback harder.
- Keep Debian 12 (bookworm): rejected because Debian 13 is the current stable release.

## R6. Resolution and lockfile strategy

**Decision**: Regenerate `Cargo.lock`, `apps/desktop/src-tauri/Cargo.lock`, and `apps/desktop/package-lock.json` with the official tooling after each batch. Commit the regenerated records with the manifest changes. Do not hand-edit resolved versions, use force-resolution flags, or skip install scripts.

**Rationale**: The project ships three resolution records and CI enforces locked installs. Regenerating per batch keeps failures attributable and preserves deterministic installs. The spec prohibits workarounds that hide incompatible peer graphs.

**Alternatives considered**:
- Delete all lockfiles and resolve once at the end: rejected because it would make regressions difficult to bisect and could silently change unrelated transitive versions.
- Hand-pin transitive versions in manifests: rejected except for an approved, documented resolution override required to remove an advisory.

## R7. Security verification and exception handling

**Decision**: Use the repository's dependency audit checks, the platform advisory dashboard, the generated SBOM, and the license script as the authoritative evidence. Run `cargo audit` and `npm audit` at baseline and after each batch. Require zero critical/high findings and resolve at least 90% of the initial alert set. Any remaining finding must be recorded in `exceptions.json` using the exception schema and mirrored in `exceptions.md`. Changed dependencies must also be reviewed for new telemetry, account, cloud, or network behavior, and for any new plaintext secret path; any such change requires an approved exception under FR-025.

**Rationale**: The starting `npm audit` baseline already shows 16 vulnerabilities (2 critical, 1 high, 10 moderate, 3 low) in the desktop dependency graph, independently confirming the need for the upgrade. The full Dependabot total also covers Rust and automation dependencies, so both ecosystems must be measured. Time-bound exceptions keep residual risk visible.

**Alternatives considered**:
- Ignore low and moderate findings: rejected because the spec requires at least 90% overall resolution and explicit triage for every remainder.
- Suppress alerts in configuration: rejected by the spec.
- Rely only on Dependabot: rejected because local audits and the SBOM provide reproducible evidence.

## R8. Behavior, performance, and compatibility baseline

**Decision**: Capture a baseline before any upgrade: full test results, benchmark results, startup timing, artifact sizes, CLI/MCP contract outputs, security test results, and the current saved-data fixtures. Re-run the same measurements after each batch and compare against a 5% performance tolerance.

**Rationale**: The spec requires no functional or security regression. A measured baseline makes "no regression" testable and prevents a broad upgrade from hiding a slow transfer path or a changed CLI output. The existing test suites already cover protocols, automation security, host behavior, and server favorites.

**Alternatives considered**:
- Rely on tests alone: rejected because tests may not catch performance or artifact-size changes.
- Compare against a previous release artifact: rejected because the working tree may already differ from the last release; the pre-upgrade working tree is the correct baseline.

## R9. Batching and rollback

**Decision**: Deliver six batches: baseline, toolchain/CI/containers, Rust majors, frontend majors, desktop runtime alignment, and cleanup/evidence. Each batch has its own lockfile update, test run, security check, and rollback note.

**Rationale**: A single all-at-once change would make a failure across SSH, crypto, credentials, frontend tooling, and CI difficult to attribute. Batches preserve bisectability and allow a destabilizing dependency to be pinned without reverting unrelated security fixes.

**Alternatives considered**:
- One large pull request: rejected for reviewability and bisectability.
- One pull request per dependency: rejected because it multiplies CI cost and ignores coupled peer/version constraints.

## R10. Documentation and durable guidance

**Decision**: Produce `inventory.json`, `exceptions.json`, and `evidence.json` as the authoritative machine-readable records alongside `upgrade-inventory.md`, `exceptions.md`, and `validation.md` under the feature directory, and update contributor-facing documentation with the new minimum toolchain and runtime versions.

**Rationale**: The spec requires the upgrade to be auditable and repeatable. Keeping the detailed evidence with the feature and the durable minimum versions in the contributor docs separates release evidence from long-lived guidance.

**Alternatives considered**:
- Put everything in the pull request description: rejected because it is not durable or discoverable.
- Put everything in `docs/`: rejected because it would mix release-specific evidence with long-lived guidance.

## Resolved Unknowns

- **Latest stable targets**: resolved in R1–R5.
- **TypeScript 7 peer conflict**: resolved with a documented TypeScript 6.0.3 exception in R3.
- **Rust crypto/SSH coupling**: resolved by upgrading the SSH and cryptographic crates as one compatibility group in R2.
- **CI and container targets**: resolved in R5.
- **Lockfile and resolution policy**: resolved in R6.
- **Security evidence and exception format**: resolved in R7 and the contracts.
- **Performance and rollback approach**: resolved in R8 and R9.
- **Documentation artifacts**: resolved in R10.

No `NEEDS CLARIFICATION` items remain.
