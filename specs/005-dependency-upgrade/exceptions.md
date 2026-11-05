# Upgrade Exceptions

The authoritative machine-readable exception register is [exceptions.json](./exceptions.json).

| ID | Component | Current | Latest stable | Reason | Status | Expires |
|----|-----------|---------|---------------|--------|--------|---------|
| EX-001 | typescript | 6.0.3 | 7.0.2 | Peer conflict with `typescript-eslint` 8.72.1 (`<6.1.0`) | approved | 2027-05-05 |
| EX-002 | deepmerge | 4.3.1 | 4.3.1 | No fixed release for GHSA-gcqx-w48m-w2h4 | approved | 2027-05-05 |
| EX-003 | minimatch | 10.2.6 | 11.0.4 | Resolved by approved npm override to 11.0.4; retained closed for audit | closed | 2027-05-05 |
| EX-004 | rsa | 0.10.0-rc.18 | 0.9.10 | No fixed upgrade for RUSTSEC-2023-0071 | approved | 2027-05-05 |
| EX-005 | ssh-key | 0.7.0-rc.11 | 0.6.0 | Pre-release transitive dependency of russh 0.65.1 | approved | 2027-05-05 |
| EX-006 | zbus_names/zvariant | 5.15.1 | 5.15.1 | Unmaintained Linux Secret Service transitive stack | approved | 2027-05-05 |
| EX-007 | pkcs1 | 0.8.0-rc.4 | 0.7.5 | Pre-release transitive dependency of rsa/russh | approved | 2027-05-05 |

Only the low-severity deepmerge advisory and the medium-severity RSA timing advisory remain, covered by EX-002 and EX-004. EX-003 is closed after the approved minimatch override. No critical or high advisory remains. All CI actions, Rust crates, and container images are on current stable releases or verified commit pins.
