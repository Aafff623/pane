# 14 - Pricing policy

Type: task
Status: resolved
Blocked by: 13
Coverage: W12

## What to build

Give shared priority/model pricing rules one owner. Test actual resolution via both pricing and spend interfaces, including affected multipliers and unknown models.

Authority: ../spec.md. Retain existing refresh-ux/trend-units product requirements.

## Acceptance

- [x] Trace all production, persisted and external consumers at this seam.
- [x] Record a behavioral red-before-green check, or contract proof for a justified retained abstraction.
- [x] Implement the complete vertical change; remove replaced duplicate policy.
- [x] Run targeted checks and affected integration/build checks.
- [x] Standards review, Spec review and simplification scan; fix findings and re-test.
- [x] Record exact evidence and any runtime/owner acceptance still open.

## Evidence

- Added `pricing::priority_multiplier` as the single owner for Codex priority
  policy, including Kimi/Moonshot exclusion, published 2.5x rows, default 2x
  and live supplement multipliers. `spend.rs` now consumes this interface;
  its duplicate table was removed.
- Pricing resolution for `-priority` slugs calls the same policy, so spend
  events and direct model lookup cannot drift. Existing pricing tests cover
  published and unknown-model cases.

## Comments


## Restart continuation evidence (2026-10-18)

Resume audit fixed two regressions in the earlier consolidation: resolve
already owns the catalog mutex, so the shared priority policy now accepts a
lazy supplement reader rather than locking the global catalog again. Spend
supplies probe_fast_multiplier, preserving cache revalidation dependencies.
A held-lock catalog regression checks an unknown model with a 3x supplement;
published fixed tiers never consult that fallback. CORRECTIONS_REV is now 14
so old pre-priced totals cannot survive the policy change. Removed the unused
public wrapper and obsolete keep-two-tables-in-sync comment.
