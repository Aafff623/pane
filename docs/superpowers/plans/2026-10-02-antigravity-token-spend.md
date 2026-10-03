# Antigravity Token 花费接入 + 三家统计机制复盘（2026-10-02）

> 目标：把 Antigravity（用户主力 IDE 之一）的 token 消耗接入 Pane 总花费面板；
> 复盘 cc-switch / ZCode / Pane 三家统计机制，明确差距与本次决策。
> 前置：`temp/reports/spend-stats-gap-analysis.md`（10-01 cc-switch/ZCode 分析）。

## 一、三家机制复盘（本轮源码实证）

### cc-switch（`temp/research/cc-switch`）
- **双轨采集**：①内置代理逐请求记账（`proxy/usage/parser.rs`，Claude/OpenAI/Gemini 流式+非流式，cache 五桶全解）；②会话日志导入（`services/session_usage*.rs`：claude/codex/gemini/grokbuild/mcode/opencode/pi）。
- **增量**：`session_log_sync` 表存每文件游标（mtime+line/byte offset+尾部指纹），只解析追加部分；Pi 用编码 revision 当游标。
- **去重**：request_id 命名空间（claude/claude-desktop 共享裸 `session:{message_id}`，其余 app:provider 作用域）；±10 分钟四桶全等的疑似重复单列不弃。
- **Codex delta**：`event_msg(token_count)` 的 `total_token_usage` 是会话累计值，必须算相邻 delta；跨模型不拆累计基线。
- **持久化**：明细 `proxy_request_logs`（26 列）30 天 → `usage_daily_rollups`（date×app×model×五桶token+cost）永久，明细 prune。
- 本机 `~/.cc-switch/cc-switch.db` 可当历史对账基线（有 9 月 codex/claude/pi 日数据；今日无代理流量）。

### ZCode（`temp/research/ZCode` + 本机 `~/.zcode/cli/db/db.sqlite`，1.05GB 活库）
- 事实表 `model_usage`：每次模型调用一行，含 `started_at`(ms)、`provider_id/model_id`、五桶 token（input/output/reasoning/cache_creation/cache_read）、`computed_total_tokens = input+cache两桶+output`、status。30 天滚动 prune。
- 写入在运行时钩子（每次调用一条），订阅额度走服务端 monitor API，与本地统计严格隔离。
- Pane 现扫 `~/.zcode/cli/rollout/model-io-*.jsonl` 已够用；DB 直读留作备选（WAL 活库，读需防锁）。

### Antigravity（本轮新破解，无任何先例工具支持）
- **本地数据**：`~/.gemini/antigravity/conversations/<uuid>.db`（IDE）与 `~/.gemini/antigravity-cli/conversations/`（CLI）各一个 SQLite/会话；同目录 `conversation_summaries.db` 有会话元数据。
- **库结构**：`gen_metadata(idx, data BLOB, size)`，每行一次模型调用，data 为 protobuf。
- **protobuf 字段**（wire 实测解码，见 `temp/scripts/probe-antigravity-*.js` 探针）：
  - 顶层 field 1（len）→ 子消息 `{ 4: usage, 9: 上下文遥测, 17: 回合聚合, 19: model 字符串 }`
  - `4 = { 1: input, 2: output, 3: reasoning(thoughts), 5: 回合累计总量, 10: tool/cache? }`——1/2/3 即 IDE 界面显示的 Prompt/Completion/Reasoning，**每次调用独立值，无需 delta**
  - `9.10.{1,4}` = 上下文已用/上限（遥测，不计费）
- **时间锚**：blob 内无 epoch；用 `conversation_summaries.db.conversation_summaries.last_modified_time`（.NET datetime 字符串）作会话日锚。**局限**：跨天会话全部归到最后一日；`0001-01-01` 哨兵回退文件 mtime。
- **数据质量实测**（本机 72+9 库 / 199MB）：7573 次模型调用，模型仅 `gemini-3.8-flash`(7234) / `gemini-3.7-flash`(339)，相邻完全重复行仅 1（无需去重机制）。
- 服务端 `cloudcode-pa.googleapis.com` 只有配额百分比（Pane 已接入），无逐调用 token 明细 → 本地 protobuf 是唯一源。

## 二、差距矩阵（本机实际存在的源）

| 源 | cc-switch | Pane 现状 | 本机有无 | 决策 |
|---|---|---|---|---|
| Claude/Codex/Grok/Pi/OpenCode/Kimi/Qwen/Hermes/MiniMax/Devin | ✅ | ✅ 已扫 | ✅ | 不动 |
| Gemini CLI | ✅ | ❌ | `~/.gemini/tmp` 空 | 跳过（机器上没在用） |
| grokbuild / mcode | ✅ | ❌ | 无目录 | 跳过（没装） |
| ZCode DB 直读 | — | jsonl 已扫 | ✅ | 维持 jsonl，DB 只做校验源 |
| **Antigravity** | ❌ 无人支持 | ❌ | ✅ 用户主力 IDE | **本次实现** |

## 三、实现方案（spend.rs，全自含 + 单测）

1. `fn antigravity() -> ProviderSpend`：
   - 遍历两个 `conversations/` 目录的 `*.db`（排除 -wal/-shm），mtime ≤31 天过滤；
   - 每库经现有 (mtime,size) 文件缓存（新增 `cached_parse` 通用助手，复用现有 cache()/probes/touched 机制）解析为 FileData；
   - 解析：rusqlite read-only 打开（失败即跳过、不缓存空结果），`SELECT data FROM gen_metadata ORDER BY idx`，protobuf 解码取 1.4.{1,2,3} + 1.19；
   - 日锚：先读 `conversation_summaries.db` 建 conversation_id→date 映射（.NET 时间字符串 chrono 解析，哨兵→库文件 mtime）；
   - tokens = input+output+reasoning；价格查 `pricing::lookup(model)`（gemini-3.x 目前无价 → note_unpriced，token 照计——用户明确 token 优先）；查到价则 reasoning 并入 output 计费（对齐 minimax 先例）。
2. `collect_daily` 注册 `antigravity()`，id="antigravity"（品牌图标/配色自动复用 provider 目录）。
3. parse-tests 内联单测：protobuf 解码（手工构造 blob）、.NET 时间解析+哨兵、日锚回退、缓存键。
4. 前端零改动（面板数据驱动）。

## 四、验证方案

1. parse-tests 全绿 + pnpm build + cargo check。
2. **Antigravity 自洽校验**：脚本独立求和 vs Pane 6736 API `antigravity` 切片（today/30d tokens 全等）。
3. **ZCode 交叉校验**：`db.sqlite` 的 `model_usage` 今日/30 日聚合 vs Pane zcode 切片（jsonl 口径差异需说明：五桶 vs Pane jsonl 计的桶）。
4. **cc-switch 历史对账**：其 `usage_daily_rollups` 9 月 codex 日 tokens vs Pane spend_history 同日值（口径：cc-switch 含 cache_read，Pane codex 计费桶不同——只对 output_tokens 这类可比桶）。
5. 用户实机验收：总花费面板出现 Antigravity 行 + 悬浮卡双指标。

## 五、风险与边界

- Antigravity protobuf 无官方 schema，版本更新可能改字段号 → 解码器写成宽松失败（缺字段即跳过该行），绝不相面崩溃；
- 跨天会话归末日（v1 已知局限，UI 不做特殊展示）；
- 1GB ZCode DB 不直读（WAL 锁风险 + jsonl 已覆盖）；
- gemini-3.x 模型当前无价 → 美元记 0 + ⚠ 标（tokens 是用户第一优先）。

## 六、实施与验证结果（2026-10-02 当日）

- 实现：spend.rs 新增 `antigravity()`（protobuf 解码 + 会话库扫描 + summaries 日锚 + cached_parse 缓存）；
  **zcode() 重写为 SQLite `model_usage` 直读**（jsonl 只补 30 天窗口外的旧日，按日互斥防双计）——
  实测发现 rollout jsonl 仅 3 个文件、漏计 99%+（84.5M/30d vs 真实 14.08B/30d），DB 才是 ZCode
  设置页自己的数据源，切换后与「工具自身统计」对齐。
- 途中修复：summaries 库实际位于 conversations 的父目录（初版拼错路径，静默走了 mtime 回退，被独立
  校验脚本抓出后修正）。
- 测试：parse-tests **359 通过 / 0 失败**（+3：protobuf 解码/字段缺失/.NET 时间）。
- 验证：
  1. Antigravity 自洽：独立 Node 重实现（temp/scripts/validate-antigravity-spend.cjs）last30=71,632,592
     与 Pane 完全一致；today 21.9M 一致。属双实现交叉，非工具官方数（Antigravity 无官方历史页，IDE 内
     逐回合 Prompt/Completion/Reasoning 可人工抽查）。
  2. ZCode 对工具权威源：Pane tokens30=14,081,019,735 vs db.sqlite 独立查询 14,080.5M（差值=对账
     间隙的实时增长），同表同口径即 ZCode 设置页数字。
  3. cc-switch 历史对账：跳过——其 9 月 codex 数据在 Pane 的 31 天 mtime 扫描窗外且无落盘基线，
     桶口径也不同（cache_read 单列）；前两项已覆盖「对齐工具自身统计」的要求。
- 修正第五节一条预判：gemini-3.x 实际查到了价（目录含 Gemini 定价），美元非 0。
