# 05 - Snapshot composition

Type: task
Status: resolved
Blocked by: 04
Coverage: A5, A6, W10, S3

## What to build

Move shared Snapshot decisions, identity comparison and cooldown policy to locally testable production interfaces. Preserve prune-before-stamp persistence, account generations, 429 and manual-refresh semantics.

Authority: ../spec.md. Retain existing refresh-ux/trend-units product requirements.

## Acceptance

- [x] Trace all production, persisted and external consumers at this seam.
- [x] Record a behavioral red-before-green check, or contract proof for a justified retained abstraction.
- [x] Implement the complete vertical change; remove replaced duplicate policy.
- [x] Run targeted checks and affected integration/build checks.
- [x] Standards review, Spec review and simplification scan; fix findings and re-test.
- [x] Record exact evidence and any runtime/owner acceptance still open.

## Evidence

Moved 11 existing fallback/wallet/cooldown tests from lib.rs into the real snapshot_store module compiled by parse-tests. Added6 composition/identity/cooldown tests:17 passed. RED additional repeated disk-failure test exposed stamp advance after a prior in-memory prune; GREEN requires persisted prune on every known swap before stamp advance. Shared identity_changed now serves boot and live paths. merge_snapshot owns success/failure-streak/wallet-cache acceptance; bench_after_error owns retry timing. Publishing/telemetry/notification effects and generation rechecks remain in orchestration. GNU check --tests passed; no new warnings retained.

## Comments
