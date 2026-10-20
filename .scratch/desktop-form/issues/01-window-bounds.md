# 01 — panel 窗默认尺寸与 bounds 记忆

**Blocked by:** None
**Status:** done (2026-10-20) — 已实测：拖到物理 (150,150) → 350ms 防抖写入 `panel-window.json` → WM_CLOSE → 重开恢复到同一位置，尺寸 1350×900 逻辑 / 2025×1350 物理。

`ensure_panel_window` 目前固定 960×640 + 居中。改为：

- 默认 `LogicalSize(1350, 900)`，`min_inner_size(960, 640)`；无保存状态时 `.center()`。
- 保存状态放在 `providers::config_dir()/panel-window.json`：`{x, y, width, height}`（物理像素，outer 位置 + inner 尺寸）。
- 事件：`Moved`/`Resized` 350ms 合并后写一次；`CloseRequested` 立即写；`close_panel_window` 关闭前也写一次。
- 恢复：与 `available_monitors()` 求交，交集 ≥120×80 才采用；宽高 clamp 到该显示器尺寸（不低于最小值）；否则返回 None 走居中。
- 最小化时不写入（避免 -32000 坐标）。

**验收**
- [ ] 首次打开约 1350×900 且居中
- [ ] 移动/缩放 → 关闭 → 重开位置尺寸一致
- [ ] 保存文件内容非法 / 显示器拔掉 → 回退居中，不崩溃
- [ ] `cargo check` 通过（GNU toolchain）
