# Spec: 看板数据诚实（未知≠0% / pace 文字 / 数据不足弱化）

## 背景

Pane 的 pace 数学（`src-tauri/src/pace.rs:41-104`：projected = used/窗口已过比例；≥100 over / ≥90 close / 否则 ahead；≥99.5 spent；未起步 not_started；太年轻 <15min 或 <5% unknown）与 QuotaBar 实质相同，UI 已有徽章+均速竖线（`src/main.ts:2570-2640` renderMetric）。不缺数学，缺的是「数据诚实」的一致裁决与解释。

## 现状缺陷（行号）

| # | 位置 | 缺陷 |
|---|---|---|
| 1 | main.ts:5675-5676（多账号 meter）、5687-5691（多账号 ring） | `window===null` 渲染 "0%"，title 写 "· 0%" |
| 2 | main.ts:3162 formatHoverMetric | `m.used_percent ?? 0`，hover 里 null 变 0% |
| 3 | main.ts:2676-2760 renderTrend | 无数据仅 `trend-zero`/`trend-nodata` 两态，无原因 |

环本体（5815-5845）、条（5970-5973）、auth-center（18997/19007）已正确用 —，说明是漏网。

## 方案

### 01 未知≠0%（先手，低成本）

三处 null 分支改渲染 —（与环/条/auth-center 现有行为对齐）；title/hover 同步。httpapi 已用 settled 区分，后端不动。

### 02 pace 文字标签 + 三段 tooltip

- 语义（QuotaBar `Pace.swift`）：`used_pct − elapsed_pct`，±10 个百分点容差 → on track；超出即 slow down / speed up。Pane pace.rs 已有 projected/Ahead/Close/Over，新增独立「快慢」维度（`PaceState` 扩展或脚注字段）。
- 呈现（QuotaBar `QuotaComponents.swift:190-230`）：over→火焰+"runs out in X"（run_out_at 已有）；close→琥珀 "~N% left at reset"；ahead→默认隐藏、偏好开才暗显；nil→不显示。Pane 现在 ahead 也常显徽章，收敛。
- tooltip（QuotaBar `projectionHelp` 式）："已过 X% · 已用 Y% · 预计重置时 Z%" 三段可验算。
- i18n：`src/i18n.ts:2718-2723` 三语键表。

### 03 「数据不足」分类型弱化

magpie 双支柱：弱化行（fill 0%、track 隐藏、按钮 disabled、文案 "No readings in this cycle"）+ 六类原因 tooltip（失败请求/无时长/无输出/未流式/首 token 后太少/突发到达，每类一句人话）。Pane 落点：
- renderTrend 的 `trend-nodata` 柱加原因 tooltip（showTrendTip 扩充分支：供应商未报比例 / 本日无用量 / 窗口未开始）。
- progress 行 `used_percent===null` 已有 fallback 到 metric-text（main.ts:2571），复用同套理由文案。

### 04（P2）每卡燃烧曲线——依赖后端采样

- 后端 `usage_history.rs`：新增窗口内采样点 `{at, left, start, resetsAt}`（参照 magpie `quota_history.go:43-49`：45 天保留 / 6000 点每窗 / 5 分钟采样 / 同值只留首尾 / `newCycle` 周期断裂判定），写入挂 lib.rs:3409 同链路；新 command 暴露。
- `pace.rs`：最小二乘斜率拟合（≥3 点且跨度 ≥1h 才给 ETA，外推至 0 得 run-out 时刻）。
- 前端 renderMetric 内嵌 SVG（参照 magpie `drawQuotaPlot` app.js:16438：0/50/100 网格 + 实线 + 匀速虚线[start→reset] + now 竖线）；ETA 文案 "Runs out {when} at this pace" / "Lasts past the reset"。
- 注意：新增持久化文件或新 meter shape；WebView2 缓存需按 AGENTS.md 清理。

## 验收

- 01-03：`pnpm build` + `pnpm test` 全绿；UI 验收（用户本人）确认三处 — 与 tooltip 三语。
- 04：后端采样落盘可查；UI 验收曲线渲染；ETag/缓存清理按 AGENTS.md。

## 不做

cycle 回看选择器、ResetMoment 扫动演出、双 horizon 字形、色盲 dash 编码（Pane 每卡通常单曲线，加 legend 足够）、balanceCurve 余额曲线。
