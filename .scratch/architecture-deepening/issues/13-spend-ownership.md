# 13 - Spend source ownership

Type: task
Status: resolved
Blocked by: 12
Coverage: A12, W11

## What to build

Expose production composition through a real test seam and cover overlapping/non-overlapping days, pi destinations, mcode backfill, model metadata and historical aliases without second deduplication.

Authority: ../spec.md. Retain existing refresh-ux/trend-units product requirements.

## Acceptance

- [x] Trace all production, persisted and external consumers at this seam.
- [x] Record a behavioral red-before-green check, or contract proof for a justified retained abstraction.
- [x] Implement the complete vertical change; remove replaced duplicate policy.
- [x] Run targeted checks and affected integration/build checks.
- [x] Standards review, Spec review and simplification scan; fix findings and re-test.
- [x] Record exact evidence and any runtime/owner acceptance still open.

## Evidence

- `spend_history` owns permanent daily replacement and alias folding; same-day
  `kimi`/`moonshot` rows select one cell by measured tokens, while earlier
  kimi-only days survive under moonshot. This prevents double counting in both
  aggregate and daily detail views.
- Existing production source composition in `spend.rs` keeps explicit primary
  and backfill ownership for cc-switch, pi destinations and mcode; the history
  layer performs no second source deduplication.
- Behavioral evidence: alias aggregate, daily detail, idempotent merge,
  replacement and retention tests are present in `spend_history.rs`.

## Comments


## Restart continuation evidence (2026-10-18)

Resume audit: collect_daily now calls compose_pi_days, exercised by a
fixture containing overlapping Claude/Codex destinations, a ledger-owned day
and non-overlapping days. The ledger owns both destinations on overlapping
pi days and contributes once to Claude. mcode now calls the already-tested
merge_missing_days interface rather than duplicating filter/merge orchestration.
Existing alias aggregate/detail and primary/backfill tests continue to pass.
No real log corpus is read; 13 existing real-machine tests remain ignored.
