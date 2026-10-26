# #77 桌面常驻卡片：预研结论（2026-10-26）

## 结论

**本轮不实现，保留 issue 作为下一版本候选。** 不投入 8h 运行时 spike 的原因不是能力不足，而是：
本批收尾已经包含 20+ 张票 + 一次发版，卡片档是 P3 形态实验，且依赖 #75 的 WDA（已完成）。
在未做 owner 视觉验收前把主窗口形态再扩展一层，会把发版风险从“数据正确”扩大到“窗口合成 + 常驻内存 + 多窗口生命周期”。

## 已核验的可行性（代码证据）

- `transparent(true)` 已有先例（panel 窗口）；`skip_taskbar` / `always_on_top` 已在用。
- `focusable(false)` 在 Tauri Windows 映射为 `WS_EX_NOACTIVATE`；本项目已有窗口创建封装。
- 分数坐标/工作区换算已有 `monitor.work_area()` 路径。
- 前端 `WINDOW_LABEL` 可直接加 `IS_CARD_FORM`；`renderCard`/`renderMetric`/skins 可复用。
- 数据：`cached_usage` + `usage-updated` + 独立低频轮询即可，不依赖 popover 显示。
- 与 popover/panel 是并存窗口，不做模式切换；配置可复用 `config.pinned`。

## 未核验 / 风险

1. WebView2 常驻窗口的内存占用与 Win11 合成性能（需要真实 8h spike）。
2. 卡片窗口在多显示器/DPI 切换、全屏应用、任务栏自动隐藏下的定位。
3. 截图排除（WDA）必须对卡片窗口同样生效，否则共享屏幕会泄露数字。
4. 三皮肤 + 明暗的卡片尺寸适配，以及窄卡片下的信息取舍（不能把所有卡片行塞进去）。

## 建议的下一版路径

1. 先做 1 天 spike：一个 `card` label 窗口，透明/置顶/不抢焦点，显示一张固定卡片，验证内存与合成。
2. spike 通过后再立实现票（24–40h），范围只做“单卡/多卡列 + 刷新”，不做编辑布局。
3. 若 spike 失败，保留 popover/panel 两形态，关闭本 issue。
