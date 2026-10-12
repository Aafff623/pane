# 卡片额度趋势改为「核心计量单位」趋势

**Status:** needs-triage — 等用户对 D1/D2/D5 拍板后转 ready-for-agent
**Feature dir:** `.scratch/trend-units/`
**Date:** 2026-10-12
**调研人:** ZCode agent（用户 2026-10-12 点名）

---

## Problem Statement

额度总览大卡片下面，每个 provider 详细卡片里的**额度趋势**，目前分三类数据源，其中一类是用户明确不想要的：

| 优先级 | 条件 | 现在画什么 | 用户评价 |
|---|---|---|---|
| 1 | 该 card id 有本地 CLI 日志（`lastSpend`） | **token/天**（`trend`）或 **美元/天**（`trend_cost`，看 `config.spendMetric`） | 可接受 |
| 2 | 有 credits 台账（Qoder CN / Trae CN） | credits/天 | 可接受 |
| 3 | 其余全部（API-key / 中转 key / MCP / 闪电说…） | **额度"最差已用百分比"/天**（`usage_history::worst_used_percent` 采样） | **❌ 明确不喜欢** |

代码锚点：`src/main.ts:811-826` 的 `trendSourceFor()`（三档优先级就在这十几行里）；第 3 档的数据由 `usage_history.rs` 的 `record_samples`/`trend_map` 提供。

用户原话：**"我并不喜欢这种方式，应该像 GPT 一样显示具体的使用趋势，而不是额度百分比趋势。"** 计量要按各自的核心单位：Coding Agent 按 token、闪电说按分钟、MCP 按 credits。

---

## 现状事实（已逐条读码核实）

### 1. 每张卡的"核心单位"其实已经在 metric 里了，只是没被用来画趋势

- 闪电说 `shandianshuo.rs`：`speech.usedMinutes` / `quotaMinutes` → 进度条，单位是**分钟**（`shandianshuo.rs:98-136`）。
- MCP `searchquota.rs`：Tavily 等已返回 credits（`searchquota.rs:412-424`，`"账户套餐已用 38 / 2000 credits"`），Firecrawl 有 `Metric::text("Credits", …)`（`:452`）。
- Coding Agent：token 由本地日志扫描器给出（见下）。

### 2. 现有 credits 台账是靠**解析用户可见字符串**得来的 —— 这是必须一并修掉的脆弱点

`usage_history.rs:281-289`：

```rust
pub fn credits_used_from_metrics(metrics: &[Metric]) -> Option<f64> {
    let metric = metrics.iter().find(|m| m.label == "Credits")?;
    let detail = metric.detail.as_deref()?;
    if !detail.ends_with("credits used") { return None; }          // ← 只认英文
    detail.split_whitespace().next()?.parse().ok()                  // ← 解析自然语言
}
```

后果：① MCP 的 credits **进不了台账**（detail 是中文「已用」、且 label/形状不一）；② 任何文案改动都会静默断掉台账；③ 这个函数依赖"生产者恰好产出 `"{used} of {total} credits used"`"——**CodeBuddy 分支的 spec D6 想要 `450 / 500 credits left`，落地时被迫改成 `49 of 500 credits used` 就是为了喂这个解析器**（评审判定它与 spec 不符，实质是 spec 与既有契约冲突）。

### 3. 跨 harness 聚合**已经存在**，就在本地日志扫描器里

`spend.rs:143-160` 的 source 表，每条带 `feeds: Vec<String>`——即"这个 harness 的日志喂给哪些 provider"：

- `claude-jsonl` → `["claude","minimax","aihubmix","kimi"]`
- `ccswitch-sqlite` → `["claude","codex","opencode","grok","mcode","gemini","claude-desktop"]`
- `zcode-sqlite` → `["zcode"]`；`codex-jsonl` → `["codex","kimi"]`；`antigravity-sqlite` → `["antigravity"]` …（共 16 条源）
- 每条事件带时间戳，按**本地日期**归日，产出 `ProviderSpend { trend: Vec<f64>（token/天，[29]=今天）, trend_cost, today/yesterday/last30 }`（`spend.rs:40-60`）。

所以"按时间顺序聚合到每个 provider 每天多少 token"这件事，**机制已在，不需要新建**；需要补的是"单位声明"和"没被扫描器覆盖的那些单位"。

### 4. 两种数据形状，聚合算法不同（这是设计的关键区分）

- **流量型（flow）**：每条事件自带时间戳与用量 → **按天求和**。token（本地日志）、credits（若上游给的是逐次消耗）属此类。
- **读数型（meter）**：上游只给"累计已用"读数，没有逐次事件 → **按天取当日最大值，再算相邻两天的差**；遇到额度重置（读数变小）必须夹到 0，不能算成负数。credits（多数 API-key 卡）、分钟（闪电说）属此类。

现有 credits 台账（`usage_history.rs:291-303`，同日取 max）只做了"读数型"的前半段，**没有对外给"每天消耗量"的正式形状**（`credit_extras` 是给积分列表用的）。

---

## Solution（2026-10-12 用户已拍板）

### D0 — 用词与悬浮（用户 2026-10-12 明确要求）

- 区块名**一律叫「额度趋势」**，不再出现「token 趋势」「credits 趋势」这类按单位命名的说法。
- 柱形图悬浮提示**必须显示该卡核心计量单位的当日用量**（Coding Agent 显示"当天用了多少 token"；闪电说显示分钟；MCP 显示 credits）。与 GPT 同逻辑。

### D1 — 单位声明表：一个家族一个核心单位（**分组已确认，按现有表**）

Rust 侧为权威，前端镜像（沿用 `providerCatalog` 的既有做法）：

| 模块 / 家族 | 核心单位 | 来源 |
|---|---|---|
| Coding Agent（claude / codex / antigravity / opencode / qwen / kimi / zcode / cursor / grok / droid …） | **token** | 本地日志扫描器（flow，已存在） |
| 闪电说 shandianshuo | **分钟** | 该卡 metric 的 `usedMinutes`（meter，新增台账） |
| MCP（bocha / tavily / firecrawl / brave / exa …） | **credits** | 该卡 metric 的 credits 读数（meter，接入既有台账） |
| Qoder CN / Trae CN | credits | 既有 credits 台账（已可用） |

### D2 — 台账改为**结构化读数**，不再解析字符串

把"单位 + 已用 + 总量"做成 metric 上的机器可读字段（或独立于 `Metric` 的并行结构），采样器只读结构化值。理由见现状事实 2。
兼容：既有 `credit_history.json` 保留，读取时按旧形状迁移；`credits_used_from_metrics` 的字符串解析**删除**（它现在同时是 CodeBuddy D6 那条文案的隐形约束，删掉后文案可以自由）。

### D3 — 趋势轴统一为"核心单位/天"，30 天，左旧右新，今天在最右

与现有 `ProviderSpend.trend` 同轴。百分比采样（`usage_history.json` 的 `worst_used_percent`）**不再作为趋势源**（它仍可留给别处用，如额度环，不在本次范围）。

### D4 — 两种形状各自的"每天消耗"算法

- flow：按本地日期求和（既有逻辑）。
- meter：`当日 max(读数) − 前一日 max(读数)`，负数夹 0；首日无前值则不计入。
- 跨 harness：沿用 `feeds` 映射，**不做二次去重**（去重规则见 `docs/project-facts.md` 的 cc-switch/ZCode 双计章节，本次不改）。

### D5 — 没有核心单位数据的卡：退而求其次画**请求次数**（用户 2026-10-12 拍板）

- 兜底顺序：**核心单位（token/credits/分钟）→ 请求次数 → 留空**。
- 请求次数的来源按家族定（搜索类卡多数已有 searches 计数；纯余额型 key 若连次数也没有，则该卡不画趋势区块，而不是编一个数）。
- 具体哪些家族有请求次数、从哪个字段取，在票 01 的单位表里一并列全（**不猜**：取不到就不画）。

### D6 — 总花费面板**保留**现有 token↔美元切换（用户 2026-10-12 拍板，不改）


---

## 影响面（估）

- Rust：`usage_history.rs`（台账结构化 + meter 差分）、`providers/mod.rs` 的 `Metric` 形状、`shandianshuo.rs`、`searchquota.rs`、`spend.rs`（flow 侧只读不改）、`lib.rs`（新增/调整 fetch 命令）、`provider_catalog.rs`（单位表）。
- 前端：`src/main.ts` 的 `trendSourceFor()` 与趋势渲染（`fmt`/tooltip 文案随单位变）、`providerCatalog.ts` 镜像。
- 测试：`parse-tests/` 加 meter 差分（含重置夹 0、首日无前值）、credits 结构化读数、单位表一致性（可失败的检查，守住"四层同步"）。
- 不动：本地日志扫描器的采集与去重规则、额度总览环、托盘投影。

## Ticket 拆分（草案，拍板后再建票）

1. `01-unit-model.md` — 单位表 + `Metric` 结构化读数（D1/D2），含 parse-tests 一致性检查。
2. `02-meter-ledger.md` — meter 型台账的"每天消耗"差分与重置夹 0（D4）。
3. `03-mcp-credits.md` — MCP 卡片接入 credits 趋势。
4. `04-shandianshuo-minutes.md` — 闪电说接入分钟趋势。
5. `05-trend-axis.md` — `trendSourceFor` 改为核心单位 + 无数据卡的处置（D3/D5）。
6. `06-remove-percent-trend.md` — 退役百分比趋势源 + 迁移/清理。

## Open Questions（不替你决定）

1. **D1 分组**：上面那张单位表的分组对不对？特别是"哪些家族算 Coding Agent（按 token）"。
2. **D5 兜底**：没有单位数据的卡——留空 / 保百分比 / 换请求数？
3. **总花费面板**现有"token / 美元"切换（`config.spendMetric`）要不要跟着简化成"只按 token"？还是保留切换？
4. 你提到"像 GPT 一样"——是指 GPT 网页版那种**按天柱状 + 具体数值**（数量级带单位、不显示百分比）吗？我按这个理解做的设计，如果指的是别的形态（比如折线、或者带累计曲线），告诉我。
