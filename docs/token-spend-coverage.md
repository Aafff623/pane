# Pane Token Spend Coverage

这份文档记录 Pane 当前如何收集 token 消费，以及新增工具形态时应放在哪一层。美元只作为可选派生值；主指标是实际记录到的 token 数。

## 计量原则

1. **先保留 token 事实，再计算美元。** 日志里有 token 数就计入，即使模型没有价格目录；没有 token 也没有可靠的猜测值。
2. **工具自己的 cost 优先。** OpenCode、Pi、Claude 等日志带有单次 cost 时直接采用；没有 cost 才走共享 pricing catalog。
3. **按请求的真实 token 桶计算。** input、output、reasoning、cache read、cache write 分开读取，避免把缓存 token 重复当作普通输入。
4. **同一请求只计一次。** 每种日志使用自己的去重键；Codex 还处理累计计数的增量和子会话回放。
5. **来源与计费方分开。** Pi、Hermes、OpenCode gateway、Claude 的兼容端点可能把请求转给另一家服务；扫描器先读来源，再按目标计费方归并到对应卡片。
6. **美元不确定时显式标记。** 未定价模型的 token 仍进入总量和趋势，美元留空并记录 `unpriced`，不伪造价格。

## 已覆盖的工具形态

| 形态 | 当前入口 | 计量方式 | 归并/限制 |
| --- | --- | --- | --- |
| Claude Code CLI | `~/.claude/projects/**/*.jsonl` | assistant `message.usage`，优先 `costUSD` | MiniMax、Qwen、Kimi 路由按模型拆出 |
| Codex CLI / rollout | `~/.codex/sessions`、`archived_sessions` | `token_count`，累计值转单次增量 | 子 agent/fork 回放去重 |
| ZCode CLI/Desktop | `~/.zcode/cli/db/db.sqlite` + rollout JSONL | SQLite `model_usage` 为主，旧日志补历史 | DB 覆盖日不再叠加 JSONL |
| Pi coding agent | `~/.pi/agent/sessions/**/*.jsonl` | assistant message usage | 折叠到它实际驱动的 Claude/Codex 卡 |
| Grok CLI | `~/.grok/logs/unified.jsonl` | `inference_done` token 桶 + 进程模型映射 | 没有模型归属的行不计 |
| OpenCode Desktop / CLI | `~/.local/share/opencode/opencode.db` | SQLite assistant message 的 token 与 cost | OpenCode gateway 行可归并到 AihubMix |
| Devin CLI | 本机 `config_home/devin/cli/sessions.db` | SQLite 每消息 metrics | Cloud Devin ACU 不在本地 ledger 中，无法从这里补齐 |
| MiniMax Agent | `~/.minimax/sqlite.db` | `token_usage` 每回合 token 桶 | Claude 兼容端点的 MiniMax 行合并 |
| Hermes Desktop | `%LOCALAPPDATA%/hermes/state.db` | `session_model_usage` 累计桶 | MiniMax/OpenRouter 路由分流 |
| Qwen Code | `~/.qwen/usage/token-usage-*.jsonl` | 每请求 token ledger | 只读近 31 天 |
| Kimi Code | Kimi code home 下 `sessions/**/*.jsonl` | `usage.record` turn 桶 | Moonshot 前缀归一化 |
| Antigravity IDE / CLI | `~/.gemini/antigravity*/conversations/*.db` | SQLite `gen_metadata` 中的 protobuf usage | 对话库按摘要日期归属日 |
| Cursor IDE | 认证后的 usage-events CSV | 远端 CSV 的已汇总 token | 需要 Cursor 可用认证；本地无日志时属于运行时来源 |

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

`spend::source_statuses()` 和 Tauri 命令 `fetch_spend_sources` 提供运行时来源盘点：

- `detected`：本机发现对应 ledger 或日志入口；
- `not_detected`：代码支持，但本机没有发现入口；
- `runtime`：需要认证或网络请求，不能用本地文件存在性判断。

这个清单只报告覆盖证据，不生成 token，也不把“发现文件”宣称成“所有请求都已记录”。后续收到新的 OpenCode Desktop 调研后，应先把它映射到上面的形态和契约，再决定是否新增解析器。
