Status: ready
Type: task
Blocked by: 02
Priority: P2

# 04 · 每卡燃烧曲线（后端窗口内采样 + pace 拟合 + 前端 SVG）

## 做什么

1. 后端 `src-tauri/src/usage_history.rs`：新增窗口内采样点 `{at, left, start, resetsAt}`——45 天保留 / 6000 点每窗 / 5 分钟采样 / 同值只留首尾 / `newCycle` 周期断裂判定（参照 magpie `internal/provider/quota_history.go:43-49`）；写入挂 lib.rs:3409 同链路；新 command 暴露给前端。
2. `src-tauri/src/pace.rs`：最小二乘斜率拟合（≥3 点且跨度 ≥1h 才给 ETA；外推至 0 得 run-out 时刻）。
3. 前端 renderMetric 内嵌 SVG：0/50/100 网格 + 实线（剩余%）+ 匀速虚线[start→reset] + now 竖线；ETA 文案 "Runs out {when} at this pace" / "Lasts past the reset"（三语）。

## 验收

- parse-tests：采样 newCycle/去首尾/6000 点上限单测；pace 拟合边界（<3 点或 <1h 拒答）。
- `pnpm build` 全绿；UI 验收（用户本人）曲线渲染正确、无数据卡走 03 的弱化态。
- WebView2 缓存按 AGENTS.md 清理后验收。

## 注意

会新增持久化文件或新 meter shape——与主对话确认存储位置（%APPDATA%\Pane\ 下）；色盲 dash 编码不做（单曲线加 legend 足够）。
