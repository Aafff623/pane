# 03 — 项目维度

Type: task
Status: implemented (2026-10-24) — automated green; owner visual acceptance pending
Blocked by: 01
Issue: #50

## What to build

- 复制 `%LOCALAPPDATA%\rtk\history.db` 与 `-wal`/`-shm` 到临时目录，只读打开；不直接读运行库。
- `commands` 按 `project_path` 汇总 today/7d/30d/all 的 saved/commands；时间戳是 RFC3339，按本地日期分桶。
- 归一化：去掉 `\?\`；过滤 `%TEMP%`、`%LOCALAPPDATA%`、`%APPDATA%`、`.qoder-cn`、`resources/app`、纯日期/长 GUID 目录、用户根目录；剩余 basename。
- 输出 Top 5 可识别项目 + `unattributed_saved`（临时/系统路径总和）；不显示完整路径。
- DB 锁/复制失败：项目区单独降级，不拖垮全局窗口。
- 单测：路径归一化、过滤、Top、未归属求和、DB 锁降级。

## Acceptance

- [x] 本机 Top 不再出现 `f628eddd` 这类 GUID；出现 `pane`、`Firefly`、`src-tauri` 等可识别项目。
- [x] Top + 其他项目 + 未归属 = 全局对应窗口（单测锁定；实测 46.2M 对得上）。
- [x] 不输出原始命令、完整路径或用户名。
- [x] DB 被锁/缺失时全局卡仍有数，项目区显示不可用（单测覆盖损坏/缺失库的降级）。

## Implementation notes (2026-10-24)

- 过滤 `.qoder-cn` 时不要求尾随反斜杠，避免 `C:\Users\…\.qoder-cn` 自身漏进 Top。
- 新增 `other_saved/other_commands`：Top 5 之外仍可识别的项目单独成「其他项目」行，不混进「未归属」。
- 「未归属」同时覆盖临时/应用数据命令，以及 RTK 历史库已无项目信息的旧记录；tooltip 写清两种情况。
- `scan_projects` 复制 DB+WAL 到临时目录后只读打开；损坏库返回错误，不 panic。
