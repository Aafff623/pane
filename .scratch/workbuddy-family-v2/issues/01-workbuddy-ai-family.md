# 01 — WorkBuddy AI 国际档（全链路）

**Blocked by:** None — can start immediately.
**Status:** done（代码层；真实扫码待 owner）
**Issue:** #8（实施）

**What to build:** 在 `codebuddy_login::FAMILIES` 增加 `workbuddyai` 一行（`display_name = "WorkBuddy AI"`、`api_endpoint = https://www.workbuddy.ai`、`platform = workbuddy-ai`），并打通目录四层、登录命令、授权中心、图标/i18n。

**范围**
- Rust catalog + 前端镜像各一条；`LOGIN_FAMILIES` 纳入；三个 `<family>_login_start/poll/cancel` 命令注册并在前端 `BROWSER_LOGIN_PROVIDERS` 纳入。
- 国际档 billing 路径：先 `/billing/meter/...`，**仅 HTTP 404** 回落 `/v2/billing/meter/...`。
- 国际档 refresh 头：`X-Auth-Refresh-Source: plugin`。

**不做**：不改国内档已有卡 ID / 历史。

**验收**
- [ ] `workbuddyai` 出现在设置授权中心、可搜索、有独立图标
- [ ] 扫码 start/poll/cancel 三条命令可用
- [ ] `catalog_consistency` 两个门禁绿
- [ ] 解析测试覆盖 billing 路径候选顺序
- [ ] 真实扫码由 owner 验收（本机无国际档账号）

## Comments

- 2026-10-19 出自 `.scratch/workbuddy-family-v2/spec.md` D1；上游 `variant.rs:14-31,160-210`。
- 2026-10-19 已实现：`FAMILIES` 加 `workbuddyai` 行（含 `refresh_source` /
  `bare_billing_prefix` / `intl` / `local_auth_file` / `data_root`）；
  Rust + TS catalog、`LOGIN_FAMILIES`、`AUTH_FAMILIES`、三条 Tauri 命令、
  前端 `BROWSER_LOGIN_PROVIDERS`、机制说明全部纳入。`parse-tests` 898 通过
  / 0 失败。真实扫码需要 owner 用国际版账号验收。
