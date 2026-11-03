# Spec: ClinePass 订阅到期标注

**Status:** implemented 2026-11-03 — 实机验证通过，待随版本发布 · **Owner:** Aafff623

## 现象

ClinePass 卡片只有三个配额窗口（5h / 周 / 月）和它们的重置倒计时，看不到订阅本身何时到期。owner 明确要求：能查到的过期字段都要标出来。

## 证据（2026-11-03 用真实 key 实测）

- `GET https://api.cline.bot/api/v1/users/me/plan`，`Authorization: Bearer sk_…` 直接可用（不需要 OAuth）：
  ```json
  {"data":{
    "plan":{"displayName":"Cline Pass (Monthly)","interval":"Monthly","isActive":true,...},
    "subscriptionId":"sub_…",
    "currentPeriodStart":"2026-10-04T04:48:20Z",
    "currentPeriodEnd":"2026-11-04T04:48:20Z",
    "cancelAt":"2026-11-04T04:48:20Z",
    "canceledAt":"2026-10-04T04:48:37Z"
  },"success":true}
  ```
- 语义：`canceledAt` 非空 = 已取消续费；`cancelAt` = 服务实际结束时间；`currentPeriodEnd` = 本期结束（自动续费时即续费日）。`/users/me/plan` 404 = 没有当前订阅，402 = 订阅未付费（Cline 客户端源码 `sdk/packages/core/src/account/types.ts` 的 `UserCurrentPlan` 与 `account-service.ts` 的 `fetchClinePassSubscribedRPC` 同口径）。
- API key 本身没有过期字段（`/api/v1/api-keys` 无 expiresAt），到期只存在于订阅层。

## 决定

1. `clinepass.rs` 每次刷新额外请求一次 `/users/me/plan`（每个账号各自请求，失败不影响配额卡）。
2. 卡片第一行新增文本行 `Subscription`：
   - 已取消：`Cline Pass (Monthly) · ends 2026-11-04 12:48 (cancelled)`
   - 自动续费：`… · renews 2026-11-04 12:48`
   - 已过期：`… · expired …`
   - 402：`ClinePass subscription is unpaid`
   - 404 / 网络失败：不显示该行（不猜）。
3. 时间按本机时区显示（`chrono::Local`），分钟精度。
4. 文案参与 i18n：`Subscription` 标签三语（订阅 / Подписка）；日期后的状态词沿用供应商文本的英文口径（与 `14% used` 一致）。

## 边界

- 只覆盖 ClinePass；其它 provider 的到期字段按需另开票。
- 默认账号实测：`/api-keys` 200（key 有效）、`/plan` 与 `/usage-limits` 均 404 —— 该 Cline 账号没有 ClinePass 订阅。卡片改为 `needs_setup` 并提示「This Cline account has no ClinePass subscription」，不再显示 404 错误与旧缓存。

## 验证

- 单测：取消订阅 / 自动续费 / 无订阅 / 未付费 四条路径。
- 实机（新构建，停掉安装版后运行）：6 个账号里 5 个显示订阅行，其中 4 个 `ends 2026-11-04 … (cancelled)`、1 个 `renews 2026-11-08 …`；默认 key 因 404 无行。
- `parse-tests` 全绿（含新增 4 条），前端构建与三语对齐通过。
