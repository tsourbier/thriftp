# Release process

1. Ensure `main` is green on all CI jobs: unit, frontend, protocol integration,
   platform matrix, audit, SBOM, build, and reproducible.
2. Update the version in the workspace and `apps/desktop/package.json`.
3. Update `README.md` and relevant `docs/`.
4. Run locally:

   ```bash
   cargo fmt --all -- --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace --locked
   RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked
   python3 scripts/validate-dependency-records.py specs/005-dependency-upgrade --release
   python3 -m unittest discover -s scripts/tests -p 'test_*.py'
   scripts/check-dependency-policy.sh
   (cd apps/desktop && npm ci && npm run lint && npm run format && npm test && npm run build)
   scripts/test-integration.sh
   scripts/verify-reproducible-build.sh
   ```

5. Generate the release SBOM with `scripts/generate-sbom.sh`.
6. Build and sign the macOS, Linux, and Windows desktop bundles.
7. Tag the release and attach bundles, checksums, and the SBOM.
8. Verify that no secrets, telemetry endpoints, or account requirements were
   introduced.

Docker is used only by the protocol test harness; release artifacts do not
depend on a container runtime.
