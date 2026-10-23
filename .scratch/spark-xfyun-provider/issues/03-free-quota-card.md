# 03: free-quota 额度快照 + 卡片

**What to build:** 用已保存的网页会话查询 `free-quota?channel=spark-x2.6`，解析总量、已用、剩余、状态和到期，卡片显示免费额度进度、已用/总量、剩余、到期；`claim=null` 显示未领取；`ACTIVE` / `EXHAUSTED` / `EXPIRED` 正确表达；会话过期引导重新登录。首次真实登录时抓一次响应并校准结构。

**Blocked by:** 02

**Status:** done (real response calibration pending owner login)

## 要点

- 响应信封 `{code, message, data}`；`code:4001` 是会话失效，不能只看 HTTP 200。
- `data.claim` 可能为 `null`；已领取时读 `data.claim.freeQuota` 的 `totalTokens`、`usedTokens`、`remainingTokens`、`quotaStatus`、`expiresAt`。
- 数字可能是 JSON number 或字符串；`expiresAt` 精确格式待首登校准，解析失败不要伪造日期。
- 不把 10 亿写死为卡片值；服务端返回的总量是权威。10 亿/31 天只是控制台领取文案。
- 主进度条用「免费额度」，`used_percent = used / total`，detail 用「已用 X / 共 Y tokens」；另给「剩余」「到期」文本行。不要用 `resets_at` 的 “Resets in …” 文案表达一次性到期。
- `claim=null` → `needs_setup`，提示去控制台领取；`EXHAUSTED` / `EXPIRED` → 状态文本 + 剩余 0，不显示正常百分比。
- 刷新沿用现有 provider 刷新循环；额度取数失败保留原因，不把失败显示成 0；不影响其他 provider/账号。
- 第一张 provider 票必须完成一次真实登录并记录实际响应；未知结构按 `待确认` 诚实失败，不猜数字。

## 验收

- [ ] 卡片数字与网页控制台一致（10 亿总量、已用、剩余、到期；需 owner 首登校准真实响应）。
- [x] `claim=null`、`EXHAUSTED`、`EXPIRED`、`code:4001` 四种状态都有明确 UI（`needs_setup` / 状态行 / 重登提示）。
- [x] 解析器测试覆盖 number/string、缺字段、未知 `quotaStatus`、不可解析日期（7 个 xfyun 单测）。
- [x] 额度刷新失败保留原因，不显示假 0；不影响其他 provider。
- [ ] token/cookie 不入日志或跟踪文件；parse-tests、tsc、vite、i18n 门禁全绿；owner 实机验收。

## Comments

- 2026-10-23：`free-quota` 防御式解析已实现：claim null → `needs_setup`；数字/数字字符串兼容；used/remaining 互推；未知状态不用 ACTIVE 百分比；expiresAt 支持 epoch/ISO，解析失败显示 Unconfirmed；`code:4001`/401/403 → 重新登录。真实响应结构仍需 owner 首登后校准（当前字段来自控制台前端证据）。
