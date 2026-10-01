# Spend Stats Upgrade Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把 Pane 的总花费面板和用量趋势从「31 天内存窗口 + 百分比采样」升级为「永久日聚合历史 + 多时间范围 + USD/Token 双口径趋势」，并修掉 donut 大字号溢出和配额趋势的"0 即无数据"歧义。

**Architecture:** 新增 `src-tauri/src/spend_history.rs` 做日聚合持久化（JSON，沿用 `usage_history.rs` 的 OnceLock+tmp-rename 模式），数据源是 `spend::collect()` 每次扫描产出的 (provider, day, model) 三元组——扫描对近 31 天是权威的，所以历史合并用**按日替换**语义（幂等，不双算）；31 天前的日子永久保留。前端总花费面板右列改成 7天/30天/全部 三档范围切换，全部档从 `fetch_spend_history` 读。

**Tech Stack:** Rust (serde/chrono, 无新依赖) + 原生 TS/SVG 前端（无框架）。测试走 `parse-tests` harness（`cargo +stable-x86_64-pc-windows-gnu test`），前端 `pnpm build`（tsc + vite）。

## Global Constraints

- 不新增任何依赖（Rust crate 或 npm 包）。
- `src-tauri/Cargo.toml` crate-type 保持 `["rlib"]`，不得加 `cdylib`/`staticlib`。
- 所有新 Rust 代码必须能被 `parse-tests` harness 编译：不能用 `tauri::` 真实 API，配置目录走 `crate::providers::config_dir()`。
- 前端文案必须三语言齐全（en/zh/ru），key 加在 `src/i18n.ts`。
- 趋势数组约定：`trend[29]` 是今天（`TREND_DAYS = 30`），前后端一致。
- 美元数字的 ⚠ 语义不变：无法定价的模型 token 计入 tokens 但成本记 0。
- 提交规范： Conventional Commits，如 `feat(spend): ...` / `fix(ui): ...`。
- 工作分支：`codex/spend-stats-upgrade`（Task 0 建）。

---

### Task 0: 固化现场 + 建分支

**Files:** 无代码改动。

- [ ] **Step 1: 提交 Codex 已验证的存量改动**

```bash
cd /d/code/pane
git add -A
git commit -m "feat(stepfun): Step Plan subscription provider + spend metric toggle + account archive"
```

- [ ] **Step 2: 建功能分支**

```bash
git checkout -b codex/spend-stats-upgrade
```

---

### Task 1: donut 中心大字号自适应

**Files:**
- Modify: `src/main.ts:2190-2203`（`spendCenter` 附近新增 helper）、`src/main.ts:2497-2521`（两处 `<text>` 硬编码 font-size）
- Modify: `src/styles.css`（`.col-total` 增加长文本降档类，现约 1478-1482 行写死 11.5px）

**Interfaces:**
- Produces: `fitFontSize(text: string, base: number, maxChars: number): number` —— 超出 `maxChars` 按比例缩，下限 8。

- [ ] **Step 1: 实现 helper 并替换两处硬编码**

```ts
/// Shrink the ring's center number when it outgrows the hole: base size
/// fits ≤maxChars, longer strings scale down linearly (floor 8).
function fitFontSize(text: string, base: number, maxChars: number): number {
  return text.length <= maxChars ? base : Math.max(8, Math.floor((base * maxChars) / text.length));
}
```

`main.ts:2502-2503` 与 `2514-2515` 改为（两处相同）：

```ts
<text class="donut-total" x="48" y="50" text-anchor="middle" font-size="${fitFontSize(center.primary, 14, 7)}" font-weight="600">${center.primary}</text>
<text class="donut-sub" x="48" y="62" text-anchor="middle" font-size="${fitFontSize(center.sub, 8, 12)}">${center.sub}</text>
```

- [ ] **Step 2: `.col-total` 长文本降档**

`main.ts` 两处 `col-total`（2458、2483 行）包一层：

```ts
const colTotalHtml = (w: SpendWindow) => {
  const s = fmtSpendVal(w);
  return `<div class="col-total${s.length > 10 ? " long" : ""}" title="${escapeHtml(s)}">${escapeHtml(s)}</div>`;
};
```

`styles.css` 在 `.col-total` 规则后加：

```css
.col-total.long { font-size: 10px; }
```

- [ ] **Step 3: `pnpm build` 通过，commit `fix(ui): donut center and column totals shrink to fit`**

---

### Task 2: 配额趋势「无数据」断点

**Files:**
- Modify: `src-tauri/src/usage_history.rs:112-132`（`trend_for`）、`:170-178`（`trend_map`）、`:229-239`（测试）
- Modify: `src-tauri/src/lib.rs:2541-2544`（`fetch_usage_history` 返回类型）
- Modify: `src/main.ts:512`（`TrendSource`）、`:516-522`（`trendSourceFor`）、`:1363-1394`（`renderTrend`）、`lastQuotaTrend` 声明处（约 6128-6146 附近 fetch 处）
- Modify: `src/i18n.ts` 加 `spend.noDataDay`（en "No data" / zh "无数据" / ru "Нет данных"）

**Interfaces:**
- Produces: `fetch_usage_history() -> BTreeMap<String, Vec<Option<f64>>>`（serde: `None` → `null`）
- Produces: `type TrendSource = { id: string; trend: (number | null)[]; quota: boolean }`

- [ ] **Step 1: 改 `trend_for` 返回 `Vec<Option<f64>>`，无采样日为 `None`**

```rust
fn trend_for(
    entries: &BTreeMap<String, Vec<DaySample>>,
    id: &str,
    today: &str,
) -> Vec<Option<f64>> {
    let Some(today) = parse_day(today) else {
        return vec![None; TREND_DAYS];
    };
    let samples = entries.get(id);
    (0..TREND_DAYS as i64)
        .map(|i| {
            let day = (today - Duration::days(TREND_DAYS as i64 - 1 - i))
                .format("%Y-%m-%d")
                .to_string();
            samples.and_then(|list| list.iter().find(|s| s.day == day)).map(|s| s.used)
        })
        .collect()
}
```

`trend_map` 返回类型同步改 `BTreeMap<String, Vec<Option<f64>>>`；`lib.rs:2542` 签名同步。

- [ ] **Step 2: 更新 `usage_history.rs` 测试**：`trend_spans_thirty_days...` 中 `trend[1]` 断言改 `assert_eq!(trend[1], None)`，missing 卡断言改 `iter().all(|v| v.is_none())`。先跑 parse-tests 确认失败/通过后继续。

- [ ] **Step 3: 前端渲染断点**

```ts
// renderTrend 顶部
if (!source.trend.some((v) => v != null && v > 0)) return "";
const max = Math.max(...source.trend.map((v) => v ?? 0));
// bars map 内：
const h = v != null && v > 0 ? Math.max(2, (v / max) * 30) : 1;
const cls = v == null ? "trend-nodata" : v > 0 ? "trend-bar" : "trend-zero";
// trend-day <g> 的 hit rect data-trend 不变；tooltip 处理处（搜 data-trend 的
// mouseover 监听）对 quota 源 null 日显示 t("spend.noDataDay")。
```

`trendSourceFor` quota 分支：`if (sampled?.some((v) => v != null && v > 0))`。spend 分支 trend 仍是 `number[]`，直接用（并集类型兼容）。

`styles.css` 加：

```css
.trend-nodata { fill: var(--line, #888); opacity: 0.25; }
```

（先看 `.trend-zero` 现有规则用什么颜色变量，保持一致。）

- [ ] **Step 4: `pnpm build` + parse-tests 全绿，commit `feat(trend): distinguish no-data days from zero usage in quota trends`**

---

### Task 3: USD 花费趋势序列

**Files:**
- Modify: `src-tauri/src/spend.rs:39-55`（`ProviderSpend` 加字段）、`:400-447`（`build_spend` 填值）
- Modify: `src/main.ts:78-87`（TS 接口）、`:512-522`（`TrendSource`/`trendSourceFor`）、`:1381-1394`（label 选择）
- Modify: `src/i18n.ts` 加 `spend.costTrend`（en "Spend Trend" / zh "花费趋势" / ru "Расход по дням"）

**Interfaces:**
- Produces: `ProviderSpend.cost_trend: Vec<f64>`（serde 默认 snake_case → 前端 `costTrend`？——注意 serde 不重命名，字段名 `cost_trend` 到前端就是 `cost_trend`，TS 接口写 `cost_trend: number[]`）
- Consumes: Task 2 的 `TrendSource` 形状

- [ ] **Step 1: 后端加字段**

```rust
// ProviderSpend 内，trend 之后：
/// Dollars per day, same axis as `trend` — trend_cost[29] is today.
pub trend_cost: Vec<f64>,
```

`build_spend`：初始化 `trend_cost: vec![0.0; TREND_DAYS]`，在 `if day > today - TREND_DAYS as i32` 块内 `sp.trend_cost[idx] += cost;`。

- [ ] **Step 2: parse-tests 里 spend 相关测试若构造 ProviderSpend 字面量会编译失败——逐个补 `trend_cost: vec![0.0; TREND_DAYS]`。跑测试确认。**

- [ ] **Step 3: 前端趋势跟随统计口径**

```ts
// trendSourceFor 的 local 分支：
const metricCost = config.spendMetric === "cost";
if (local) return { id, trend: metricCost ? local.trend_cost : local.trend, quota: false };
```

`renderTrend` 的 label 三元改成：quota → `spend.quotaTrend`；非 quota 且 `config.spendMetric === "cost"` → `spend.costTrend`；否则 `spend.tokenTrend`。tooltip 的 `spend.trendTip` 用的是 `fmtTokens(max)`——cost 模式下需用 `fmtMoney(max)`，给 `TrendSource` 加 `fmt: (v: number) => string` 字段在 `trendSourceFor` 里绑定。

- [ ] **Step 4: `pnpm build` + parse-tests 全绿，commit `feat(spend): USD daily trend series, card trend follows metric toggle`**

---

### Task 4: 日聚合持久化 spend_history.rs

**Files:**
- Create: `src-tauri/src/spend_history.rs`
- Modify: `src-tauri/src/lib.rs`（`mod spend_history;` + `fetch_spend` 尾部合并 + 新命令 `fetch_spend_history` + invoke handler 注册）
- Modify: `src-tauri/src/spend.rs:2956`（`collect` 改造为同时产出每日明细）
- Modify: `parse-tests/src/lib.rs`（若 harness 用 `#[path]` 显式列模块，加 spend_history）

**Interfaces:**
- Consumes: `spend::DayMap`（需要把 `(i32, String) → (f64, f64)` 的私有类型改成 `pub`，或新增 pub 结构承接）
- Produces:
  ```rust
  /// spend.rs 侧
  pub struct ProviderDays { pub id: String, pub name: String, pub days: DayMap }
  pub fn collect_daily(cursor_csv: Option<String>) -> (Vec<ProviderSpend>, Vec<ProviderDays>)

  /// spend_history.rs 侧
  pub struct RangeSpend { pub id: String, pub cost: f64, pub tokens: f64,
                          pub active_days: u32, pub models: Vec<spend::ModelSpend> }
  pub fn merge_daily(days: &[ProviderDays])            // 按日替换，幂等
  pub fn range_spend(range_days: Option<u32>) -> Vec<RangeSpend>  // None = 全部
  ```

- [ ] **Step 1: spend.rs 暴露 DayMap 并拆 collect**

`type DayMap` 改 `pub type DayMap`。`collect` 现在是 giant 函数（2956 行起）：找到它组装 `build_spend(id, name, data)` 的位置，把每次调用的 `(id, name, data.days.clone())` 收集进 `Vec<ProviderDays>`，新签名：

```rust
pub fn collect(cursor_csv: Option<String>) -> Vec<ProviderSpend> {
    collect_daily(cursor_csv).0
}

pub fn collect_daily(cursor_csv: Option<String>) -> (Vec<ProviderSpend>, Vec<ProviderDays>) {
    // 原 collect 体，返回 (result, daily)
}
```

注意 Cursor：CSV 没拉到（网络失败/禁用）时该次 collect 没有 cursor 数据——`merge_daily` 只合并本次结果里出现的 provider，不出现的不动（历史保留），天然安全。

- [ ] **Step 2: 写 spend_history.rs**

模式照抄 `usage_history.rs`（OnceLock<Mutex<HistoryFile>>、tmp+rename 持久化、`config_dir().join("spend_history.json")`）：

```rust
//! Permanent per-day spend rollups. spend.rs only scans logs modified in
//! the last 31 days; each collect() replaces that window's days here
//! (the rescan is authoritative inside the window), and older days are
//! kept forever — archived accounts included, keyed by spend provider id.

use chrono::{Duration, Local, NaiveDate};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

const VERSION: u32 = 1;

#[derive(Serialize, Deserialize, Clone, Default)]
struct ModelDay { cost: f64, tokens: f64 }

#[derive(Serialize, Deserialize, Clone, Default)]
struct ProviderDay {
    cost: f64,
    tokens: f64,
    models: BTreeMap<String, ModelDay>,
}

#[derive(Serialize, Deserialize)]
struct HistoryFile {
    version: u32,
    /// "YYYY-MM-DD" → provider id → rollup.
    days: BTreeMap<String, BTreeMap<String, ProviderDay>>,
}

// store()/persist() 照抄 usage_history.rs 模式。

/// Replace-merge one collect() run: every (day, provider) present in the
/// scan overwrites that cell; cells the scan didn't touch keep their value.
/// Idempotent because collect() recomputes the whole 31-day window from
/// cached+fresh parses every run.
pub fn merge_daily(daily: &[crate::spend::ProviderDays]) {
    if daily.is_empty() { return; }
    let today = Local::now().date_naive();
    let epoch = NaiveDate::from_ymd_opt(1, 1, 1).unwrap();
    let file = &mut *store().lock().unwrap();
    for pd in daily {
        for ((day_ce, model), (cost, tokens)) in &pd.days {
            let day = (epoch + Duration::days((*day_ce - epoch.num_days_from_ce()) as i64))
                .format("%Y-%m-%d").to_string();
            let cell = file.days.entry(day).or_default()
                .entry(pd.id.clone()).or_default();
            let m = cell.models.entry(model.clone()).or_default();
            m.cost = *cost; m.tokens = *tokens;
        }
    }
    // re-derive per-day totals from models
    for provs in file.days.values_mut() {
        for pd in provs.values_mut() {
            pd.cost = pd.models.values().map(|m| m.cost).sum();
            pd.tokens = pd.models.values().map(|m| m.tokens).sum();
        }
    }
    persist(file);
}
```

注意：cell 级替换要先清空该 provider 当日的 models 再写（模型集合会变）：`cell.models.clear()` 后再填。

`range_spend(range_days: Option<u32>)`：cutoff = today - range_days（None 不过滤），遍历 days 聚合每 provider 的 cost/tokens/models（模型跨天相加），`active_days` = 该 provider tokens>0 或 cost>0.004 的天数；只返回有数据的 provider。复用 `spend::ModelSpend` 输出模型榜（调 `spend::finalize_models` 的话需改 pub，或直接全量返回让前端折叠——选择：返回全量 models 排序后前 20，前端折叠）。

- [ ] **Step 3: 接命令**

`lib.rs`：`mod spend_history;`；`fetch_spend` 里 `spawn_blocking` 改调 `collect_daily`，拿到后 `spend_history::merge_daily(&daily)`，返回 `.0`；新命令：

```rust
/// Long-range spend from the permanent daily rollups. range_days=None = all time.
#[tauri::command]
fn fetch_spend_history(range_days: Option<u32>) -> Vec<spend_history::RangeSpend> {
    spend_history::range_spend(range_days)
}
```

注册进 invoke handler（搜 `fetch_usage_history` 在 handler 列表的位置，旁边加）。

- [ ] **Step 4: 单测**（写在 spend_history.rs 内 `#[cfg(test)]`，harness 自动编译）：
  - `merge_is_idempotent`：同一 ProviderDays 合并两次，结果相同。
  - `merge_replaces_cell_keeps_others`：day1 有 a/b 两 provider，第二次合并只含 a 的新值，b 不变。
  - `range_spend_filters_and_counts_active_days`：构造 3 天数据，range=2 只回 2 天，active_days 正确。
  先写测试确认编译失败 → 实现 → 全绿。
- [ ] **Step 5: parse-tests 全绿，commit `feat(spend): permanent daily spend rollups (spend_history.json)`**

---

### Task 5: 前端范围切换（7天/30天/全部）

**Files:**
- Modify: `src/main.ts:2415-2539`（`renderTotalSpend` 右列改三档 tab）、`lastSpendHistory` 状态 + `refresh()` 里 fetch（约 6011-6015、6128-6146 附近）、 SpendTab 类型（搜 `type SpendTab`）、`donutEntries`/`donutGeometry` 适配
- Modify: `src/i18n.ts` 加 `spend.days7`（"7 Days"/"7 天"/"7 дней"）、`spend.rangeAll`（"All"/"全部"/"Всё"）
- Modify: `src/styles.css` 如需微调 tab 组

**Interfaces:**
- Consumes: `fetch_spend_history(range_days: number | null) → RangeSpend[]`（`{id, cost, tokens, active_days, models}`）
- Produces: `let rangeTab: "d7" | "d30" | "all" = "d30"`；history 数据转成 `ProviderSpend` 形状塞进 donut 流水线：

```ts
/// History rows ride the same donut/legend pipeline as live spend windows.
function historyWindow(range: "d7" | "all"): Map<string, SpendWindow> {
  const rows = lastSpendHistory[range] ?? [];
  return new Map(rows.map((r) => [r.id, { cost: r.cost, tokens: r.tokens, models: r.models }]));
}
```

- [ ] **Step 1: 状态与拉取**：`refresh()` 内在 `fetch_spend` 后并行 `invoke("fetch_spend_history", { rangeDays: 7 })` 和 `{ rangeDays: null }`，存 `lastSpendHistory = { d7, all }`（30 天直接用现有 `lastSpend` 的 last30，不重复拉）。
- [ ] **Step 2: 右列三档 tab**：`renderTotalSpend` 的 `spendCol("last30", ...)` 换成：tab 行（7天/30天/全部，active 跟随 `rangeTab`）+ 内容按 tab 取数（d30 → 现有 `donutEntries("last30")`；d7/all → `historyWindow` 转的 entries，构造 `DonutEntry` 时 `s.name` 从 `lastSpend` 找名字，找不到用 providerCatalog 的 displayName，再找不到用 id）。donut 在主视图 tab 是范围档时也跟着切换（`donutEntries` 扩展支持 range 源）。
- [ ] **Step 3: 全部档图例行尾追加活跃天数**：legend row 的 detail 文案对 range 源显示 `t("spend.activeDays", { n })`（i18n 三语言："{n} days"/"{n} 天活跃"/"{n} дн."）。
- [ ] **Step 4: 点击 tab 的事件委托**：搜现有 `data-tab` 处理处，加 `data-range-tab` 分支，写 `rangeTab` 并 re-render。
- [ ] **Step 5: `pnpm build` 全绿，commit `feat(ui): 7d/30d/all range tabs on the total spend panel`**

---

### Task 6: 全量验证 + 收尾

- [ ] **Step 1: `pnpm build` 通过**
- [ ] **Step 2: parse-tests `cargo +stable-x86_64-pc-windows-gnu test` 全绿**（ mingw PATH 前置）
- [ ] **Step 3: `cargo +stable-x86_64-pc-windows-gnu check`（src-tauri，确认主 crate 也编译）**
- [ ] **Step 4: 重启 Pane**：`temp/scripts/pane-wmi-start.ps1` 流程（杀 pane/vite → 清 EBWebView → WMI 起 vite + pane → 探 1420/6736）
- [ ] **Step 5: 生成用户验收清单**（donut 大金额、趋势断点、范围切换、归档后全部档仍见历史）
- [ ] **Step 6: 更新 `temp/reports/spend-stats-gap-analysis.md` 标记已交付项**

## Self-Review 记录

- Spec 覆盖：P0 三项 → Task 1/2/3；P1 持久化+范围切换+归档保留 → Task 4/5（归档保留由 Task 4 的永久保留 + Task 5 全部档按 id 兜底显示天然覆盖，无需专门代码）。
- 类型一致性：`trend_cost` 前后端均为 snake_case（serde 默认）；`TrendSource.trend` 在 Task 2 后统一 `(number|null)[]`。
- 已知留白：请求数（requests）维度本版不做（DayMap 改动面大，收益低），活跃天数由 history 天数推导已够用。
