# 02 · WorkBuddy card details (#31)

Status: done — implemented 2026-10-28; owner visual acceptance blocked on a valid WorkBuddy CN login | Priority: P2 | Depends: none

## Result (2026-10-28)

- `WorkBuddyUsage` is now a structured snapshot field (`requests`, `credits`, `partial`, `by_day`, newest 20 rows); the credits meter is untouched.
- The card gets a native `<details>` disclosure: daily credits for the last 14 days and the newest 10 requests (time / model / client / credits), with a `partial` badge.
- Missing model/client renders as `—`; no fabricated zeros; no HTTP-default change.
- Tests: `official_usage_snapshot_keeps_sorted_days_and_rows` plus the existing parser/aggregation suite pass; Node 254/254; build and site 55/55.
- Not visually verified yet: the current local WorkBuddy credential returns 401, so the owner needs to sign in once and open the card disclosure.

## Deep-review fixes (2026-10-28)

- The four-column header no longer squeezes "credits" into the model track; it is a flex title/unit row with a localized `wb.credits` unit.
- Out-of-range timestamps are filtered from the daily chart instead of rendering a "wn" label.

## Build

- Extend `OfficialUsage` in `providers/codebuddy.rs` with the already-computed `by_day` map and a bounded latest-rows list (reuse `MAX_DETAIL_ROWS`).
- Expose the new fields through the snapshot/detail contract without writing them into the credits ledger or HTTP defaults.
- Card UI: a "Show more" disclosure that renders a compact daily trend and the latest N requests (time, model, client, credits); no account filter/virtualization until real data needs it.
- Empty/partial states: missing model/client stays absent; `partial` remains visible; no fabricated 0.
- Tests for parsing/aggregation/detail rows and the TS type mapping.

## Done when

- With a real WorkBuddy CN account, the card expands and shows daily credits plus recent requests matching the official ledger.
- Without an account, empty state remains empty, not 0.
- `pnpm build`, Node tests and parse-tests green; owner visual acceptance.

## Out of scope

- Full four-layer WorkBuddy panel, account filter, pagination UI, AI-tier.
