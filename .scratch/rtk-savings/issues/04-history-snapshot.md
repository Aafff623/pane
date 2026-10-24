# 04 — 长期快照

Type: task
Status: implemented (2026-10-24) — automated green; owner visual acceptance pending
Blocked by: 01
Issue: #50

## What to build

- `%APPDATA%\Pane\rtk_savings_history.json`，版本化文件：`days[date] = {saved_tokens, commands, input_tokens, output_tokens, savings_pct, captured_at}`。
- 每次刷新把 CLI `daily` 合并进去：同日**覆盖**，绝不累加；缺失日保留。
- 读取时优先用快照补 30 天 sparkline 和 7d/30d 窗口；RTK 已有日期以本次为准。
- 原子写（tmp + rename），坏文件不 panic、不阻塞卡片；最多保留 400 天。
- 单测：同日重复刷新不翻倍、旧日保留、坏文件降级、窗口合计。

## Acceptance

- [x] 连续刷新两次，同日 saved 不变（单测）。
- [x] 手动移除 RTK 旧行后，Pane 快照仍能补出旧日（单测：CLI daily 为空时快照日仍出现）。
- [x] 坏 JSON/权限失败只影响快照，不影响全局 CLI 卡（坏文件另存 `.json.bad`，卡片显示一行提示）。
- [x] 定向 `rtk_savings` 单测 + `pnpm build` + node 测试全绿；全量 parse-tests 在最终一遍跑完后回填。

## Implementation notes (2026-10-24)

- 合并抽成纯函数 `merge_into`，测试直接覆盖同日覆盖语义，不再复制一份测试用实现。
- 读取到坏文件时改名为 `rtk_savings_history.json.bad` 再继续，不静默覆盖旧快照；卡片用一行「这次本地快照不可用」说明。
- 保留期 400 天按文件里最大日期裁切；原子写仍是 tmp + rename。
