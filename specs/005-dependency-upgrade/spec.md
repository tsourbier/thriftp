# Feature Specification: Dependency and Toolchain Upgrade

**Feature Branch**: `005-dependency-upgrade`

**Created**: 2026-11-05

**Status**: Draft

**Input**: User description: "dependabot triggered about 60+ errors, to fix most of those let's update all libraries/components used to their latest version."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Remove known security exposure from third-party code (Priority: P1)

A maintainer reviews the repository's dependency alerts and finds that the application no longer ships known-vulnerable libraries or runtime components. Every remaining advisory has an explicit, reviewed exception instead of being silently ignored.

**Why this priority**: The upgrade is primarily driven by security alerts. Users and maintainers must be able to trust that the shipped application does not carry avoidable vulnerabilities.

**Independent Test**: Run the project's dependency audit and review the advisory dashboard; confirm that critical and high-severity findings are resolved, that at least 90% of the original alert set is gone, and that every remaining finding has a documented exception.

**Acceptance Scenarios**:

1. **Given** the repository starts with roughly 60 open dependency alerts, **when** the upgrade is complete, **then** no open critical or high-severity advisory remains for any direct or transitive dependency.
2. **Given** the initial alert set, **when** the upgrade is complete, **then** at least 90% of all original alerts are resolved, and the remaining ones are explicitly triaged.
3. **Given** an advisory that cannot be resolved because no fixed release exists, **when** the upgrade is complete, **then** a documented exception records the dependency, risk, owner, expiry, mitigation, and approval.
4. **Given** the dependency audit runs in continuous integration, **when** it completes, **then** it passes without unapproved advisories for any supported platform.
5. **Given** the generated software bill of materials, **when** it is compared with the resolved dependencies, **then** it reflects the upgraded versions and contains no untracked components.
6. **Given** the repository's alert settings, **when** the upgrade is complete, **then** no alert rule or advisory is muted, snoozed indefinitely, or disabled to hide an unresolved issue.

---

### User Story 2 - Bring every dependency to the latest stable release (Priority: P1)

A maintainer reviews the dependency inventory and sees that every library, runtime component, build tool, automation action, and test container runs on the latest stable release available, with no accidental pre-release or abandoned versions.

**Why this priority**: Updating to the latest stable baseline fixes most alerts, reduces drift, and prevents the project from accumulating unsupported components.

**Independent Test**: Compare the committed dependency inventory against each component's latest stable release; confirm every direct dependency matches, every transitive dependency is resolved to a supported version, and every exception is documented.

**Acceptance Scenarios**:

1. **Given** the direct dependency inventory, **when** the upgrade is complete, **then** every direct dependency is on its latest stable release available at implementation time.
2. **Given** transitive dependencies, **when** the dependency resolution records are inspected, **then** they resolve to supported versions that remove known advisories and avoid duplicate versions where compatible.
3. **Given** language runtimes, toolchains, package managers, and build tools, **when** the upgrade is complete, **then** each uses its latest stable supported release.
4. **Given** continuous-integration automation actions, **when** the upgrade is complete, **then** each action uses its latest stable major release and is pinned according to project policy.
5. **Given** the containerized protocol test services, **when** the upgrade is complete, **then** their base images and installed packages are on current supported releases and the services still start correctly.
6. **Given** an unused dependency, **when** the inventory is reviewed, **then** it is removed rather than upgraded.
7. **Given** a deprecated or unmaintained dependency, **when** the upgrade is complete, **then** it is replaced or recorded as an approved exception with a migration plan.
8. **Given** a pre-release, beta, release candidate, or nightly component, **when** the inventory is reviewed, **then** it is absent unless an approved exception explains why no stable release is usable.
9. **Given** the dependency resolution records, **when** a clean install is performed, **then** the resolved versions are deterministic and committed, and no force-resolution, legacy-peer, or skipped-install option is required.
10. **Given** a new dependency source such as an unofficial fork or alternate registry, **when** it is proposed, **then** it is reviewed for provenance, maintenance, license, and security before adoption.

---

### User Story 3 - Keep the product behaving and feeling the same (Priority: P1)

A user upgrades to the new release and can connect, browse, transfer, use favorites, run the command-line interface, and use the automation server exactly as before. Security prompts, credential handling, and keyboard workflows are unchanged.

**Why this priority**: Dependency upgrades are only acceptable if they do not regress the product. A green build that changes behavior or weakens security is not a successful upgrade.

**Independent Test**: Run the existing automated suites and a manual smoke test covering every supported protocol and primary workflow; compare outputs, exit codes, security prompts, saved data, and performance with the pre-upgrade baseline.

**Acceptance Scenarios**:

1. **Given** the existing automated test suites, **when** the upgraded code is tested, **then** every suite passes without skipped, ignored, or deleted tests used to hide failures.
2. **Given** FTP, explicit FTPS, implicit FTPS, and SFTP servers, **when** the user connects, authenticates, lists directories, uploads, downloads, renames, and deletes, **then** each operation behaves as before.
3. **Given** the command-line interface, **when** its documented commands are exercised, **then** machine-readable output, exit codes, and credential behavior remain compatible.
4. **Given** the automation server, **when** its documented tools, approvals, scopes, and audit behavior are exercised, **then** they remain compatible and no security rule is weakened.
5. **Given** credential storage, certificate validation, host-key verification, and plain-FTP warnings, **when** the upgraded application is used, **then** the same protections and user confirmations apply.
6. **Given** existing saved profiles, server favorites, directory favorites, tabs, preferences, queue state, and transfer history, **when** the application starts after the upgrade, **then** the data loads without loss or unintended migration.
7. **Given** the desktop interface, **when** the user navigates by keyboard, switches themes, opens dialogs, and uses previews, **then** behavior, focus handling, and accessibility remain equivalent.
8. **Given** the application's performance baseline, **when** listings, transfers, and startup are measured after the upgrade, **then** regressions stay within the agreed tolerance and none are accepted without investigation.
9. **Given** the release artifacts, **when** they are built for each supported platform, **then** the application installs, launches, connects, and transfers successfully.
10. **Given** the product's privacy commitments, **when** the upgraded dependencies are reviewed, **then** no new mandatory telemetry, account, cloud dependency, or secret-handling change is introduced.

---

### User Story 4 - Restore a clean, reproducible build on every platform (Priority: P1)

A contributor clones the repository on macOS, Linux, or Windows, installs from the committed dependency records, and gets a green build and test run without workarounds. Continuous integration is green and no longer reports the dependency-related failures that triggered this work.

**Why this priority**: The reported problem includes 60+ errors. A modern dependency set is only useful if it produces reliable, repeatable builds and tests on every supported platform.

**Independent Test**: From a clean checkout and a fresh cache, install, build, lint, format-check, test, and package the application on each supported operating system; confirm every required check passes.

**Acceptance Scenarios**:

1. **Given** a clean checkout with no cached dependencies, **when** a contributor installs and builds, **then** the process succeeds from the committed resolution records on macOS, Linux, and Windows.
2. **Given** the full continuous-integration pipeline, **when** it runs on the upgraded code, **then** every required job passes on every supported operating system.
3. **Given** formatting, linting, type-checking, documentation, and warning-denied checks, **when** they run, **then** they pass with zero unresolved warnings.
4. **Given** the reproducible-build verification, **when** it runs, **then** repeated builds from the same inputs produce the expected identical or equivalently verified output.
5. **Given** the containerized protocol tests, **when** they run with the upgraded test images, **then** they pass and remain runnable on every supported development platform with a container runtime.
6. **Given** a failure in the upgraded pipeline, **when** the failure is reviewed, **then** it has an actionable cause and is not masked by retries, ignored jobs, or removed coverage.
7. **Given** a fresh environment with no manual setup beyond the documented prerequisites, **when** a maintainer follows the contributor instructions, **then** the result matches continuous integration.

---

### User Story 5 - Modernize the runtime and platform baseline safely (Priority: P2)

A maintainer can see which language runtimes, desktop runtime components, package managers, automation runners, and test containers are supported, and can confirm that the upgrade does not silently drop a supported operating system or raise requirements without a decision.

**Why this priority**: Latest dependencies can raise minimum runtime or operating-system requirements. The project must make that trade-off explicitly instead of discovering it after release.

**Independent Test**: Review the documented minimum versions, build and run on each supported platform, and confirm every runtime, runner, and container image is current and supported.

**Acceptance Scenarios**:

1. **Given** language runtimes, desktop runtime components, and package managers, **when** their versions are reviewed, **then** they are latest stable and documented as minimum supported versions.
2. **Given** the supported operating systems, **when** the upgrade is complete, **then** each still builds and runs, or an explicit approved decision records the dropped support and its user impact.
3. **Given** continuous-integration runner images, **when** the upgrade is complete, **then** they are current supported images and match the documented toolchain.
4. **Given** the test container images, **when** they are rebuilt, **then** they use current supported base images and the protocol integration suite passes.
5. **Given** desktop runtime components and their companion packages, **when** they are upgraded, **then** their versions remain mutually compatible and the desktop shell builds on every supported platform.
6. **Given** installer and bundle generation, **when** it runs on each supported platform, **then** the artifacts build and launch successfully.
7. **Given** a latest release that would require dropping a supported platform, **when** the upgrade is planned, **then** the trade-off is documented and approved before the change is merged.

---

### User Story 6 - Make the upgrade auditable and repeatable (Priority: P2)

A maintainer or reviewer can see exactly what changed, why each version was chosen, which breaking changes required code updates, and how to roll back or repeat the upgrade. Exceptions and security-sensitive changes are visible rather than buried in a large change set.

**Why this priority**: A broad upgrade is risky if it cannot be reviewed, reproduced, or reversed. Clear evidence protects future maintenance and release quality.

**Independent Test**: Hand the upgrade inventory, exception register, test evidence, and rollback notes to another maintainer and confirm they can reproduce the result or reverse a problematic batch.

**Acceptance Scenarios**:

1. **Given** the dependency inventory, **when** it is reviewed, **then** every dependency lists its previous version, new version, update type, and reason for the change.
2. **Given** a major-version upgrade, **when** it is reviewed, **then** the inventory records the migration guidance consulted and the code or configuration changes made.
3. **Given** an approved exception, **when** it is reviewed, **then** it records the dependency, advisory or compatibility reason, risk, owner, expiry, mitigation, and approval.
4. **Given** security-sensitive dependencies, **when** they are upgraded, **then** the security review evidence is recorded with the change.
5. **Given** the release notes or changelog, **when** they are published, **then** they summarize the upgrade scope, any required user action, and any visible behavior change.
6. **Given** the upgrade batches, **when** a batch cannot be stabilized, **then** it can be reverted or pinned independently using the documented rollback notes.
7. **Given** a new maintainer, **when** they follow the documented upgrade steps, **then** they can reproduce the dependency resolution and verification results.
8. **Given** dependency monitoring, **when** the upgrade is complete, **then** it remains enabled so newly disclosed issues continue to surface.

---

### User Story 7 - Reduce dependency debt while upgrading (Priority: P3)

A maintainer sees fewer redundant, unused, or abandoned dependencies after the upgrade. Duplicate versions and unnecessary installation weight are reduced where compatible, and licensing remains clean.

**Why this priority**: Removing dependency debt lowers future alert volume and maintenance cost, but must not compromise compatibility or delay the security upgrade.

**Independent Test**: Compare dependency counts, duplicate versions, installation size, license reports, and generated bills of materials before and after; confirm reductions are explained and no required capability was removed.

**Acceptance Scenarios**:

1. **Given** dependencies that no code imports or uses, **when** the inventory is reviewed, **then** they are removed and the build still passes.
2. **Given** multiple resolved versions of the same library, **when** they are compatible, **then** they are consolidated, or the remaining duplication is documented with the blocking constraint.
3. **Given** a deprecated or unmaintained dependency, **when** it is replaced, **then** the replacement is actively maintained, appropriately licensed, and covered by tests.
4. **Given** the license report, **when** the upgraded dependency set is checked, **then** all licenses remain allowed under project policy.
5. **Given** installation size or dependency count, **when** installation size increases by more than 5% or dependency count increases by more than 10% over the baseline, **then** the increase is justified in the upgrade inventory.
6. **Given** the reduced dependency set, **when** the application is exercised, **then** no user-facing capability has been removed.

---

### Edge Cases

- The latest release of a library exists only as a pre-release, beta, nightly, or release candidate: no stable upgrade is possible, so the current stable version stays with a documented exception and a review date.
- Two required libraries depend on incompatible versions of a shared library: the conflict is resolved by choosing compatible releases, replacing one library, or documenting the duplication with its risk.
- The latest release of a component requires a newer operating system, runtime, or processor architecture than the project supports: the platform decision is escalated and recorded before the upgrade is accepted.
- A dependency has no fixed release for a known advisory: the project replaces it, applies an approved mitigation, or records a time-bound exception.
- A transitive dependency is vulnerable but no direct upgrade can resolve it: the project uses an approved resolution override, replaces the parent dependency, or records an exception.
- A package is yanked or withdrawn after being selected: the resolution is rebuilt against a stable alternative and all affected lock records are refreshed.
- A dependency's latest release adds telemetry, a network call, or a license change: it is reviewed against the product's privacy and open-source commitments before adoption.
- A dependency's latest release changes authentication, cryptography, or transport behavior: the security review blocks the upgrade until behavior parity is demonstrated.
- A major upgrade removes an API used by the project: the code is migrated, or the upgrade is deferred with a documented exception and follow-up plan.
- A peer-dependency conflict requires a force-resolution or legacy-peer option: the option is rejected unless an approved exception documents why it is safe.
- Continuous integration passes locally but fails on one operating system: the platform-specific failure is fixed or documented, and the job is not allowed to remain red.
- A test fails only because it asserts a removed or changed dependency behavior: the test is updated only when the product behavior is intentionally preserved and the change is reviewed.
- Reproducible builds stop matching after an upgrade: the source of nondeterminism is identified and fixed before release.
- A dependency upgrade causes a performance regression beyond tolerance: the upgrade is optimized, deferred, or replaced, with benchmark evidence.
- The dependency registry is unavailable or rate-limited during resolution: the upgrade is retried, and no unverified or manually edited resolution records are committed.
- A container base image tag disappears or becomes unsupported: a current supported image is selected and the integration suite is rerun.
- A dependency is abandoned but has no maintained replacement: it is recorded as an exception with a migration plan and a review date.
- A malformed or compromised dependency artifact is suspected: the upgrade stops, the artifact is reported, and provenance is verified before continuing.

## Requirements *(mandatory)*

### Functional Requirements

**Upgrade scope and version policy**

- **FR-001**: The project MUST maintain a complete inventory of third-party libraries, runtime components, build and test tools, automation actions, and container images used by the desktop application, core operations, command-line interface, automation server, tests, build pipeline, and release process.
- **FR-002**: Every direct dependency MUST be updated to the latest stable release available at implementation time, including major versions when available.
- **FR-003**: Pre-release, beta, release-candidate, nightly, or unreleased builds MUST NOT be used unless an approved exception documents why no stable release is usable.
- **FR-004**: Transitive dependencies MUST be resolved to supported versions that remove known critical and high-severity advisories and that avoid duplicate versions where compatibility allows.
- **FR-005**: Language runtimes, toolchains, package managers, and build tools MUST be updated to the latest stable supported releases.
- **FR-006**: Continuous-integration automation actions MUST be updated to their latest stable major releases and pinned according to project policy.
- **FR-007**: Container base images and packages used by the protocol test services MUST be updated to current supported releases.
- **FR-008**: Unused dependencies MUST be removed, and deprecated or unmaintained dependencies MUST be replaced or recorded as approved exceptions with a migration plan.
- **FR-009**: Dependency resolution records MUST be regenerated by the appropriate tooling, committed, and kept consistent across the workspace; no manual version edits, force-resolution flags, legacy-peer flags, or skipped install scripts may be used to make the build pass.
- **FR-010**: New dependency sources, unofficial forks, alternate registries, and patched packages MUST be reviewed for provenance, maintenance status, license, and security before adoption.

**Security and compliance**

- **FR-011**: After the upgrade, zero open critical or high-severity advisories may remain for direct or transitive dependencies.
- **FR-012**: At least 90% of the known initial dependency alerts MUST be resolved, and every remaining advisory MUST be explicitly triaged.
- **FR-013**: No alert rule, advisory, or security check may be muted, snoozed indefinitely, disabled, or excluded to hide an unresolved issue. Approved exceptions MUST remain visible to reviewers.
- **FR-014**: Security-sensitive upgrades involving credentials, cryptography, authentication, transport security, or secret handling MUST receive an explicit security review and MUST preserve existing protections.
- **FR-015**: The upgrade MUST NOT weaken certificate validation, host-key verification, credential storage, plain-FTP warnings, least-privilege behavior, or secret redaction.
- **FR-016**: The software bill of materials MUST be regenerated from the upgraded dependency set and MUST match the resolved components.
- **FR-017**: The license check MUST pass, and no new dependency may introduce a license that violates project policy.
- **FR-018**: Dependency manifests, resolution records, generated bills of materials, logs, and error messages MUST NOT contain credentials, tokens, private keys, or other secrets.

**Behavior and compatibility**

- **FR-019**: The upgrade MUST NOT add new user-facing features or intentionally change product behavior except where required to preserve compatibility with an upgraded component.
- **FR-020**: FTP, explicit FTPS, implicit FTPS, and SFTP connection, authentication, browsing, upload, download, rename, and delete behavior MUST remain equivalent.
- **FR-021**: Command-line interface commands, machine-readable output, stable exit codes, and credential behavior MUST remain compatible.
- **FR-022**: Automation-server tools, authorization rules, approval requirements, scopes, audit records, and security parity MUST remain compatible.
- **FR-023**: Existing saved profiles, server favorites, directory favorites, tabs, preferences, queue state, transfer history, and audit records MUST load without loss or unintended migration.
- **FR-024**: Desktop keyboard navigation, focus handling, themes, dialogs, previews, and accessibility behavior MUST remain equivalent.
- **FR-025**: The upgrade MUST NOT introduce mandatory telemetry, an account requirement, a cloud dependency, or a new plaintext secret path.
- **FR-026**: Listing, transfer, queue, and startup performance MUST remain within the agreed tolerance of the pre-upgrade baseline.
- **FR-027**: Every currently supported operating system MUST continue to build, install, launch, and pass its required checks unless an approved platform decision explicitly changes support.

**Verification and build quality**

- **FR-028**: All existing automated unit, integration, contract, security, platform, and performance tests MUST pass.
- **FR-029**: Tests MUST NOT be skipped, ignored, deleted, or weakened to accommodate an upgrade. Tests that assert intentionally preserved behavior MUST be updated only with review.
- **FR-030**: New or updated automated tests MUST cover dependency-driven behavior changes, resolution edge cases, and the failure modes introduced by major upgrades.
- **FR-031**: Formatting, linting, type-checking, documentation, and warning-denied checks MUST pass with no unresolved warnings.
- **FR-032**: The full continuous-integration pipeline MUST pass on macOS, Linux, and Windows, including the protocol integration, contract, security, multi-process, audit, bill-of-materials, reproducible-build, and packaging stages.
- **FR-033**: A clean checkout with no cached dependencies MUST install, build, and test successfully from the committed resolution records on every supported platform.
- **FR-034**: Reproducible-build verification MUST pass after the upgrade.
- **FR-035**: Containerized protocol tests MUST build and pass with the updated test images on every supported development platform where a compatible container runtime is available.
- **FR-036**: Desktop installers and application bundles MUST build and launch on every supported platform.
- **FR-037**: Any failure in the upgraded pipeline MUST be diagnosed and fixed rather than hidden by retries, ignored jobs, removed coverage, or reduced test scope.
- **FR-038**: Performance benchmarks MUST be run before and after the upgrade, and any regression beyond the agreed tolerance MUST be investigated and resolved or explicitly approved.

**Governance, documentation, and repeatability**

- **FR-039**: The upgrade inventory MUST record each dependency's previous version, new version, category, update type, and reason, plus the migration guidance and code changes for each major upgrade.
- **FR-040**: Every exception MUST record the affected dependency, advisory or compatibility reason, risk, owner, expiry date, mitigation, and approval.
- **FR-041**: The release notes or changelog MUST summarize the upgrade scope, required user action, and any user-visible change.
- **FR-042**: The minimum supported operating-system, runtime, toolchain, package-manager, and automation-runner versions MUST be documented and enforced in continuous integration.
- **FR-043**: A rollback plan MUST exist for each upgrade batch so a destabilizing change can be reverted or pinned without reverting unrelated fixes.
- **FR-044**: Dependency monitoring MUST remain enabled after the upgrade so newly disclosed issues continue to surface.
- **FR-045**: The upgrade MUST be reproducible by another maintainer using the documented steps, inventory, and verification evidence.
- **FR-046**: No upgrade batch may be released while a required check is red, an unapproved advisory remains, or an exception is missing required fields.

### Key Entities *(include if data involved)*

- **Dependency Inventory Item**: A third-party or runtime component used by the project. Attributes include name, category, direct or transitive status, previous version, target version, latest stable version, update type, source, license, and upgrade status.
- **Compatibility Baseline**: The pre-upgrade reference used to detect regressions. Attributes include supported operating systems, minimum runtime and toolchain versions, test and benchmark results, artifact sizes, startup measurements, and public interface expectations.
- **Security Finding**: A disclosed vulnerability or policy issue affecting a dependency. Attributes include identifier, severity, affected component and versions, fixed version if any, resolution status, and triage decision.
- **Upgrade Exception**: An approved deviation from the latest-stable policy or from a zero-advisory target. Attributes include dependency, reason, risk, owner, expiry, mitigation, approval, and review date.
- **Upgrade Batch**: A reviewable group of related dependency changes. Attributes include included components, breaking changes, migration work, verification evidence, rollback plan, and release status.
- **Verification Evidence**: The proof that a batch is acceptable. Attributes include test runs, platform coverage, audit output, bill-of-materials comparison, license report, performance results, reproducible-build result, and security review.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of open critical and high-severity dependency advisories are resolved at completion.
- **SC-002**: At least 90% of the roughly 60 initial dependency alerts are resolved; every remaining alert has a complete, approved, time-bound exception.
- **SC-003**: 100% of direct dependencies are on their latest stable release at completion, or are covered by an approved exception.
- **SC-004**: No pre-release, beta, nightly, or end-of-life runtime, library, tool, automation action, or container image remains in the shipped or build-critical dependency set.
- **SC-005**: 100% of required automated checks pass on macOS, Linux, and Windows in a clean run.
- **SC-006**: A fresh checkout installs, builds, and tests successfully on all supported operating systems without force-resolution or legacy-compatibility flags.
- **SC-007**: 100% of existing automated tests pass, with zero tests skipped or removed to hide a failure.
- **SC-008**: 100% of security regression checks pass, including credential handling, certificate validation, host-key verification, plain-FTP warnings, and secret redaction.
- **SC-009**: Listing, transfer, queue, and startup performance remain within 5% of the pre-upgrade baseline, or any larger change is explicitly investigated and approved.
- **SC-010**: Repeated builds from identical inputs produce the expected equivalent verified output after the upgrade.
- **SC-011**: The bill-of-materials and license checks pass, with 100% of resolved components accounted for and no disallowed licenses.
- **SC-012**: The upgrade inventory covers 100% of dependencies and records previous version, new version, update type, and reason for every entry.
- **SC-013**: 100% of exceptions contain the required rationale, risk, owner, expiry, mitigation, and approval.
- **SC-014**: Desktop installers and bundles build and launch on 100% of supported platforms.
- **SC-015**: Protocol integration tests pass in isolated test environments on every supported development platform.
- **SC-016**: Dependency monitoring remains enabled, with zero muted or disabled alerts and no unresolved critical or high-severity findings hidden from review.
- **SC-017**: Another maintainer can reproduce the upgrade and its verification results by following the documented inventory and steps.

## Assumptions

- "Latest version" means the latest stable, non-pre-release release available at implementation time, including major versions, and not a nightly, beta, release candidate, or long-term-support pin unless required.
- The upgrade covers all direct and transitive dependencies used by the application, core operations, desktop shell, command-line interface, automation server, tests, benchmarks, build tooling, continuous integration, packaging, and test containers.
- Code and configuration changes required by major upgrades are in scope. User-facing behavior, public interfaces, data formats, and security guarantees are not intentionally changed.
- The roughly 60 alerts referenced by the requester are the project's current dependency advisories. Dependency audit output and the repository's advisory dashboard are the authoritative measures of completion.
- Approved exceptions are permitted only when no stable upgrade exists, a dependency conflict cannot be resolved, a platform requirement would otherwise be dropped, or a replacement would be riskier. Every exception is time-bound and reviewed.
- The project's currently supported operating systems remain supported. If a latest release requires dropping one, the trade-off is escalated and documented before merge.
- Existing dependency monitoring remains the mechanism for future alerts. This feature does not introduce a new dependency-management service or replace the current workflow.
- Network access to official package sources is available during the upgrade and in continuous integration. Air-gapped or offline-only resolution is out of scope.
- The upgrade may be delivered in reviewable batches, but the final integrated state must satisfy every requirement and success criterion as a single released baseline.
- Performance comparison uses the project's existing benchmark and startup measurements; no new product performance target is introduced by this feature.
- The project's release process, versioning, and supported-platform policy remain unchanged except for the documented minimum-version updates required by the new dependency set.
