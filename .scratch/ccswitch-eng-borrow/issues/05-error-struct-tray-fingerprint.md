Status: ready
Type: task
Blocked by: —

# 05 · 错误结构化 + 托盘问题态指纹重建

## 做什么

1. 新建 src-tauri/src/error.rs：AppError（thiserror：Io 带 path+source、Localized 中英变体）+ `From<AppError> for String`；结构化场景用 JSON 错误串 {code,context,suggestion}。先落 spend/探测失败两个高频点，其余命令渐进迁移。
2. rebuild_tray_menu（lib.rs:600）加问题态指纹：重建前比对「问题区状态」签名（问题集合的哈希），没变跳过重建；mark_seen 式记账防重复提示。

## 验收

- 错误卡片显示可读信息（含路径/建议）；托盘连续悬停/刷新不重复重建（日志计数验证）。
- `cargo +stable-x86_64-pc-windows-gnu check` 绿。
