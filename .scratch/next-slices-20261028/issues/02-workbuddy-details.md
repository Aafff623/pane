# 02 · WorkBuddy card details (#31)

Status: ready | Priority: P2 | Depends: none

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
