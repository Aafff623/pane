# 02 - Refresh acceptance

Type: task
Status: resolved
Blocked by: 01
Coverage: A4

## What to build

Reproduce the creditTrend old-result overwrite through production behavior. Concentrate per-dataset acceptance and queued-attempt rules; preserve successful older data when newer attempts fail.

Authority: ../spec.md. Retain existing refresh-ux/trend-units product requirements.

## Acceptance

- [x] Trace all production, persisted and external consumers at this seam.
- [x] Record a behavioral red-before-green check, or contract proof for a justified retained abstraction.
- [x] Implement the complete vertical change; remove replaced duplicate policy.
- [x] Run targeted checks and affected integration/build checks.
- [x] Standards review, Spec review and simplification scan; fix findings and re-test.
- [x] Record exact evidence and any runtime/owner acceptance still open.

## Evidence

RED: actual refresh late trend test returned 100 instead of 200; queued manual-refresh test lost clearBenches. GREEN: five refresh tests plus all account-save regressions; combined frontend 40 passed, tsc/Vite passed. RefreshState now owns active generation, forced request merging, waiters and per-dataset newest-success acceptance. Removed redundant module-level refresh flags/counter/waiters. Standards/Spec direct review: preserve noninteractive timers, usage-only follow-up merge, failed newer results fallback and saved-key waiters. Existing account VM fixture now injects real RefreshState; original assertions retained. UI acceptance remains open at stage16.

## Comments
