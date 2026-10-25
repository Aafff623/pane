Status: ready
Issue: https://github.com/Aafff623/pane/issues/55
Priority: P2
Area: UI

# 看板数据诚实：未知≠0%、pace 文字标签与数据不足弱化

## 问题（本机代码排查，3 处真实缺陷）

1. **没有数据的账号显示成 "0%"**：`src/main.ts` 5675-5676（多账号 meter）与 5687-5691（多账号 ring）`pct = ... : 0`，`window===null` 直接渲染 "0%" 且 title 写 "· 0%"。环本体（5815-5845 else 分支）与条（5970-5973）已正确用 —，auth-center（18997/19007）也用 ———是漏网不是设计。
2. **hover 提示 null 变 0%**：`src/main.ts` 3162 `formatHoverMetric`：`Math.round(m.used_percent ?? 0)`。
3. **「数据不足」无解释**：`renderTrend`（2676-2760）只有 `trend-zero`/`trend-nodata` 两态，无原因文案；pace 徽章（2617-2619）在 ahead 时也常显，缺「相对窗口进度的快慢」维度。

## 方案（详见 spec.md）

- 01：3 处 null→0% 改 —（含 title/hover）。
- 02：pace 文字标签 slow down / on track / speed up（QuotaBar `pace()` 语义：used_pct − elapsed_pct，±10pp 容差）+ 三段可验算 tooltip（已过 X% · 已用 Y% · 预计重置时 Z%）+ i18n 三语。
- 03：「数据不足」分类型弱化（magpie `ledNoSpeed_` 六类：失败请求/无时长/无输出/未流式/首 token 后太少/突发到达；Pane 落点 renderTrend/showTrendTip + progress 行 fallback 2571）。
- 04（P2，依赖后端采样）：每卡燃烧曲线——后端 `usage_history.rs` 新增窗口内采样 {at,left,start,resetsAt}（45 天/6000 点/5 分钟/newCycle），pace.rs 最小二乘拟合（≥3 点且跨度 ≥1h 才给 ETA），前端 SVG（实线+匀速虚线[start→reset]+now 竖线+ETA 文案）。

## 验收

- `pnpm build` + 现有 `pnpm test` 全绿；三语 key parity（如已有测试）。
- UI 验收（用户本人）：无数据账号显示 — 而非 0%；pace 徽章与 tooltip 三语正确；趋势图无数据时有原因文案。

## 不做

- magpie 的 cycle 回看选择器（Pane 一卡 1-3 条 progress 行，价值低）。
- QuotaBar ResetMoment 扫动演出（popover 常驻刷新场景分散注意力，与 alert 通道重复；至多低调小 chip）。
- 双 horizon 字形、色盲 dash 编码（Pane 每卡通常单曲线，加 legend 足够）。

## 参考实现许可证

magpie-yetone（MIT）、QuotaBar（MIT）——可借鉴，抄代码带版权声明。
