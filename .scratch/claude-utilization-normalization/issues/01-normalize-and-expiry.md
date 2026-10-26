Status: done (2026-10-26)
Type: task
Blocked by: —

# 01 · utilization 归一 + 过期置空 + pace/alerts 纵深防御

## 做什么

1. claude.rs：`normalize_utilization`（规则见 spec）；push_window（:375-380）与 limits 路径（:321-328）接入；过期→text "Window expired — awaiting refresh"，归一失败→text "Usage unavailable"。
2. pace.rs compute（:70-88）：used>=99.5 短路**之前**插 `if resets_at <= now_ms → unknown()`。
3. alerts.rs：verdict（:45-73）过期检查移到 left<0.5 之前；evaluate :118 almost_now 加 !expired。
4. parse-tests 11 条（spec 测试节）。

## 验收

- parse-tests 全绿；pace/alerts 现有测试零回归。
- 本机：对过期 16.57 天的 commandcode Weekly 确认不再显示 42%、不再报「将会用完」；utilization=1.0 显示 1%。
- owner UI 验收：刷新前 boot cache 旧值+chip；刷新后 text 行文案三语。

## 证据

temp/research/20261025-final-sweep-7repos.md §1-1（含本机 21/80 过期行数据与 11 条测试清单）。
