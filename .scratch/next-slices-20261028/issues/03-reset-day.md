# 03 · resetDay billing cycle (#104-4)

Status: ready | Priority: P2 | Depends: none

## Build

- Add config `resetDay` (1–28) with a default that preserves today's behaviour (1 or 30-day approximation as appropriate).
- Replace the 30-day approximations in `clinepass`, `copilot` and `stepfun_plan` with a shared billing-cycle helper; keep `opencode`'s month-end clamp behaviour.
- Settings → general control + i18n (en/zh/ru); no change when the user never opens it.
- Tests: month-end clamp, leap February, 1–28 bounds, DST-free UTC arithmetic, old configs unchanged.

## Done when

- Monthly reset/expense extrapolation follows the configured day, not a hardcoded 30 days.
- Existing configs and providers without monthly windows are byte-identical.
- `pnpm build`, Node tests and parse-tests green; owner acceptance for the new setting.

## Out of scope

- Changing vendor reset timestamps, invoices, or historical amounts.
