# Data Model: Dependency and Toolchain Upgrade

**Feature**: `005-dependency-upgrade` | **Date**: 2026-11-05

This feature introduces no runtime data model changes. Its data model describes the governance and verification records used to plan, execute, and audit the upgrade. These records are stored under `specs/005-dependency-upgrade/` and are governed by the schemas in [`contracts/dependency-inventory.schema.json`](./contracts/dependency-inventory.schema.json) and [`contracts/upgrade-exception.schema.json`](./contracts/upgrade-exception.schema.json).

## Entity Overview

```text
Compatibility Baseline
        │
        ├── measured by ──► Verification Evidence ◄── produced by ── Upgrade Batch
        │                                                        │
        └── compared against ──► Dependency Inventory Item       │
                                   │                             │
                                   ├── affected by ──► Security Finding
                                   │                             │
                                   └── may require ──► Upgrade Exception
                                                                 │
                                                                 └── release gate ──► Upgrade Batch
```

## Dependency Inventory Item

**Represents**: One third-party library, runtime component, build or test tool, automation action, or container image used by the project.

**Fields**:

| Field | Type | Required | Description |
|---|---|---|---|
| `id` | string | yes | Stable identifier, for example `rust/russh` or `npm/vite` |
| `name` | string | yes | Component name as published |
| `category` | enum | yes | `runtime`, `library`, `frontend`, `build-tool`, `test-tool`, `automation`, `container` |
| `scope` | enum | yes | `runtime`, `development`, `build`, `test`, `release` |
| `relationship` | enum | yes | `direct` or `transitive` |
| `currentVersion` | string | yes | Version before the upgrade |
| `targetVersion` | string | yes | Selected latest stable version |
| `latestStableVersion` | string | yes | Latest stable version observed at selection time |
| `updateType` | enum | yes | `patch`, `minor`, `major`, `replacement`, `removal` |
| `source` | string | yes | Official registry, repository, action, or image reference |
| `reason` | string | yes | Why this change is part of the upgrade |
| `migrationNotes` | string | conditional | Required for major upgrades and replacements |
| `breakingChanges` | string[] | no | Known API, behavior, or configuration changes |
| `license` | string | yes | Declared license |
| `advisoryIds` | string[] | no | Security findings addressed by this change |
| `exceptionId` | string | conditional | Required when the target is not the latest stable version |
| `status` | enum | yes | `planned`, `in-progress`, `verified`, `deferred`, `removed` |
| `evidence` | string[] | no | Links or paths to test, audit, benchmark, or review evidence |

**Validation rules**:

- Every direct dependency used by the application, core, desktop shell, CLI, MCP server, tests, benchmarks, build pipeline, CI, and containers must have exactly one inventory item.
- `targetVersion` must equal `latestStableVersion` unless `exceptionId` is present.
- A `major`, `replacement`, or `removal` update must include `migrationNotes`.
- An item with an open critical or high-severity `advisoryIds` entry cannot have `status: verified`.
- `status: removed` requires proof that the component is no longer referenced.
- Pre-release versions require an exception and must be marked in `migrationNotes`.

**State transitions**:

```text
planned ──► in-progress ──► verified
   │              │
   │              ├──► deferred (exception required)
   │              └──► planned (rollback)
   └──► removed (unused dependency)
```

## Compatibility Baseline

**Represents**: The pre-upgrade reference used to detect regressions.

**Fields**:

| Field | Type | Required | Description |
|---|---|---|---|
| `capturedAt` | timestamp | yes | When the baseline was recorded |
| `commit` | string | yes | Starting commit or tree identifier |
| `toolchains` | object | yes | Rust, Node.js, package manager, and automation runner versions |
| `platforms` | string[] | yes | Supported operating systems exercised |
| `tests` | object | yes | Test suite names, pass/fail counts, and durations |
| `benchmarks` | object | yes | Benchmark names and measurements |
| `startup` | object | yes | Startup timing measurements |
| `artifactSizes` | object | yes | Installer, bundle, and release artifact sizes |
| `audit` | object | yes | Advisory counts by severity and ecosystem |
| `licenses` | object | yes | License report summary |
| `publicInterfaces` | object | yes | CLI/MCP contract results and desktop interface expectations |
| `savedDataFixtures` | string[] | yes | Profile, favorite, tab, preference, queue, and audit fixtures used for compatibility checks |

**Validation rules**:

- The baseline must be captured before the first dependency change.
- Every later performance comparison uses the same measurement method and environment class.
- The baseline must include at least one successful run of every required test class.
- A baseline missing a required measurement cannot be used to approve a regression.

## Security Finding

**Represents**: A disclosed vulnerability or policy issue affecting a dependency.

**Fields**:

| Field | Type | Required | Description |
|---|---|---|---|
| `id` | string | yes | Advisory or finding identifier |
| `severity` | enum | yes | `critical`, `high`, `moderate`, `low`, `info` |
| `ecosystem` | string | yes | Dependency ecosystem or source |
| `component` | string | yes | Affected component |
| `affectedVersions` | string | yes | Version range reported as affected |
| `fixedVersion` | string | conditional | First version containing a fix, if any |
| `resolution` | enum | yes | `upgraded`, `replaced`, `removed`, `exception`, `not-applicable` |
| `exceptionId` | string | conditional | Required when resolution is `exception` |
| `evidence` | string[] | yes | Audit output, advisory link, or review note |
| `status` | enum | yes | `open`, `resolved`, `accepted` |

**Validation rules**:

- An `open` critical or high finding blocks release.
- An `accepted` finding requires an approved exception.
- A `resolved` finding must name the version or replacement that resolves it.
- Findings must not be deleted to improve the count.

## Upgrade Exception

**Represents**: An approved deviation from the latest-stable policy or the zero-advisory target.

**Fields**:

| Field | Type | Required | Description |
|---|---|---|---|
| `id` | string | yes | Stable identifier, for example `EX-001` |
| `component` | string | yes | Affected dependency |
| `currentVersion` | string | yes | Version retained or selected |
| `latestStableVersion` | string | conditional | Latest version being deferred |
| `reason` | enum | yes | `no-stable-release`, `peer-conflict`, `platform-requirement`, `no-fixed-release`, `replacement-risk`, `other` |
| `description` | string | yes | Plain-language explanation |
| `risk` | string | yes | Residual risk and affected users or systems |
| `mitigation` | string | yes | Compensating control or containment |
| `owner` | string | yes | Accountable maintainer or role |
| `approvedBy` | string | yes | Approver |
| `createdAt` | date | yes | Creation date |
| `expiresAt` | date | yes | Review or expiry date |
| `reviewDate` | date | yes | Next scheduled review |
| `status` | enum | yes | `proposed`, `approved`, `expired`, `closed` |

**Validation rules**:

- `expiresAt` must be later than `createdAt`.
- An exception cannot be approved by the same person who proposed it unless project policy explicitly allows self-approval for non-security exceptions.
- A `peer-conflict` exception must name the incompatible peer and the supported version range.
- A `no-fixed-release` exception must include a mitigation and a review date.
- An expired exception blocks release until renewed or closed.
- Security-sensitive exceptions require the security review evidence referenced in the plan.

## Upgrade Batch

**Represents**: A reviewable group of related dependency changes.

**Fields**:

| Field | Type | Required | Description |
|---|---|---|---|
| `id` | string | yes | Batch identifier |
| `name` | string | yes | Human-readable batch name |
| `components` | string[] | yes | Inventory item IDs included |
| `dependsOn` | string[] | no | Previous batch IDs |
| `breakingChanges` | string[] | no | Migration work required |
| `verification` | string[] | yes | Commands and evidence used |
| `rollback` | string | yes | How to revert or pin the batch |
| `status` | enum | yes | `planned`, `in-progress`, `verified`, `reverted` |
| `releaseReady` | boolean | yes | Whether the batch may be included in a release |

**Validation rules**:

- A batch cannot be `verified` while any included item is `planned`, `in-progress`, or `deferred` without an exception.
- A batch cannot be `releaseReady` while a required check is red.
- Every batch must have a rollback note.
- A batch that changes a security-sensitive component must link its security review.

## Verification Evidence

**Represents**: The proof that a batch or the final baseline is acceptable.

**Fields**:

| Field | Type | Required | Description |
|---|---|---|---|
| `id` | string | yes | Evidence identifier |
| `batchId` | string | yes | Related batch |
| `kind` | enum | yes | `test`, `audit`, `benchmark`, `build`, `package`, `license`, `sbom`, `security-review`, `reproducibility`, `platform` |
| `command` | string | yes | Reproducible command or procedure |
| `environment` | string | yes | Operating system, toolchain, and relevant runtime |
| `result` | enum | yes | `pass`, `fail`, `warning` |
| `summary` | string | yes | Outcome and key measurements |
| `artifact` | string | no | Path or link to detailed output |
| `capturedAt` | timestamp | yes | When the evidence was captured |

**Validation rules**:

- A `fail` result blocks the related batch from being `verified`.
- A `warning` result requires explicit review and either resolution or an exception.
- Performance evidence must use the same method as the baseline.
- Security evidence must include the audit output and the security review for sensitive components.

## Relationships

- A **Compatibility Baseline** is measured by many **Verification Evidence** records and compared against many **Dependency Inventory Items**.
- A **Dependency Inventory Item** may address zero or more **Security Findings** and may reference zero or one **Upgrade Exception**.
- An **Upgrade Batch** contains one or more **Dependency Inventory Items**, depends on zero or more previous batches, and produces one or more **Verification Evidence** records.
- A **Security Finding** with resolution `exception` references exactly one **Upgrade Exception**.
- A **Verification Evidence** record belongs to exactly one batch and may cover multiple inventory items.

## Storage and Lifecycle

- The authoritative machine-readable records are `inventory.json`, `exceptions.json`, and `evidence.json` under `specs/005-dependency-upgrade/`. Each file is a JSON array whose elements are validated against the corresponding item schema; an empty register is `[]`.
- `upgrade-inventory.md`, `exceptions.md`, and `validation.md` are human-readable narratives that use the same record IDs and must not contradict the JSON.
- A missing or malformed JSON file fails validation. An empty inventory or evidence register fails the final release gate.
- Approved exceptions remain visible in `exceptions.json` and `exceptions.md` even after closure for audit history.
- Validation evidence is recorded in `evidence.json` and summarized in `validation.md`, with links from the batch and inventory entries.
- Records are updated after each batch and re-validated before release.
- Closed or expired records are retained for audit but excluded from the active release gate.
