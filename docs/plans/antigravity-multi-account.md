# Pane Antigravity 多账号：凭据槽 + 并行监控 + 账号切换

> 2026-09-03。目标：同时监控两个 Google AI Pro 账号的 Antigravity 额度，支持轮换。
> 参考：cockpit-tools（D:\code\temp\cockpit-tools，jlcodes99/cockpit-tools）的 Antigravity 切号实现 + ccSwitch 的 OAuthCredentials 模式。

## 背景事实（已实证）

1. Antigravity 无 API key，凭据 = Windows 凭据管理器 `gemini:antigravity` 一条 go-keyring base64 JSON：
   `{token:{access_token, refresh_token, expiry}}`（Pane 的 `antigravity.rs::load_stored_token` 已能读）。
2. OAuth client（IDE 公开值）已在 Pane 代码里：`GOOGLE_CLIENT_ID/SECRET`（antigravity.rs）→ 可独立刷新任何账号的 access token，不依赖 IDE 运行。
3. 额度查询双通道已有：IDE 本地 language server RPC（`RetrieveUserQuotaSummary`）+ Cloud Code API 回落（`try_cloud`，IDE 关着也能查）。
4. 对话历史在本地 `~/.gemini/antigravity/conversations/*.db`（每对话一个 SQLite），切账号不丢；对话/Artifacts 的云端副本绑账号。
5. cockpit-tools 的做法（1326-1653 行，account.rs）：
   - 账号库存 token 束；切号 = 关 Antigravity 进程 → 把账号 token **注入实例 user-data 目录**（`inject_account_to_profile`）→ 重启 IDE；"双切不重启"模式热注入。
   - **自动切号**：按模型分组（Claude/Gemini…）监控剩余百分比，低于阈值 → 从候选中挑平均额度最高的账号自动切换（含防抖、冷却、广播）。
   - 多实例并行：每实例独立 user-data 目录，可不同账号同时跑。

## Pane 方案（两档）

### 档一：凭据槽 + 并行监控（推荐先做，只读、零风险）

- 存储：`%APPDATA%\Pane\antigravity-accounts.json`，`[{label, token:{access,refresh,expiry}, captured_at}]`。
- 命令：
  - `antigravity_capture`：凭据管理器当前 token → 存为新槽（label 用户填）。工作流 = IDE 登录 A → 捕获 A；切登录 B → 捕获 B。
  - 槽刷新：每槽用自身 refresh_token + GOOGLE_CLIENT_ID/SECRET 刷 access token（复用 oauth/antigravity 的刷新形态），走 `try_cloud` 的 Cloud Code 查询。
- 卡片：`antigravity@<指纹>` 多账号卡片，完全复用 kimi 多账号的 UI（合并卡/账号 tab/健康状态点/趋势）；主卡 = 当前登录账号（凭据管理器实时读取，语义不变）。
- catalog：antigravity 开 `supportsExtraAccounts`（多账号机制对它特化：槽而非 accounts/ 目录，family 卡仍走原生查询）。

### 档二：账号切换（写系统凭据，参照 cockpit）

- `antigravity_activate(slot)`：把槽 token 束写回凭据管理器 → 用户重启 Antigravity 即以该账号工作。
- 进阶（cockpit 完全体）：多实例目录注入 + 自动切号（额度阈值触发）。建议等档一稳定后另行评估。

## 不做

- 不做 IDE language server 的多实例扫描（只对"当前登录账号"有意义）。
- 不动对话存储；不做对话管理。

## 验收口径

- 捕获 A/B 两个账号后，Pane 同时显示两张卡，各自独立刷新与状态点。
- IDE 关闭时槽卡仍能查询（Cloud Code 回落）。
- refresh token 失效时该槽卡明确提示重新捕获，不影响其他槽。
