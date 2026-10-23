# Provider 接入引导重构（按 Magpie 对标）

**Status:** ready-for-agent
**Feature dir:** `.scratch/provider-onboarding/`
**Date:** 2026-10-19
**上游分析:** `temp/magpie-analysis/`（5 份，本地只读）
**Pane 现状审计:** `temp/pane-analysis/01-provider-credential-matrix.md`

---

## 1. 问题

Pane 的 Customize ⚙ 面板对「怎么接入」给错动作：

- 12 个登录型 family 点「添加账号」弹出的是 **paste API key** 对话框（`main.ts:8678 → 10769 → 7969`），其中 11 个会把 key 真写入 `accounts/<family>.json`，刷新时得到 `unknown provider` 错误卡。
- 设置页对 9 个浏览器/扫码登录 family 调的是 `oauth_start`，后端只支持 codex/grok/copilot，直接报错。
- CodeBuddy 四兄弟在 `PROVIDER_CRED_INFO` 中缺失，行内「?」显示「无需凭据」。
- 21 个非 key family 没有 `customize.loginHint`；其中 10 个只看到一句通用文案。
- `stepfun-plan` 的专用账号密码面板被多账号分支遮蔽，永远不可达。
- `get_credential_status` 对登录 family 读错 store，已登录仍显示「未保存」。

## 2. 目标

每个 provider 的配置面板必须做到：

1. 先告诉用户「这个 provider 靠什么接入」（API key / 扫码 / 浏览器登录 / 设备码 / 读本机登录 / Cookie / 云凭据）。
2. 只给**真实可用**的动作按钮；不出现 paste-key 对话框给没有 key 的 provider。
3. 每个动作指向真实命令（`<family>_login_start` / `oauth_start` / `cursor_oauth` / `antigravity_capture` / `workbuddy_import_desktop` / `tokenrhythm_login` 等）。
4. 登录型 provider 给出分步指引（打开哪个页面、扫什么码、完成后回来）。
5. 保持现有 `mechanismFactsHtml` / credential chips；Magpie 只补「动作 + 步骤 + 风险提示」，不反向覆盖 Pane 更强的额度来源展示。

## 3. 设计

### D1 — 单一接入描述表（前端）

新增 `src/providerOnboarding.ts`：

```ts
type CredentialKind = "apiKey" | "oauthDevice" | "oauthBrowser" | "qrScan"
  | "localRead" | "cookie" | "cloud" | "none";
type AccountAction = "addLogin" | "pasteKey" | "captureSlots" | "importAccounts"
  | "importDesktop" | "importCookie" | "openAuthCenter" | "pasteKeyPage";

interface Onboarding {
  kind: CredentialKind;
  actions: AccountAction[];
  hintKey: string;       // i18n：一句话说明它靠什么接入
  stepsKey?: string;     // i18n：2-3 步操作指引（登录型必需）
  docsKey?: string;
}
```

`onboardingFor(family)` = 默认（由 catalog flags + 已知集合推导）+ 显式覆盖。所有 70 个 family 必须有条目；登录型 family 的 `actions` 不得包含 `pasteKey`。

### D2 — Customize 面板按描述表渲染

`renderCustConfig` 的第一个分支改为 `onboardingFor(fam)`：

- `addLogin` / `openAuthCenter`：走 `panelIntent` 打开授权中心并定位该 family（复用已验证的跨窗口机制，不新增 IPC）。
- `pasteKey`：保留现有单 key 表单 / 账号对话框（仅 key 型 provider）。
- `captureSlots`：antigravity 捕获。
- `importAccounts`：cursor 三栏对话框。
- `importDesktop`：`workbuddy_import_desktop`。
- `importCookie`：workbuddy 的 web 会话区块；tokenrhythm 跳设置页登录按钮。
- `pasteKeyPage`：bocha/tavily/firecrawl/brave 跳「密钥与额度」页（它们的 key 在 `search-keys.json`）。

### D3 — 设置页登录分发

`startOauthLogin` 只用于 `OAUTH_PROVIDERS`（codex/grok/copilot）。其余登录型 family 一律打开授权中心并定位 family；禁止再对它们调 `oauth_start`。

### D4 — 凭据状态读对 store

`get_credential_status`：`login_accounts::takes_login_accounts(family)` 为真时读 `login-accounts/<family>.json`，否则读 `accounts/<family>.json`。

### D5 — 文案

新增 `customize.loginHint.<family>`（10 个通用提示家族 + CodeBuddy 四兄弟）与 `customize.onboarding.<family>` 分步指引（登录型）。三语齐平。

### D6 — 不做

- 不改后端认证实现（Qoder CN 多账号另立票）。
- 不引入 Magpie 的插件运行时、`migrations.json`、风险确认弹窗（本轮不做）。
- 不改 catalog 的 `supports_api_key` 语义（搜索类 provider 的 key 在 `search-keys.json`，用 `pasteKeyPage` 表达）。

## 4. 工单

| # | 内容 |
|---|---|
| 01 | `providerOnboarding.ts` 描述表 + 类型 |
| 02 | Customize 面板按描述表渲染，修掉 12 个 paste-key 误路由 |
| 03 | 设置页登录分发 + `get_credential_status` store 修正 + stepfun-plan 分支顺序 |
| 04 | i18n：CodeBuddy 家族 + 10 个通用家族 hint，登录型 steps（三语） |
| 05 | Qoder CN 多账号（device flow，参照 Magpie）——blocked：需真实 CN 账号验收 |
| 06 | 验证：tsc/vite/node tests/cargo check + owner 手工清单 |

## 5. 验收

1. codex/copilot/grok/codebuddy/codebuddycn/workbuddy/workbuddyai/kiro/windsurf/trae/qoder/zed 的「添加账号」不再出现 API key 输入框。
2. 设置页对这些 family 点登录不再报 “OAuth sign-in is not available”。
3. CodeBuddy 四兄弟的「?」不再显示「无需凭据」。
4. stepfun-plan 能看到用户名/密码/Oasis token 面板。
5. 已登录的 CodeBuddy/WorkBuddy 账号在 chips 里显示已保存。
6. `tsc` + `vite build` + `node --test` + `cargo check` 全绿。

## 6. 已知限制

- 真实扫码/OAuth 需要 owner 用各账号验收；本轮只保证按钮指向正确命令。
- Qoder CN 多账号需要新 device flow，未在本轮实施。

## 7. 2026-10-23 update

- WorkBuddy / WorkBuddy AI 的桌面端登录导入已移除：授权中心和 Customize
  不再显示 `importDesktop`，后端 `workbuddy_import_desktop` 命令与相关
  三语文案一并删除。账号创建只走网页/扫码登录回调。
- `workbuddy_local` 仍保留给卡片读取桌面端的只读 usage fallback；不再
  复制桌面会话到 Pane 的账号库。
