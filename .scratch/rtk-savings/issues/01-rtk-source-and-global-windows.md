# 01 — RTK 数据源与全局窗口

Type: task
Status: implemented (2026-10-24) — automated green; owner visual acceptance pending
Blocked by: None
Issue: #50

## What to build

- `src-tauri/src/rtk_savings.rs`：调用 `rtk gain --all --format json`，10s 超时；解析 `summary/daily/weekly/monthly`，保留 60 秒内存缓存。
- 返回 `RtkSavings`：`status`（ok/missing/unreadable/scanning）、`windows`（today/7d/30d/all 的 saved/commands/rate）、`daily`（30 个本地日期，补 0）、`captured_at`。
- 定位 `rtk.exe`：优先 PATH；Windows 缺失时尝试 `~/.local/bin/rtk.exe`；不自动安装。
- 错误降级：缺失、非零退出、非 JSON、字段缺失都返回状态而不是 panic。
- `lib.rs` 新增 `fetch_rtk_savings` 命令并注册；不加入 `fetch_spend`。
- 纯函数 `parse_gain(&str)` + `windows_from_daily(...)` + 单测；bad JSON/旧版/缺失三态测试。

## Acceptance

- [x] 全局数字与本机 `rtk gain --all --format json` 对得上（实测 all 精确一致；today 两次采样间差一条命令属正常写入）。
- [x] today/7d/30d 用本地日期，all 用 summary；四个窗口来自同一快照。
- [x] RTK 缺失/坏 JSON 返回可读状态，命令不报错。
- [x] 连续调用两次在 60s 内不重复起进程。
- [x] `cargo check` + parse-tests 定向全绿。

## Implementation notes (2026-10-24)

- `RTK_BIN` 现在是权威覆盖：指向不存在/非文件时直接返回 missing，不再静默回退 PATH（也让三态可复现）。
- `rtk gain` 通过 `platform::hidden_command` 启动，满足「控制台子进程必须隐藏」的源码扫描测试。
- 前端在 `fetch_rtk_savings` 一落地就写入并重绘，不等 spend 扫描完成；冷启动时卡片不会卡在「正在读取」。
- 状态已实测：正常（45.9M / 1,008 命令）、`missing`、`unreadable` 三态均有截图；`scanning` 为短暂过渡。
