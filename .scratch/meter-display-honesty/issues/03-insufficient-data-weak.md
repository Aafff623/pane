Status: done (2026-10-25)
Type: task
Blocked by: 01

# 03 · 「数据不足」分类型弱化

## 做什么

1. `renderTrend`（main.ts:2676-2760）`trend-nodata` 柱：加原因 tooltip（showTrendTip 扩充分支）——供应商未报比例 / 本日无用量 / 窗口未开始，每类一句人话。
2. progress 行 `used_percent===null` 的 fallback（main.ts:2571）复用同套理由文案。
3. `src/i18n.ts` 三语键表。
4. 参照 magpie `ledNoSpeed_` 六类（失败请求/无时长/无输出/未流式/首 token 后太少/突发到达）做 Pane 版子集——Pane 数据来自 provider 快照，可用类别以快照字段为准，不硬凑六类。

## 验收

- `pnpm build` 全绿；UI 验收（用户本人）：无数据柱有原因文案，progress 行 fallback 一致。
