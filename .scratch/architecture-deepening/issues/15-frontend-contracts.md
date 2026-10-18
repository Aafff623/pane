# 15 - Frontend and localization contracts

Type: task
Status: resolved
Blocked by: 14
Coverage: W8, W9

## What to build

Inventory real frontend behavioral coverage. Add the missing high-value pure-module tests and a cross-language metric localization contract gate; keep source literals/protocol errors intact.

Authority: ../spec.md. Retain existing refresh-ux/trend-units product requirements.

## Acceptance

- [x] Trace all production, persisted and external consumers at this seam.
- [x] Record a behavioral red-before-green check, or contract proof for a justified retained abstraction.
- [x] Implement the complete vertical change; remove replaced duplicate policy.
- [x] Run targeted checks and affected integration/build checks.
- [x] Standards review, Spec review and simplification scan; fix findings and re-test.
- [x] Record exact evidence and any runtime/owner acceptance still open.

## Evidence

Completed 2026-10-18; see the continuation evidence below.

## Comments


## Restart continuation evidence (2026-10-18)

Resume audit: tests/peak-hours.test.mjs covers UTC+8 interpretation,
half-open windows and weekend differences. tests/reset-wake.test.mjs exercises
the real resetWakeBadge via the established TypeScript/VM adapter, including
disabled providers, unknown states and failure hints. Removed the unnecessary
resetWakeState.ts extraction after the deletion test showed it only moved a
small expression for testing. tests/metric-language-contract.test.mjs reads the
actual Rust zh/ru metric match arms and requires matching frontend label
mappings; vendor-defined labels remain unchanged. The pre-existing dictionary
pairing tests still check all three locale key sets. Node total: 49 passed.
