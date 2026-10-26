Status: done (2026-10-26)
Type: task
Blocked by: —

# 01 · set_capture_excluded + share_watcher 接线 + HIDE_WANT 热切换

## 做什么

1. platform/windows.rs：`set_capture_excluded(window, exclude: bool)`（SetWindowDisplayAffinity WDA_EXCLUDEFROMCAPTURE/WDA_NONE；照 hide_window_border :360 cfg+no-op 模式；旧 Win10 静默降级）。
2. lib.rs share_watcher 循环：HIDE_WANT 改循环内重读（修热切换）；对 main/panel 两 label 调 set_capture_excluded；窗口显形路径（apply_popover_pin :6263 模式）补当前状态。
3. 设置页 tooltip 补盲区说明（i18n 三语）。

## 验收

OBS+Teams 各截一次对比通过；关闭恢复；旧 Win10 不崩；配置热切换回归；owner 场景验收。
