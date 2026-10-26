Status: done (2026-10-26) — window-form pill (owner approved); sliding thumb for select pills and new pills.
Type: task
Blocked by: 08

# 09 · pill 分段控件大面积推广 + thumb 真滑动

## 做什么

1. `.st-radio-group`（窗口形态二选）：**先请 owner 拍板**（第一轮遗留），拍板后换 pill。
2. 08 的新设置项直接用 pill。
3. 盘点其余「互斥多选一」的 radio/checkbox，评估收编清单交 owner；布尔开关保留 switch（既定决策）。
4. thumb 真滑动升级：WAAPI animate transform+width 280ms --ease + ResizeObserver 重定位 + `.still`+`void offsetWidth` 免动画直放（约 60 行 JS；magpie app.js:9020-9047/9006-9019 同款；跨重绘续滑 Map 可省）。
5. 动态选项控件（#pinned 托盘显示）维持排除——选项运行时生成，pill 不适用（票 01 既定）。

## 验收

- thumb 滑动无布局抖动（性能面板）；切页后重定位正确（非当前页 offsetWidth=0 的坑，activatePanelPage 重测）。
- owner 视觉验收。

## Comments

- 2026-10-26: 实施与验证见 `temp/handoff/20261026-magpie-borrow-batch.md`；未 commit/push，UI 由 owner 验收。
