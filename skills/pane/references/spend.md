# Spend panel

The spend panel scans **local CLI session logs** — Pane never asks a provider for
per-token data on subscription plans (server-side only exposes window
percentages there).

## Wording that matters

- **Tokens are the primary fact.** A model with no known price keeps its measured
  tokens and is marked `unpriced` (⚠) — unknown prices never erase tokens.
- Dollars come from a shared pricing catalog (LiteLLM / models.dev) unless the
  provider itself reports cost.
- Subscription "value" shown in dollars is an equivalent list-price figure, not
  cash spent — say so when the user is surprised by the total.

## Why a day or a tool can read 0

- That tool's logs were not found on this machine (`fetch_spend_sources` /
  `/v1/sources` reports detected vs absent).
- The tool stores usage somewhere Pane does not scan.
- A relay zeroed the CLI logs — for Claude, Pane switched to the cc-switch ledger
  as primary because relays under-counted the CLI logs by 7–24×.

## Diagnosing

1. Open the total-spend bar's detail icon → the heatmap; click a day for
   per-tool/per-model cards.
2. Compare the day against the tool actually used that day.
3. If a source is missing entirely, report it as **unknown coverage**, not as
   "zero usage".
