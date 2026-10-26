# Spec: WDA 窗口级采集排除（共享屏幕隐私）

**Status:** ready · **Priority:** P2 · **Area:** Privacy
**Issue:** https://github.com/Aafff623/pane/issues/75
**Evidence:** temp/research/20261025-final-sweep-7repos.md §1-8-A（现状缺口+设计+验收）

## 问题

检测链齐全但**零窗口级消费**：platform/windows.rs:93-113 有 screen_is_being_shared()（SM_REMOTECONTROL+SHQueryUserNotificationState）；lib.rs:1222-1254 spawn_share_watcher 3s 轮询；lib.rs:1181 配置开关+main.ts:18527/18747 设置页——但 watcher 只重绘托盘，前端唯一消费者 main.ts:22573 只 requestTraySync()，**popover/panel 弹出时数字照常入屏**。检测盲区（浏览器标签页共享、OBS 窗口采集）不触发 QUNS——需要 OS 级窗口排除兜底。

## 范围（1 票，~3-4h）

1. platform/windows.rs 加 `set_capture_excluded(window, exclude)`：raw `SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE/WDA_NONE)`，照 hide_window_border（:360）的 `#[cfg(not(feature="harness"))]`+非 Windows no-op 模式；旧 Win10（无该 API）调用失败静默降级。
2. spawn_share_watcher 循环内对 main/panel 两 label 生效；窗口显形路径（apply_popover_pin :6263 模式）补一次当前状态。
3. **小改造**：HIDE_WANT 现在只在 spawn 时读一次（lib.rs:1223-1226），配置热切换不生效——改循环内重读。
4. 前端零改动（WDA 是 OS 级，数字不进采集帧）；不做前端内容遮罩（那挡的是用户自己）。
5. 设置页 tooltip 补盲区说明（全屏显示器采集 WDA 无效；OBS/Teams 标签共享走窗口位图采集，WDA 兜底）。

## 验收

- 开启共享时用 OBS 窗口采集 + Teams 标签共享各截一次对比（数字区域变黑/不可见，本地肉眼正常）。
- 关闭共享后恢复（WDA_NONE）；旧 Win10 静默降级不崩。
- 配置热切换生效（HIDE_WANT 循环内重读的回归）。
- owner 场景验收。

## 协调

是 desktop-card-spike（桌面卡片悬浮档）的前置组件——card 窗口开启共享时同样要 set_capture_excluded，先本票后 spike。与 #55/既有 share_watcher 检测链并存（检测做告警、排除做保底，两层互补）。
