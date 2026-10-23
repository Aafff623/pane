# Pane Token Spend Coverage

这份文档记录 Pane 当前如何收集 token 消费，以及新增工具形态时应放在哪一层。美元只作为可选派生值；主指标是实际记录到的 token 数。

## 计量原则

1. **先保留 token 事实，再计算美元。** 日志里有 token 数就计入，即使模型没有价格目录；没有 token 也没有可靠的猜测值。
2. **工具自己的 cost 优先。** OpenCode、Pi、Claude 等日志带有单次 cost 时直接采用；没有 cost 才走共享 pricing catalog。
3. **按请求的真实 token 桶计算。** input、output、reasoning、cache read、cache write 分开读取，避免把缓存 token 重复当作普通输入。
4. **同一请求只计一次。** 每种日志使用自己的去重键；Codex 还处理累计计数的增量和子会话回放。
5. **来源与计费方分开。** Pi、Hermes、OpenCode gateway、Claude 的兼容端点可能把请求转给另一家服务；扫描器先读来源，再按目标计费方归并到对应卡片。
6. **美元不确定时显式标记。** 未定价模型的 token 仍进入总量和趋势，美元留空并记录 `unpriced`，不伪造价格。

## 成本口径（2026-10-23）

- **谁记的钱**：日志/网关自带 `costUSD` / `cost_usd` / `cost` 的行标记为
  `stored_cost`；其余走 Pane 的价格目录。模型行同时带 `missing_pricing`，
  未定价的 token 仍进趋势，美元显示为空并标 `unpriced`，绝不伪造 `$0`。
- **优先级**：`stored > override > catalog > unknown`。override 文件是
  `%APPDATA%\Pane\pricing_overrides.json`，单位 USD / 百万 token：
  `{"model-slug":{"input":3,"output":15,"cache_read":0.3,"cache_write":3.75}}`。
  设置 → 通用里有“模型价格覆盖”编辑器，保存时严格校验 JSON 并立即生效；
  直接改文件也可以，进程启动后首次查询会读取。
- **时间感知**：`pricing::lookup_at(timestamp, model)` 已接 Codex/Copilot 的
  计价路径，并用带时间戳的 probe 保护每文件缓存。时间窗表默认为空——
  没有第一方历史价目就不编造；DeepSeek 时段价、web search、flex/priority
  费用同理，只有官方依据才启用。
- **来源诊断**：设置 → 通用里的“花费来源”面板显示
  `detected / not_detected / empty / unreadable / parse_error / needs_probe / runtime`
  及原因，路径在悬停 tooltip 里；数据来自 `/v1/sources` 的同一注册表。
- **归因**：project/branch/session 归因仍是 opt-in 待办，当前不读 cwd、
  不读 prompt；关闭时行为与之前完全一致。Currency 仍按 ADR 0003 延后。

## 已覆盖的工具形态

| 形态 | 当前入口 | 计量方式 | 归并/限制 |
| --- | --- | --- | --- |
| cc-switch 聚合 ledger | `~/.cc-switch/cc-switch.db`（只读） | `proxy_request_logs` 明细（近 35 天，`created_at` 为秒）∪ `usage_daily_rollups`（永久，本地日 TEXT）；`input_token_semantics` CASE 归一 fresh input | claude 主源（自扫只补 cc 没有的日）；codex/opencode/grok 补缺日；mcode/gemini/claude-desktop 纯 cc 来源；成本走 Pane 自己的价目表，不采信 `total_cost_usd` |
| Claude Code CLI | `~/.claude/projects/**/*.jsonl` | assistant `message.usage`，优先 `costUSD` | MiniMax、Qwen、Kimi 路由按模型拆出 |
| Codex CLI / rollout | `~/.codex/sessions`、`archived_sessions` | `token_count`，累计值转单次增量 | 子 agent/fork 回放去重 |
| ZCode CLI/Desktop | `~/.zcode/cli/db/db.sqlite` + rollout JSONL + `~/.zcode/v2/provider_config.json` | SQLite `model_usage` 为主，旧日志补历史；`providerRules` 的 baseUrl host 决定归到哪张卡（tokenrhythm / sensenova / stepfun-plan / clinepass / minimax / deepseek） | DB 覆盖日不再叠加 JSONL；无 baseUrl 的规则按 providerId 同名兜底；host 未映射的留在 ZCode 卡，模型标签写 `providerName · modelId`（2026-10-20） |
| Pi coding agent | `~/.pi/agent/sessions/**/*.jsonl` | assistant message usage | 折叠到它实际驱动的 Claude/Codex 卡 |
| Grok CLI | `~/.grok/logs/unified.jsonl` | `inference_done` token 桶 + 进程模型映射 | 没有模型归属的行不计 |
| OpenCode Desktop / CLI | `~/.local/share/opencode/opencode.db` | SQLite assistant message 的 token 与 cost | OpenCode gateway 行可归并到 AihubMix |
| Devin CLI | 本机 `config_home/devin/cli/sessions.db` | SQLite 每消息 metrics | Cloud Devin ACU 不在本地 ledger 中，无法从这里补齐 |
| MiniMax Agent | `~/.minimax/sqlite.db` | `token_usage` 每回合 token 桶 | Claude 兼容端点的 MiniMax 行合并 |
| Hermes Desktop | `%LOCALAPPDATA%/hermes/state.db` | `session_model_usage` 累计桶 | MiniMax/OpenRouter 路由分流 |
| Qwen Code | `~/.qwen/usage/token-usage-*.jsonl` | 每请求 token ledger | 只读近 31 天 |
| Kimi Code | Kimi code home 下 `sessions/**/*.jsonl` | `usage.record` turn 桶 | Moonshot 前缀归一化 |
| Antigravity IDE / CLI | `~/.gemini/antigravity*/conversations/*.db` | SQLite `gen_metadata` 中的 protobuf usage（1=system/2=input/3=output/5=cacheRead） | 按事件时间戳归属日（`steps.metadata`，field 1 = Timestamp；缺省降级摘要日期→库 mtime） |
| Cursor IDE | 认证后的 usage-events CSV | 远端 CSV 的已汇总 token | 需要 Cursor 可用认证；本地无日志时属于运行时来源 |
| WorkBuddy CN | `~/.workbuddy/projects/**/*.jsonl` | `message.usage` / `providerData.usage` / 顶层 `usage`；input 含 cache read | `subagents/` 排除；跨文件共享请求指纹（2026-10-21） |
| WorkBuddy AI | `~/.workbuddy-ai/projects` + `sessions` | 同上 | 与 CN 分开成卡，绝不合并（2026-10-19） |
| CodeBuddy CLI | `~/.codebuddy/projects/**/*.jsonl` | 同上 | 与 WorkBuddy 分开成卡；跨文件共享请求指纹（2026-10-21） |
| CodeBuddy IDE | `CodeBuddyExtension/Data/**/history/**/index.json` | `requests[].usage`（camelCase；input 含 cache read） | 与 CLI 分开成卡；跨文件共享请求指纹（2026-10-21） |
| GitHub Copilot CLI | `~/.copilot/session-store.db` → `assistant_usage_events`；`~/.copilot/session-state/*/events.jsonl` 兜底 | 逐调用 token 桶：`input_tokens` 是整段 prompt（含 cache），tokens = prompt + output；cache read/write 分开计价 | DB 优先，按 session 去重；DB 无该 session 时才用 `session.shutdown.modelMetrics`；`total_nano_aiu` 是 AI credits，不折美元 |

订阅额度 API、网页余额和服务端百分比不属于 token spend ledger；它们继续走 quota provider，不能拿百分比反推 token。

## 新增来源的最小契约

新增工具时先判断它属于哪种形态：

- **本地逐请求日志**：新增一个解析器，输出 `(timestamp, model, input, output, reasoning, cache_read, cache_write, reported_cost)`。
- **本地 SQLite**：只读打开；运行中的数据库先复制主库和 WAL，再查询；用主键或 `(session, message)` 去重。
- **服务端汇总接口**：只能把接口返回的 token 汇总作为独立来源，并标记时间范围、是否跨设备；不要和本地日志直接相加。
- **IDE/desktop 共享 ledger**：优先找它真正写入的 SQLite/JSONL，而不是从 UI 文本或截图推算。
- **路由器/代理**：保留来源工具字段，同时根据 billing provider 把事件归并到实际付费方；未知路由留在来源卡并标记未归并。

每个新来源都要补：

1. 真实样本或字段证据；
2. 时间戳单位和时区规则；
3. token 桶定义，尤其是 reasoning/cache 是否已包含在 total；
4. 去重和累计值转 delta 规则；
5. 失败时是跳过、保留上次结果，还是标记部分覆盖；
6. 一个解析单测和一个重复/缺字段单测；
7. `spend::source_statuses()` 中的来源登记。

## 本轮新增的来源层

`spend::source_statuses()` 和 Tauri 命令 `fetch_spend_sources` 由
`src-tauri/src/spend/sources/mod.rs` 的注册表生成（新增来源 = 模块 + fixture + 一行注册），
提供运行时来源盘点：

- `detected`：本机发现对应 ledger 或日志入口，且文件可读；
- `not_detected`：代码支持，但本机没有发现入口；
- `empty`：入口存在但是空文件/空目录；
- `unreadable`：入口存在但读不了（权限、锁、损坏）；
- `parse_error`：入口可读但解析器连 schema 都打不开（例如缺表）；
- `needs_probe`：代码尚未针对真实样本校准（当前是 Gemini CLI）；
- `runtime`：需要认证或网络请求，不能用本地文件存在性判断。

Codex 与 Claude 的 JSONL 扫描使用 `(mtime, size, 4 KiB boundary hash)` + 字节 offset
断点：追加时只读新增行并合并摘要；文件缩短、边界哈希不符、价格目录换代、状态损坏
或最后一行未写完都会回退整文件重读，宁可慢不可漏。Codex `.jsonl.zst` 每次完整解压，
因为压缩流内的字节 offset 不是可靠断点。Codex 新版的 `token_usage_record` 按
`response_id` 去重并优先于紧随其后的重复 `token_count`；compaction 行不改写累计
基准，已被现有“相同累计值不重复计数”规则覆盖。

### WorkBuddy / CodeBuddy 口径（2026-10-21）

- 三种 usage 位置并存（`message.usage` camelCase、`providerData.usage`
  snake_case、顶层 `usage`），以「存在 input 字段」为锚；input 为 0 仍有效。
- **input 已包含 cache read**（实测 `inputTokens` 29751 / `cached_tokens`
  15744），所以计价用 `input - cache_read` 作为 fresh input；总量 =
  input + output + cache_write。
- **reasoning 已包含在 completion 内**（实测 `reasoning_tokens` 52 <
  `completion_tokens` 66），不再二次相加。
- CodeBuddy IDE 不写 JSONL：每个会话的 `requests[].usage` 在
  `CodeBuddyExtension/Data/**/history/**/index.json`；模型取自同层 workspace
  `index.json` 的 `conversations[].selectedModelId`，缺失按 unknown/未定价计。
- 去重：按 `(timestamp,input,output,cache_read,cache_write,model)` 指纹，
  **跨文件共享**（文件按 mtime 从旧到新处理，复制/分叉会话只算一次）；
  `subagents/` 目录整体排除。逐文件缓存只存日汇总，无法事后扣除副本，因此
  这几类源每次扫描重读（31 天窗口内），不再走 `file_days` 缓存。
- 账号归属：日志里没有 uid/email，按 **family 级**归卡（CN / AI / CLI / IDE
  四张卡），不伪造账号级归因。

这个清单只报告覆盖证据，不生成 token，也不把“发现文件”宣称成“所有请求都已记录”。后续收到新的 OpenCode Desktop 调研后，应先把它映射到上面的形态和契约，再决定是否新增解析器。

## Qoder CN — local transcript ledger (2026-10-20, GitHub #28 phase 1)

The earlier "all-zero usage" premise was wrong for `~/.qoder-cn/projects`:
desktop transcripts carry real per-request `credits`, `original_credits` and
`context_usage_ratio`; only `input_tokens` / `output_tokens` are zero.
`src-tauri/src/qoder_sessions.rs` now scans the last 31 days (subagents under
`<sessionId>/subagents/` fold into the parent and inherit its window) and
exposes `fetch_qoder_spend`:

- **Credits** are measured; **billed input tokens** are derived as
  `Σ(context_usage_ratio × contextWindow in force at that request)`.
- **Nominal vs charged:** each request's usage also carries `billable`.
  `billable: false` means a free/zero-rate model (observed for
  DeepSeek-V4-Flash on 2026-10-20) — the transcript still records nominal
  credits, but the platform quota never moves. The scanner therefore reports
  `credits` (nominal) and `charged_credits` (Σ credits where
  `billable != false`) separately, plus `free_trips`.
- A missing window leaves credits intact and marks those trips' tokens
  unavailable — never a silent 200K fallback.
- Credits stay out of the dollar totals and the dollar donut, and are never
  converted into dollars. The **derived tokens** now join the token views:
  each spend window carries per-model totals, so Total Spend's by-model and
  by-tool lists include Qoder CN's contributions (cost 0, merged by the same
  normalized model name the other tools use) with an `est.` badge. The
  credits row shows **charged** credits, so an all-free day is visible at `0`
  with a `free N · nominal X` note instead of hiding. The day block shows
  charged/nominal per model and the Top 10 sessions by nominal credits.
- No network requests, no conversation bodies, no credits→dollar conversion.
