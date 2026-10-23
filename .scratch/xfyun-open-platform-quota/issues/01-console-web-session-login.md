# 01: `xfyun-speech` family + console 网页登录与多账号

**What to build:** 在 Pane 里新增第三个 web-session provider「讯飞开放平台语音」（family 建议 `xfyun-speech`），用 Pane 自己的隔离 WebView2 登录 `console.xfyun.cn`，用轻量 dashboard 接口确认登录态，按稳定 uid 去重存入现有 `login_accounts` + `secretstore` 路径。本票不含取数逻辑。

**Blocked by:** 无（复用 #38 已落地的通用 web-session 机制）

**Status:** ready-for-agent

## 要点

- 接线面照 #38 那一轮：`provider_catalog.rs`（`ProviderDefinition` + `WEB_SESSION_FAMILIES`）、`providers/mod.rs`、`provider_dispatch.rs`、`lib.rs` 的 web-session 命令面与 snapshot 路由、`auth_center.rs`、`login_accounts.rs`；前端 `providerCatalog.ts`（`webSession: true`）、`providerMechanisms.ts`、`providerOnboarding.ts`、`providerVisuals.ts`、图标 svg、`src/i18n.ts` 三字典。
- `query_kind` 用 `NativeSnapshot`（一卡多行指标，不是单端点余额）；`supports_api_key: false`；`supports_extra_accounts: true`；`primary_unit` 由实现者定（建议 `Requests`）。
- 登录判定必须走接口，不能「有 cookie 就算登录」：`/dashboard/user/profile/get` 或 `/dashboard/user/balance`，要求 `flag == true && code == 0`。实测未登录时 **HTTP 仍是 200**，只有 body `code=80000`。
- 稳定账号 id：`account_id` cookie 的值即纯数字 uid（与 `message/count?receiver=<uid>` 一致）；备选 `/dashboard/user/certification/get` 的 `data.uid`。**只取 uid**，同响应里的 `idName/idNo/mobile` 属 PII，不落库、不进日志、不进快照。
- 会话只从 Pane 自己的 incognito WebView 取（`platform::read_webview_cookies`），不读浏览器 profile 的 SQLite；实测最小可用凭据是 `ssoSessionId` 一个 cookie，存整包 jar 也可。
- 新 family 会落进 `default_disabled_families()`，保持默认关闭。
- 登录窗口关闭、超时、取消不得留僵尸窗口或半条会话（对照 `xfyun.rs` 的处理）。
- `WEB_SESSION_FAMILIES` 仍是硬编码列表；要不要改成 catalog 的 `web_session` 字段由实现者权衡，本票不强制。

## 验收

- [ ] 设置行有「网页登录 / 重新登录 / 清除会话」，登录窗口打开 `console.xfyun.cn` 并由 owner 手动登录。
- [ ] 登录成功后账号出现在认证中心，重启 Pane 会话仍有效。
- [ ] 未登录/过期时不产生任何「额度为 0」的展示，走 `no_credentials` 语义与重新登录提示。
- [ ] 同一 uid 重复登录只更新该账号，不新增卡片；两个不同 uid 各自独立。
- [ ] cookie 与 uid 不出现在日志、Git 跟踪文件、公开状态接口里（可用一次 grep 自查）。
- [ ] `node --test tests/*.test.mjs`、i18n 配对、catalog 一致性门禁全绿。

## Comments

- 2026-10-23：接口与登录态判定已在真实登录态实测，证据见 `temp/xfyun-console-quota-analysis-20261023.md`。owner 已完成个人实名认证并领取全部五个语音资源包，登录即可校准。
