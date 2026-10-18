# 04 - Platform seam

Type: task
Status: resolved
Blocked by: 03
Coverage: A2

## What to build

Map Registry, ACL, random and path consumers. Reuse existing OS implementations/dependencies, move actual OS knowledge into platform, and strengthen the seam guard without banning ordinary portable Rust.

Authority: ../spec.md. Retain existing refresh-ux/trend-units product requirements.

## Acceptance

- [x] Trace all production, persisted and external consumers at this seam.
- [x] Record a behavioral red-before-green check, or contract proof for a justified retained abstraction.
- [x] Implement the complete vertical change; remove replaced duplicate policy.
- [x] Run targeted checks and affected integration/build checks.
- [x] Standards review, Spec review and simplification scan; fix findings and re-test.
- [x] Record exact evidence and any runtime/owner acceptance still open.

## Evidence

RED guard named kimi.rs, providers/mod.rs, onenewapi/ids.rs and store.rs. GREEN platform7, store29 (including real temporary-file DACL tightening/replacement), ids2, proxy1 and home2 tests passed. Direct Win32/extern/path-prefix knowledge now belongs to platform. Existing rand_core OsRng reused; no dependency added. keyvault private-write caller now uses platform instead of borrowing relay storage. GNU crate/test compilation passed. Other OS adapter compilation/runtime still pending stage16; source assertions are not a cross-platform runtime claim.

## Comments
