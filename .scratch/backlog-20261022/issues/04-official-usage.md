# 04 — Official WorkBuddy usage (#31)

Status: S0/S1 done; S2/S3 partial (official request row on the WorkBuddy CN card; daily trend/filter pending)
Blocked by: verifiable WorkBuddy account

## S0 — probe

- One script, redacted output only: HTTP status, business code, field names,
  row counts, min/max request time, continuation advance, boundary duplicates.
- CN/AI; personal/enterprise header matrix; pageSize 1/3000; one continuation.
- Never print tokens, cookies or row values.

## S1 — pure module

- Sanitize, parse, sweep, merge, aggregate; fixtures only; no network, UI or
  ledger write.
- Adopt the sweep rules: full page -> advance `startTime` to newest timestamp;
  short page -> done; no advance -> `Stalled`; 100-round cap; dedupe on
  `(requestId, requestTs)`.
- Keep official rows out of `record_meter_samples`; never sum with
  `credit_history` snapshots.

## S2+ — gated

- Absolute-host POST, cache, command and UI only after S0 passes.

## Acceptance

- Probe exists and is safe to share.
- S1 tests cover pagination stall, page tail overlap, malformed timestamps,
  negative credit, dedupe and region isolation.
- No UI or ledger change before S0.

## Comments

- 2026-10-22: checked the local Pane WorkBuddy CN store — one metadata
  row, but no `token:login/workbuddy@...` in the OS vault (only tokenrhythm
  login tokens exist), so the real probe still cannot run. The probe script
  passed a local mock self-test (counts/field names/timestamps, no token or
  row payload). S0 remains blocked on a usable account/reauth.
- 2026-10-23: after a fresh web login, S0 passed against the real CN
  endpoint: 200 / code 0, 26 requests in the 30-day window
  (2026-10-12 21:44 → 2026-10-22 17:25), `pageSize=1` reports `total=26`
  with one row, and a continuation returns one boundary row without
  advancing. Fields match the port plan. The token currently sits in the
  plaintext `login-accounts/workbuddy.json` fallback (no OS-vault entry);
  that is a separate vault-write issue to investigate. S2/S3 is next; UI and
  ledger remain untouched.

- 2026-10-24：S0 复测通过（用本机 WorkBuddy CN 登录态，隐私输出）：30 天 26 条、`code 0`、字段 `requestId/credit/model/client/requestTime/inputTrunc/input/agentPurpose/appType`；`pageSize=3000` 返回全量，`pageNum=2` 只回一条边界行且不前进，确认必须用 `startTime` 推进。基于已落地的 `workbuddy_official_usage` 纯模块，S2/S3 接了一张只读文本行：`Official requests (30d)`，显示请求数与 credits 合计，不写 `record_meter_samples`、不与 `credit_history` 相加。卡片抓取失败时静默省略该行，不影响 credits 主表。真实 UI 视觉验收待 owner；账号筛选与每日积分趋势仍未做。
