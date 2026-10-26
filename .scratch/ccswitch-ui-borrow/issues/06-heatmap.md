Status: superseded by #64 stats page (2026-10-26)
Type: task
Blocked by: 05

# 06 · 53 周热力图 + 偏好记忆

## 做什么

1. 热力图：53 周×7 天 CSS grid（`gridTemplateColumns: max-content repeat(53,1fr)`，UsageHeatmap.tsx:357 写法直抄）+ 四分位 5 色档（--heat-1..4 CSS 变量入主题）+ 悬停自绘 tooltip 卡；range=all 时替换趋势图（cc-switch 同语义），可展开多段。
2. 偏好记忆：localStorage 记 tab/指标/时间范围（rememberedChoice 思路），重进恢复。

## 验收

- 色阶四分位正确、悬停卡不溢出；365 天数据铺满 53 周；reduced-motion 无碍（热力图静态）。
- owner 视觉验收。
