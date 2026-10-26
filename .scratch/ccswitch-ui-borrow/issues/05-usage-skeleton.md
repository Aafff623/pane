Status: superseded by #64 stats page (2026-10-26)
Type: task
Blocked by: —

# 05 · 用量统计页骨架（时间范围 + 趋势 SVG + Hero 成本/Tokens）

## 做什么

1. 新 overlay 页「用量统计」：页头（标题+同步时间提示可选）+ 筛选行（时间范围 today/1d/7d/14d/30d/all/custom，usageRange.ts:31 语义平译）+ Hero 前两格（总成本/总 Tokens，collect()/fetch_spend 求和）+ 趋势图（fetch_spend_daily 365 天，内联 SVG 柱形/面积，**不引 recharts**，参照 main.ts:2779 趋势条画法升级）。
2. 渲染照 Pane 风格：renderXxx(): string 模板 + replaceChildren + 事件委托；状态模块级变量；i18n usage.* 关键 key 搬运（zh 文案 MIT 可直用）。

## 验收

- 切 7d/30d/all 图随动、金额与现有卡片对账一致；空态（无数据）有引导文案。
- `pnpm build` 绿；owner 视觉验收。
