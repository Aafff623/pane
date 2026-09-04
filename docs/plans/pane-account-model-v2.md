# Pane 账号模型 v2：趋势图全卡片 + 账号化凭据 + OAuth 扩容

> 2026-09-03，基于用户验收反馈的四个问题 + ccSwitch 调研结论。
> 原则：照搬 ccSwitch 的设计思想（device flow OAuth、多账号 + 默认账号、瞬时额度本地累积成历史），不引入 SQLite / QuickJS 脚本平台。

## 调研结论（事实基础）

1. **趋势图数据现状**：`spend.rs::collect()` 只扫本地 CLI 会话日志（`~/.kimi-code/sessions` 等），按 id 精确匹配产出 `ProviderSpend`。API key 账号（`kimi@<fp>`）在本地没有日志 → spend 侧无条目 → `defaultProviderLayout`/`ensureLayout` 不给账号卡加 TREND_KEY → 账号卡无趋势图。渲染分支本身是通用的"无数据不渲染"。
2. **额度接口只给瞬时值**：ccSwitch 三条查询路径（balance/coding_plan/subscription）全部只返回当前窗口状态，无历史序列。ccSwitch 的趋势图来自本地 SQLite 逐请求落库 + 会话文件解析。→ Pane 要给账号卡补趋势，必须**本地累积快照历史**。
3. **账号卡数值不是 bug**：实测 6736 端口数据，kimi99 的重置时间与主卡完全不同（各自 key 独立查询成功）；kimi199 与主卡逐秒一致是因为用户把主 key（kimi.json）又添加了一遍当账号——同一把 key 的窗口本来就相同。"全员每周已达上限"是这些共享 key 的真实状态。
4. **ccSwitch OAuth 没有 localhost 回调/URL scheme**：codex（OpenAI 私有 device flow + 服务端 code_verifier）、copilot（标准 GitHub device flow）、xai（OIDC discovery + device flow）全是 device code；`ccswitch://` deep link 只用于配置导入。Pane 已为 codex/grok 复刻同款（oauth.rs）。真正缺的是 **copilot 的 GitHub device flow 托管登录**。
5. **获取 API Key 链接**：ccSwitch 预设字段 `websiteUrl` + `apiKeyUrl`，表单内渲染"获取 API Key"外链。

## 分期实现

### Phase 1 — 额度快照历史 + 全卡片趋势图

- 新增 `src-tauri/src/usage_history.rs`：`%APPDATA%\Pane\usage_history.json`，结构 `{ version:1, entries: { <card_id>: { day: "YYYY-MM-DD", used: f64 }[] } }`，每 id 保留最近 35 天。
  - 采样规则：每次成功快照落 last_snapshots 时同步记录；`used` = 该卡所有 progress 指标 used_percent 的最大值（Session 会滚动清零，Weekly 才是真实消耗水位，取 max 免去挑指标的脆弱性）。
  - 同一天多次刷新 = 覆盖为当天最大值（日粒度，与现有 30 天趋势轴对齐）。
  - 纯函数拆出 `record_sample` / `prune` / `trend_for`（parse-tests 可测）。
- 前端：`fetch_usage_history` 命令返回 `{ id: number[] }`；`refresh()` 里与 fetch_spend 并行拉取。
- 渲染合并：`renderCard` 取 spend 时，本地日志 spend 优先；没有则用历史趋势合成一个 `{id, trend}` 兜底（悬浮提示文案区分"本地 token 消耗"与"额度已用 % 历史"）。`defaultProviderLayout`/`ensureLayout` 的 TREND_KEY 判断同步使用兜底后的数据源。
- 效果：所有有至少一次成功查询的卡（含账号卡、onenewapi 卡）都有趋势；纯本地日志卡（claude/codex）行为不变。

### Phase 2 — 齿轮面板账号化重构 + 设置页瘦身

**账号化模型（对齐 ccSwitch 的 accounts + default_account_id）**：
- 多账号 family（deepseek/kimi/stepfun/siliconflow/novita/relaybalance）的所有 key 统一活在 `accounts/<family>.json`；**首条 = 默认账号**。
- 默认账号的快照以 family id（如 `kimi`）发布 → 渲染为主卡（布局/托盘连续性）；其余账号照旧 `kimi@<fp>` 独立卡。同一把 key 只存在一份，天然消除"主卡 + 账号卡重复显示同一 key"。
- 迁移（启动时一次性、保留原文件可回滚）：若 family 主 key 文件存在且指纹不在账号列表 → 导入为首条（label "默认"）；若已在列表（现在的 kimi199 场景）→ 跳过导入，主 key 文件不再参与查询。
- 账号行新增「设为默认」（★，移到首位）操作；删除/备注逻辑不变。
- 齿轮面板（多账号 family）最终内容：OAuth 块（仅 codex/grok 类）+ 「获取 API Key ↗」外链 + 「添加账号」+ 账号列表。**删除主 key 输入/测试/保存行**（与添加账号弹窗重复）。
- 添加账号弹窗流程不变（key → 备注 → 测试 → 保存）。

**设置（⚙）与凭据说明（？）拆分**：
- ⚙ = 纯配置动作：OAuth 登录/登出、添加账号、账号列表、（单 key family 保留的）key 字段、获取 API Key 链接。
- ？= 说明文档：自动读取哪些凭据（auto 事实）、支持哪些登录方式（methods）、实时检测 chips、Pane 已存凭据列表、One/New API 卡的账号归并说明。
- One/New API 站点管理从设置页搬进 onenewapi family 行的 ⚙ 面板（站点列表 + 添加表单整体迁移，逻辑不变）。

**设置页（index.html）删除三个区块**：`settings.apiKeys`（15 个静态 key 表单，与齿轮面板完全重复）、`settings.relayBalance`（base_url+key 已由添加账号弹窗覆盖）、`settings.onenewapi`（已搬迁）。`saveApiKey()` 及其静态输入引用随之清理。

**获取 API Key 链接**：`providerCatalog.ts` 增加 `apiKeyUrl` 字段（deepseek=platform.deepseek.com/api_keys、kimi=platform.moonshot.cn/apikeys、stepfun/siliconflow/novita=各自控制台 key 页），齿轮面板与 ？ 面板渲染外链（走 open_link 门控）。

### Phase 3 — Copilot 托管 OAuth（GitHub device flow）+ 能力矩阵

- oauth.rs 增加 copilot：GitHub device flow（client_id `Iv1.*`，scope `read:user`）→ github_token → copilot_internal/user 查询（copilot.rs 优先托管 token）；存储走现有 StoredTokens 结构（access_token + refresh_token 同存 GitHub token，365 天假 expiry）。
- copilot.rs 查询路径支持托管 token 优先、本地 CLI 凭据回落。
- 前端 OAUTH_PROVIDERS 加 copilot；OAuth 块复用现有 start/poll/logout 流程；get_credential_status 显示 copilot OAuth label。
- 能力矩阵落地：providerCatalog.ts 增加 `supportsOAuth` 元数据；docs/providers.md 更新「认证方式 × 查询方式 × 趋势」矩阵表。
- 明确不做的：Kimi/Claude/Gemini 托管 OAuth（无公开 device 端点或需写 CLI 凭据，ccSwitch 也不做，保持 CLI 凭据只读回落）。

### Phase 4 — 验证与交付

- `npm run build`、`cargo check --tests`、parse-tests（新增 history 聚合与迁移逻辑测试）全绿。
- 不提交不推送；输出分级验收报告 + 用户点击验收指南。

## 风险与兼容

- 迁移只增不删：主 key 文件保留在原处，回滚 = 还原 lib.rs 查询分支。
- 账号卡 id 稳定性：默认账号以 family id 发布后，原 `@fp` 卡从布局消失由 reconcileAccountLayout 清理（已有机制）。
- 历史文件为新增，不碰 spend_cache.json / last_snapshots.json 格式。
