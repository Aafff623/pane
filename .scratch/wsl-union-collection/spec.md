# Spec: WSL/native 联合采集

**Status:** ready · **Priority:** P2 · **Area:** Spend
**Issue:** https://github.com/Aafff623/pane/issues/69
**Evidence:** temp/research/20261025-final-sweep-7repos.md §1-3（注入点/去重/实现坑/测试全设）

## 问题

spend.rs 各源全部 `dirs::home_dir()` 拼固定路径（claude :1940、codex :2668），**WSL 里跑的 Claude Code/Codex 用量完全扫不到**（Rust 侧 grep "wsl" 零命中）。TokenTracker/ccusage/codeburn 均有 WSL 读取。

## 范围（2 票）

**票 01 基建（spend::wsl，~2 天）**：新模块+漏斗 `scan_roots_for(rel)`=native home+每启用发行版 UNC 根；发行版枚举（`wsl.exe -l -v` 输出 **UTF-16LE+BOM+`*` 默认标记**要 utf16 解码；非零退出≠空用 `-l -q` 二次确认，仅 ENOENT=未装）；home 解析（一次 `wsl -d <d> -- pwd`+双缓存 OnceLock+持久化 TTL 1h）；三个纯函数（prefix_for_version：WSL1→\\wsl.localhost\、WSL2→\\wsl$\；parse_distro_list；posix_to_win：/mnt/c→C:\、/home→None）+ parse-tests wsl_consistency.rs；platform::run_hidden_bytes（UTF-16LE，platform/mod.rs:196/215 有守卫测试）；config `spend.wsl_mode`（native-only/both/wsl-first，默认 native-only）；UNC 独立目录预算（每 root 2k vs native 20k，:1106）；采集器不动。

**票 02 采集器接入（~2-3 天）**：claude/codex/copilot 先行（scan_roots_for+共享 collect_files 键去重+probe_error 走 copy+补拷 -journal+512MB 上限）；**同账号 WSL 日志并入默认卡扫描**（防 account_scan_roots uuid 去重丢弃，providers/mod.rs:631-657+claude.rs:124-146）；再批量推其余源；SourceSpec.roots 同步；默认切 both。

## 去重（核心设计）

cursor 以 path 为键（spend.rs:1434/1470）→同文件双路径=双算。双保险：①枚举期单前缀（prefix_for_version 纯函数，一次刷新只发一种 UNC）；②norm_key 键归一（语法归一/UNC 拆 (distro,posix)//mnt/c→native/canonicalize 兜底）；**cache/cursor 仍按原 path 键——每物理文件只进列表一次，零迁移**。

## 验收

- 纯函数单测（双 UNC 前缀同键、/mnt/c 与 native 同键、双拼写同文件只进一次）。
- 真 WSL 手测 7 项（未装/WSL1/WSL2 混排、home 在 /mnt/c、WAL 写入中、>2k 目录、冷启动耗时、三档 mode、双前缀双向可达不双算）。
- WSL 内跑一天 Claude Code，Pane 计数与 WSL 侧手工核对一致。

## 协调

与用户自实施的 scan-roots 配置域：本批只交付漏斗+norm_key，WSL 作为第一个 root 提供者；用户配置落地后向同漏斗 append 自动共用去重；本期只加 spend.wsl_mode，不占 scan-roots 键、不做配置 UI。cwd 归因 Pane 没有（grep 确认），不做重锚。
