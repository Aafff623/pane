# 07 — 本地 Token 统计：WorkBuddy CN/AI + CodeBuddy CLI

**Blocked by:** None（与 01–06 并行）
**Status:** done（代码层 + 单测；实机数据待 owner 打开用量面板核对）
**Issue:** #13（实施）

**What to build**

在 `spend.rs` 增加三个本地来源并接进 `collect_daily`：

| 来源 | 路径 | 卡 |
|---|---|---|
| WorkBuddy CN | `~/.workbuddy/projects/**/*.jsonl` | `workbuddy` |
| WorkBuddy AI | `~/.workbuddy-ai/projects` + `~/.workbuddy-ai/sessions` | `workbuddyai` |
| CodeBuddy CLI | `~/.codebuddy/projects/**/*.jsonl` | `codebuddy` |

解析口径（对齐上游 `token_stats.rs`）：
- usage 选择：`message.usage` → `providerData.usage` → 顶层 `usage`，以「存在 input 字段」为锚（允许 0）。
- input：`input_tokens|inputTokens|prompt_tokens`；output：`output_tokens|outputTokens|completion_tokens`；
  cache read：`cache_read_input_tokens|cacheReadInputTokens|prompt_cache_hit_tokens|cached_tokens` + details 回落；
  cache write：`cache_write_input_tokens|cacheWriteInputTokens|cache_creation_input_tokens|prompt_cache_write_tokens`。
- reasoning：`providerData.rawUsage.completion_thinking_tokens` / `completion_tokens_details.reasoning_tokens`，计入 output 侧总量。
- fresh input = `input - cache_read`（WorkBuddy 的 input 含 cache read）。
- 去重指纹：`(timestamp,input,output,cache_read,cache_write,model)` 跨文件共享；无时间戳不去重。
- 无价格 → `unpriced`，token 仍入账。
- `source_statuses()` 增加三条（detected / not_detected）。

**不做**：不做 CodeBuddy IDE `history/index.json`（形态复杂、属二阶段）；不做账号级归因（日志无 uid）。

**验收**
- [ ] fixture：三种 usage 路径、cache 拆分、reasoning、缺字段、重复行、无时间戳
- [ ] **同一文件内**同一请求只计一次；跨文件副本暂不去重（见下方限制）
- [ ] CN/AI/CLI 三卡互不混算
- [ ] `credit_history` 与 token 统计通道互不写对方单位

## Comments

- 2026-10-19 出自 spec D8；上游 `token_stats.rs:148-357,628-708,1092-1108`。
- 2026-10-19 已实现：`spend.rs` 新增 `workbuddy_line`（三处 usage 位置、
  input 含 cache read、reasoning 不二次相加）、`local_token_logs` 与三个来源；
  `source_statuses()` 登记 `workbuddy-jsonl` / `workbuddy-ai-jsonl` /
  `codebuddy-cli-jsonl`。
- **已知限制（如实记录）**：指纹去重的 `seen` 集合按文件建立，因为
  `file_days` 有逐文件缓存；因此**跨文件的整份副本**若同时被解析会重复计数
  （上游 `token_stats` 用跨文件集合避免这一点）。`subagents/` 目录已整体排除。
  本轮不为跨文件去重牺牲缓存；如需补齐，另立票把指纹持久化到解析缓存。
- 本机实测 `~/.workbuddy/projects` 34MB / 112 个 JSONL；`~/.codebuddy/projects`
  不存在（CLI 未使用），来源盘点会显示 `not_detected`。
