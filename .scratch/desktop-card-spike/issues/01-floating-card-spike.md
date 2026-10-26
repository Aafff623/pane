Status: deferred (2026-10-26) — feasibility memo landed; implementation waits for a dedicated spike slot
Type: task
Blocked by: capture-exclusion-privacy（WDA 前置）

# 01 · 悬浮卡 spike：窗口 + 渲染子集 + 三域验证

## 做什么

1. 新窗口 card（transparent+skip_taskbar+focusable(false)+always_on_top）；分数坐标存取（monitor.work_area 换算；config 复用 pinned 结构）。
2. main.ts IS_CARD_FORM 分支：renderCard(:3462)/renderMetric(:2666) 子集+skins；数据 cached_usage+listen(usage-updated)+低频轮询。
3. 三域验证：WebView2 常驻内存占用；Win11 透明合成视觉（圆角/阴影）；焦点行为（WS_EX_NOACTIVATE 不抢焦点）与 popover 并存快捷键冲突。
4. 产出可行性报告+实现票范围建议（或「不做」结论）写回本票评论。

## 验收

能亮/能定位/能关；三域数据成文；owner 决策记录。预估 8h；实现票（若立）24-40h。
