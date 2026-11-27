# 07 看板、统计和分享

Status: ready-for-human

**Blocked by:** Windows 真机验收（03、04）

让看板、统计和分享接入通用控件。检查拖拽、布局、图表交互和实际 PNG。最后清理已无引用的旧实现，验证整套界面与 Demo。


## Comments

2026-11-27。看板的通用界面统一到同一份底座：分享预览改用外壳（此前它自己实现了 Tab
环绕和焦点归还，但缺背景滚动锁定和 IME 防护），状态预览弹窗补齐了焦点陷阱。图表侧
改的是配色口径——热力图四档从硬编码蓝改为 `color-mix(in oklab, var(--primary) …)`，
`--stats-c1` 跟随主色，其余系列色保留以便区分。新增 `tests/stats-charts.test.mjs`
（5 例）锁定平滑三次曲线、渐变 `currentColor` 和环形间隙。实际 PNG 导出、拖拽和
换肤下的观感待真机验收。
