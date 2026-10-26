Status: done
Type: task
Blocked by: —

# 04 · 使用统计页壳 + 每日详情迁移

## 做什么

1. 设置面板左栏新增「使用统计」：PANEL_PAGES 加 `["stats","st-stats","settings.tabStats","panel.desc.stats","chart"]`（main.ts:17868-17876；**usage id 已被密钥页占用，必须用新 id**）+ panelBlock 建 stats 块 + i18n 三语 2 键。
2. 页面组装（对照 ZCode AppUsagePanel.tsx:34-131 布局）：KPI 摘要条 5 格（累计 Token/单日峰值/最长会话时长*/连续活跃/最长连续——*最长会话在票 06 前显示「—」，其余 lastSpendDaily 365 天 JS 聚合，streak 逻辑照 ZCode builder :117-135）+ 52 周 Token 热力图（扩展现有 .spend-heat-* CSS styles.css:2981-3067 到 52 列 minmax(0,1fr)；日/周/累计三 pill；月标签锚定每月 1 日列；四色阶+hover 缩放）+ 选日明细（spendDetailDayRows main.ts:4427 + Qoder 块 qoderDayHtml 原样迁入；热力格点击=内页选日）。
3. **删除悬浮窗 overlay**：renderSpendDetailOverlay（main.ts:4537-4623）与 data-spend-details 按钮（:4775/4973）、状态 spendDetailOpen、点击处理（:20429-20462）整体移除。

## 验收

- 设置页打开「使用统计」可见完整日详情（KPI+热力+选日明细），悬浮窗按钮消失无残留。
- 热力 52 周铺满、三 pill 切换、月标签正确；三皮肤+明暗正常。
- `pnpm build` 绿；owner 视觉验收（对照 ZCode 用量页截图）。
