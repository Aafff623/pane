Status: done (2026-10-26)
Type: task
Blocked by: 01

# 02 · 采集器接入（claude/codex/copilot 先行）

## 做什么

1. claude/codex/copilot 采集器单 root 改 `for root in scan_roots_for(rel)`；文件列表经共享 collect_files（norm_key 去重：语法归一/UNC 拆 (distro,posix)//mnt/c→native/canonicalize 兜底；cache/cursor 仍按原 path 键）。
2. SQLite 源：with_db_copy 补拷 -journal+512MB 上限；probe_error（:46-64）直开原库必须改 copy。
3. **同账号 WSL 日志并入默认卡扫描**（account_scan_roots uuid 去重会丢，providers/mod.rs:631-657+claude.rs:124-146）。
4. SourceSpec.roots 同步改调漏斗（状态页显示 WSL 行）；验证后默认切 both；批量推其余源。

## 验收

- WSL 内跑一天 Claude Code，Pane 计数与 WSL 侧手工核对一致。
- 双 UNC 前缀（wsl.localhost/wsl$）双向可达下回归测试证明不双算。
- parse-tests 全绿；`cargo check` 绿；owner 场景验收。
