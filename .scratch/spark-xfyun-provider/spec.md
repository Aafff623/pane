# 讯飞星火（Spark-X2.6）Provider：网页登录态额度与用量

**Status:** implemented on `codex/spark-xfyun-provider` (01–04 code complete; real `free-quota`/usage response and UI acceptance pending owner login)
**GitHub:** [#38](https://github.com/Aafff623/pane/issues/38)
**前置调研:** `temp/xfyun-maas-analysis-20261023.md`

## Problem Statement

Pane 目前没有讯飞星火 / xfyun 的 provider。owner 的账号已完成个人实名认证，拿到 Spark-X2.6 的 10 亿 token 免费额度，希望像现有 provider 一样在 Pane 里看到额度总量、已用、剩余、到期和状态，而不是每次打开网页控制台。

实测确认：

- Spark-X2.6 的三种推理协议（Chat Completions、Responses、Anthropic Messages）都能用 API Key 调用，响应也带标准 `usage`。
- **API Key 只能推理，查不了额度。** 控制台的额度接口统一要求网页登录态；9 种 header 变体和 12 条推理网关路由都试过，结果是 `code:4001` 或 404。
- 控制台前端已经写死 `Spark-X2.6`、`spark-x2.6`、`1e9`（10 亿）和 31 天，并提供了 `free-quota` 查询/领取接口。
- Pane 现在没有任何星火代码；已有的 `relaybalance` 只认 OpenAI dashboard-billing，星火网关对这些路径返回 404，不能复用。

核心矛盾：**用户手里只有 API Key，但额度属于网页账户，不属于 API Key。** Pane 必须自己承载一次网页登录，并安全地保存会话。

## Solution

新增一个 **web-session provider**「讯飞星火」（family 建议 `xfyun`）：

1. 用户在 Pane 自己打开的隔离 WebView2 登录窗口里登录 `maas.xfyun.cn`，不需要手动粘贴 token/cookie。
2. Pane 读取并验证网页会话，存入 OS vault（失败时按现有明文例外约定回退），重启后仍可用。
3. 查询控制台的免费额度接口，卡片展示总量、已用、剩余、到期和状态；`claim=null` 时提示未领取，`code=4001` 时提示会话过期并引导重新登录。
4. 后续票再补最近 7 天用量摘要；MVP 不承诺按模型明细表。

实现前先把当前 `tokenrhythm` 专用的网页登录机制抽成通用 web-session 能力。星火是第二个 web-session provider，正好是把特判收敛成机制的时机；不应在设置、定制、认证中心和登录命令里再复制一套 `family === "..."`。

## User Stories

1. 作为讯飞星火用户，我要在 Pane 卡片上直接看到 10 亿免费额度的总量、已用、剩余和到期，这样不用每次开网页控制台。
2. 作为讯飞星火用户，我要通过 Pane 打开的网页登录窗口完成登录，不用手动复制 API Key、token 或 cookie。
3. 作为讯飞星火用户，我要登录一次后重启 Pane 仍保持会话，不用每天重新登录。
4. 作为讯飞星火用户，我要在会话过期时看到明确的“重新网页登录”提示，而不是一张无解释的错误卡片。
5. 作为尚未领取免费额度的用户，我要看到“未领取”的明确状态和领取入口提示，而不是 0 或空白数字。
6. 作为额度已耗尽或已过期的用户，我要看到 `EXHAUSTED` / `EXPIRED` 状态，而不是把 0% 当成正常额度。
7. 作为多账号用户，我要为每个讯飞账号分别登录、分别显示额度，并能单独重登、归档或删除，不影响其他账号。
8. 作为隐私敏感用户，我要 Pane 不把网页 token/cookie 写进日志、Git 文件或普通状态接口。
9. 作为长期用户，我要卡片沿用 Pane 现有刷新循环、主题和 i18n，不需要为星火单独学习一套交互。
10. 作为维护者，我要新增 web-session provider 时只声明能力并实现取数模块，而不是在多个前端分支里再写一套 `tokenrhythm` 式硬编码。
11. 作为想了解消耗的用户，我要在后续版本看到最近 7 天的用量摘要，并能和网页控制台的用量统计对上。
12. 作为 owner，我要在真实登录后校准一次 `free-quota` 响应结构，确保卡片数字与控制台一致后才算验收通过。

## Implementation Decisions

### Provider 形态

- 新增 family `xfyun`，显示名「讯飞星火」，`primary_unit` 为 tokens，`query_kind` 采用余额/额度型（`NativeBalance` 语义：读一个额度端点），`supports_api_key: false`，`supports_extra_accounts: true`。
- 不提供 API Key 输入槽。API Key 只用于推理，Pane 不代理推理流量；把 API Key 放进 Pane 会产生“看起来能配、实际查不到额度”的误导。
- 卡片进入现有 provider 列表、Settings、Customize、认证中心和多账号体系；图标/配色按现有品牌资产规则补。

### 先做通用 web-session 机制（prefactor）

- 在 provider catalog 的双端定义里增加一个明确的 web-session 能力标记（例如 `web_session: bool` 或等价的登录方式枚举），由 family 自己声明，前端不再维护 `WEB_SESSION_FAMILIES = new Set(["tokenrhythm"])` 这类硬编码。
- 把现在 `tokenrhythm` 专用的设置行、Customize 跳转、认证中心添加账号、登录中状态、登录/重新登录/清除会话、onboarding 文案等分支，改成由 catalog 能力驱动；`tokenrhythm` 的行为和文案保持不变。
- 把 `tokenrhythm_login` / `tokenrhythm_login_status` / `tokenrhythm_login_clear` 收敛成按 family 分发的通用网页登录命令面；各 provider 只实现自己的登录窗口/会话读取/验证逻辑。远端 WebView 不能直接发 app command（tokenrhythm 已踩过这个限制），所以登录完成检测继续由 Rust 侧轮询网站接口，不依赖远端 IPC。
- 这一步不新增用户可见功能，只要求 `tokenrhythm` 全量回归通过；它是星火接入的前置条件。

### 登录与会话

- 登录窗口用隔离/incognito 的 WebView2，打开 `https://maas.xfyun.cn/` 的登录页或控制台入口，由用户手动完成讯飞登录；Pane 不碰验证码、不保存账号密码。
- 登录完成必须由网站接口确认，不能“有 cookie 就算已登录”。用轻量认证端点（例如控制台的 user 接口或 free-quota）判断：`code === 0` 为已登录，`code === 4001` 为未登录/过期。
- 控制台前端主要从 `localStorage.token` 取 `Authorization`；服务器是否同时下发可用认证 Cookie 尚未实测。实现顺序：
  1. 先验证“仅 Cookie”能否通过认证；能通过就复用现有 cookie manager 通道，改动最小。
  2. 不能通过时，增加一个**只读、最小面**的 WebView2 脚本读取通道，只取 `localStorage.token`（以及必要的 cookie），不开放任意脚本执行。
  3. 任何情况下都不读浏览器 profile 的 SQLite/Cookies 文件，也不要求用户手动粘贴 token（手动粘贴最多作为调试后备，不作为正式交互）。
- 会话经 `secretstore` 落 OS vault，scope 用 `web/xfyun`；vault 不可用时沿用项目现有明文回退约定并明确提示。多账号走 `login_accounts`，按网站返回的稳定用户 id 去重；重登只更新该账号，清除会话只清当前账号。
- 会话过期不是“错误卡片”：返回 `no_credentials` 语义，提示“网页会话已过期——在设置里重新登录”。网络/5xx/解析失败才返回 `error`，并保留原因。
- 登录窗口关闭、超时、用户取消都不能留下僵尸窗口或半条会话。

### 额度 API 契约

- 免费额度查询：`GET /api/v1/gpt-finetune/api-services/free-quota?channel=spark-x2.6`，带网页会话的 `Authorization`。
- 响应信封为 `{code, message, data}`；`code:4001` 表示未登录/过期，不能只看 HTTP 200。
- `data.claim` 可能为 `null`（未领取）；已领取时读 `data.claim.appId` 和 `data.claim.freeQuota`：
  - `totalTokens`、`usedTokens`、`remainingTokens`、`quotaStatus`、`expiresAt`。
- 数字可能以 JSON number 或字符串出现，解析器两者都接受；`expiresAt` 的精确格式待登录校准，解析失败时不要伪造日期。
- `quotaStatus` 至少处理 `ACTIVE`、`EXHAUSTED`、`EXPIRED`；未知值按原样显示或显示“状态未知”，不要当成 ACTIVE。
- 服务端返回的总量是权威；**不要把 10 亿写死为卡片值**。前端常量只用于领取按钮/文案，查询结果以 API 为准。
- 真实响应尚未在登录态抓包；第一张 provider 票必须先完成一次真实登录并记录结构，解析器对未知结构要诚实失败（`待确认`），不能猜数字。

### 卡片

- 主进度条 label 用「免费额度」，`used_percent = usedTokens / totalTokens`，detail 用「已用 X / 共 Y tokens」；另给「剩余」和「到期」两行文本。
- 不把到期时间塞进 `resets_at` 倒计时。`resets_at` 在现有 UI 里的文案是 “Resets in …”，对一次性 31 天额度会误导；到期用文本行表达。
- `claim=null`：`needs_setup` 卡片，提示“尚未领取免费额度，请在讯飞控制台领取后刷新”。不显示 0/0 进度条。
- `EXHAUSTED` / `EXPIRED`：显示状态文本 + 剩余 0；不要显示成正常百分比。
- 刷新沿用现有 provider 刷新循环；同一 family 的多个账号各自查询、各自过期、互不影响。
- 文案走 en/zh/ru 三字典；不新增只有中文的硬编码字符串。

### 用量摘要（后续票）

- 最近 7 天用量用 `GET /api/v1/gpt-finetune/api-services/usage`，参数含 `startDate` / `endDate` / 可选 `appId` / `modelId`；响应行含 `inputTokens`、`cacheTokens`、`outputTokens`。
- 如果现有卡片能承接，就聚合成 `UsageDays` 或文本行；不做独立表格页、不做按模型明细表、不做导出。
- 用量摘要失败不影响额度卡片；额度是主信息，用量是补充。

## Testing Decisions

- **最高 seam** 是 provider 的 snapshot/parse 边界和会话存储边界；不要为测试新增前端 UI 测试框架。解析器用纯函数测试，输入合成 JSON。
- 解析测试至少覆盖：`claim=null`；`ACTIVE` 正常值；`EXHAUSTED` / `EXPIRED`；数字为 number 和字符串两种；缺字段；未知 `quotaStatus`；`expiresAt` 不可解析；`code:4001`。
- 会话测试覆盖：保存/加载/清除；vault 失败回退；多账号按稳定 id 去重；过期只清当前账号；公开状态接口不泄露 token/cookie。
- prefactor 回归：`tokenrhythm` 的登录、状态、清除和设置行行为不变；现有 provider catalog 测试、node 测试、parse-tests 全绿。
- 不把真实 token 或真实响应体写进测试；需要真实结构的用例使用脱敏后的 fixture。
- 自动化测试不打真实网络；真实登录、控制台数字对照、会话重启持久化由 owner 实机验收。按照项目既有规则，agent 不能只凭代码阅读宣称 UI 完成。
- 测试先行的顺序：先写解析/状态机测试（红），再实现 provider 模块（绿）；prefactor 先补 tokenrhythm 回归测试，再改接线。

## Out of Scope

- API Key 推理代理、模型调用配置、base URL 管理；Pane 不转发星火请求。
- 按模型明细表、调用日志表、xls 导出、账单/充值/订单。
- 付费 Token Plan 套餐（`token-plan/quota`）只作为未来扩展，不在本次范围。
- 自动领取免费额度、自动实名认证、自动过验证码。
- 读取浏览器 profile 的 SQLite/Cookies 文件；不做“从默认浏览器导入会话”。
- 自动刷新网页 token（目前没有 refresh token 契约）；过期后人工重登。
- Spark-X2.6 之外的模型；当前 Key 只返回 `spark-x2.6`，其他模型另开需求。
- 移动端/非 Windows 登录窗口的额外适配；先按现有 Windows WebView2 路径交付。

## Further Notes

- 这是第二个 web-session provider（第一个是 `tokenrhythm`），所以 prefactor 不是洁癖，而是避免把 `family === "tokenrhythm"` 的特判复制到星火、再复制到下一个 provider。
- API Key 能做的只有推理和单次 `usage`；如果有人把“API Key 查不到额度”当成实现 bug，先回看 #38 的实测记录。
- `spark-x2.6` 是推理模型，短 `max_tokens` 会把预算全花在 reasoning 上；这影响的是调用方，不影响额度接口。
- 控制台前端把 10 亿/31 天写死是产品文案，不是 API 契约；真实总量以 `free-quota` 返回为准。
- 免费额度是实名赠送的一次性额度，不是 Token Plan；两者接口不同，不要混用。
- 真实响应结构未抓包是当前最大的不确定性；第一张 provider 票要把“登录后校准”作为验收项，而不是假设前端字段永远不变。
- 本地调研与证据：`temp/xfyun-maas-analysis-20261023.md`（不含密钥）；GitHub 父 issue #38。

## 工单拆分

- 01：通用 web-session provider 机制（prefactor；`tokenrhythm` 行为不变）
- 02：讯飞星火 family + 网页登录 + 会话持久化 + 多账号
- 03：free-quota 额度快照 + 卡片
- 04：最近 7 天用量摘要 + i18n/图标/文档收尾

阻塞关系：01 → 02 → 03 → 04；03 是主价值票，04 不阻塞前三个的验收。
