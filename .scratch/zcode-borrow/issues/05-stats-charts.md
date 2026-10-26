Status: done
Type: task
Blocked by: 04

# 05 · 统计页图表层（范围 tabs + 按模型折线 + 模型 donut）

## 做什么

1. 范围 tabs：近 7 日/近 30 日 pill（前端过滤 lastSpendDaily）。
2. 每日按模型折线：**自绘 SVG 不引 Recharts**（复用 renderTrend main.ts:2704 模式）；top6 模型各一条非堆叠折线+图例；照抄 ZCode 四规则——Y 轴上限取可见序列单点峰值（非日总量）、tooltip 过滤 0 值序列、X 轴稀疏刻度（≤14 天全显，>45 天每 7 天）、数字紧凑单位（中文万/亿）等宽字体。
3. 模型分布 donut：自绘 SVG（innerRadius 56%、环心显示范围内总量）+ 右侧明细列表（色块+模型名+百分比+Token，>6 合并「其他」）；数据=范围内 DailySpendRow.models 聚合。
4. 右下角刷新按钮（重拉 fetch_spend_daily）。

## 验收

- 折线/donut 数字与选日明细对账一致；四规则边界单测（0 值过滤/稀疏刻度）。
- `pnpm build` 绿；owner 视觉验收（对照 ZCode 图表观感，无 Recharts 依赖）。
