Status: ready
Issue: https://github.com/Aafff623/pane/issues/69
Priority: P2
Area: Spend

# WSL/native 联合采集（基建 + 采集器接入）

## 问题

spend.rs 各源全部 home_dir() 固定路径，**WSL 里跑的 Claude Code/Codex 用量完全扫不到**（grep "wsl" 零命中）。

## 方案（详见 spec.md，证据 temp/research/20261025-final-sweep-7repos.md §1-3）

- 票 01 基建：spend::wsl 模块+漏斗 scan_roots_for(rel)；发行版枚举（UTF-16LE 输出/-l -q 二次确认/ENOENT=未装）；home 解析双缓存；三纯函数（prefix_for_version/parse_distro_list/posix_to_win）+parse-tests；run_hidden_bytes；config spend.wsl_mode（默认 native-only）；UNC 目录预算。
- 票 02 接入：claude/codex/copilot 先行（键去重+SQLite copy 补 journal+大小上限）；**同账号 WSL 日志并入默认卡**（防 uuid 去重丢弃）；批量推其余源；默认切 both。
- 去重双保险：单前缀枚举+norm_key 键归一；cache/cursor 按原 path 键零迁移。

## 验收

纯函数单测；真 WSL 手测 7 项；WSL 跑一天 Claude Code 计数与手工核对一致；双 UNC 前缀不双算。

## 不做

cwd 重锚（Pane 无项目归因）；跨进程/多用户；占用户 scan-roots 配置键（只交付漏斗接口）。
