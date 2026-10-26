Status: ready
Issue: https://github.com/Aafff623/pane/issues/75
Priority: P2
Area: Privacy

# WDA 窗口级采集排除（共享屏幕时浮窗数字入屏）

## 问题

共享屏幕检测链齐全（screen_is_being_shared+share_watcher+配置开关），但**零窗口级消费**——popover/panel 弹出时用量数字照常进录屏/共享画面；检测盲区（浏览器标签共享、OBS 窗口采集）不触发 QUNS。

## 方案（详见 spec.md，证据 temp/research/20261025-final-sweep-7repos.md §1-8-A）

1. platform/windows.rs 加 set_capture_excluded：Win32 SetWindowDisplayAffinity(WDA_EXCLUDEFROMCAPTURE)（Win10 2004+；失败静默降级）。
2. share_watcher 循环内对 main/panel 两 label 生效+窗口显形路径补当前状态；**修 HIDE_WANT 只读一次的热切换缺陷**（循环内重读）。
3. 前端零改动（OS 级）；tooltip 补盲区说明。

## 验收

OBS 窗口采集+Teams 标签共享各截一次对比（数字区变黑、本地正常）；关闭恢复；旧 Win10 不崩；配置热切换生效；owner 场景验收。

## 不做

不做前端内容遮罩（挡用户自己）；不做全屏显示器采集的特判（用户本就不该开）。
