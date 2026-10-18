# 07 - Usage units and meter history

Type: task
Status: resolved
Blocked by: 06
Coverage: A7

## What to build

Implement structured machine-readable unit/amount semantics and trend-units existing decisions. Keep per-vendor credits separate, preserve old history via tested migration, and remove display-text parsing.

Authority: ../spec.md. Retain existing refresh-ux/trend-units product requirements.

## Acceptance

- [x] Trace all production, persisted and external consumers at this seam.
- [x] Record a behavioral red-before-green check, or contract proof for a justified retained abstraction.
- [x] Implement the complete vertical change; remove replaced duplicate policy.
- [x] Run targeted checks and affected integration/build checks.
- [x] Standards review, Spec review and simplification scan; fix findings and re-test.
- [x] Record exact evidence and any runtime/owner acceptance still open.

## Evidence

RED: a structured Credits reading with arbitrary display copy returned None from the label/detail parser; a U+0085-bearing recovery fixture and an unknown-version history file both behaved as documented gaps. GREEN: usage_units.rs owns UsageUnit/UsageReading/UsageDays; Metric carries optional structured usage/daily_usage; the ledger stores per-card, per-unit, per-scope, per-shape series (Cumulative vs Daily) and keeps the existing reset-aware day series for cumulative readings. Display-text parsing is deleted (readings_from_metrics replaced credits_used_from_metrics). GIPHY's vendor daily buckets flow verbatim (never differenced); Tavily samples key.usage under a credential-fingerprint scope, never the possibly shared account.plan_usage; Firecrawl/Bocha/Brave are deliberately NOT sampled because their responses are balances or shared pools. Legacy credit_history.json v0/v1 migrates into meters without erasing samples; unknown future versions and unreadable files are preserved and refuse writes. providerCatalog now mirrors a primaryUnit per family with a parse-tests consistency gate. Frontend trendSourceFor/showTrendTip read the typed meter and label it 额度趋势; tooltips carry the unit; the token/dollar panel toggle is unchanged.

Verified: parse-tests 856 passed/13 ignored; frontend 44 passed; tsc + Vite build pass; cargo check --tests passes with pre-existing warnings only.

## Comments


## Review fix (2026-10-18)

Fixed the P2 daily-meter retention gap. `record_daily_samples` now prunes
`UsageShape::Daily` samples with the same `KEEP_DAYS` window as cumulative
meters before persistence. Added a regression covering an expired daily bucket
and an in-window bucket.
