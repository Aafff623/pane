# Acceptance — 旧身份自启项退役（2026-10-29）

## 交付

- 新增 `src-tauri/src/legacy_autostart.rs`：解析 Run 值、识别 `pane.exe`、仅在目标不是当前 exe 时删除 `HKCU\...\Run\Pane`。
- `run()` 的 `setup()` 每次启动调用；不动 `Pane-Aafff623`，不动其他程序的 Run 值，不碰 HKLM/启动文件夹。
- 删除时写一行 `[pane] retired legacy autostart entry: …` 日志，便于支持排查。

## 验证

| 项 | 结果 |
|---|---|
| `cargo check` | 通过 |
| `parse-tests` | **1207 passed / 0 failed / 18 ignored**（新增 3 条：引号与参数、只识别 pane.exe、只退役其他安装） |
| 真机迁移 | 临时写入 `Run\Pane = D:\OneDrive\Desktop\tools\Pane\pane.exe`，启动新构建后该值被删除；`Pane-Aafff623` 未被误删（测试时被身份迁移指到 debug，随后已恢复为安装版路径） |
| 当前机器 | 两个 Run 值：仅 `Pane-Aafff623 → C:\Users\Lenovo\AppData\Local\Pane-Aafff623\pane.exe` |

## 边界

- 只处理当前用户的 `HKCU` Run 键；HKLM 需要管理员权限，启动文件夹快捷方式未处理。
- 只删除文件名是 `pane.exe` 且路径不等于当前 exe 的值；同名其他程序不会被误删。
- 修复要随下一个安装包才会到达已安装用户；本轮不发布、不推送。

## Review 后补强（2026-10-29）

独立 code review 发现旧版会在自己的启动流程末尾把 `Run\Pane` 写回去，单次删除会被撤销。处理：

- `setup()` 在早期删除之后，累计 12s / 42s / 132s 复检；删到一次即停止。
- 真机模拟：新构建启动 2 秒后再写入旧值（模拟旧版写回），15 秒后检查已被删除。
- 同时修掉 review 的边界问题：缺失闭合引号仍能识别、`%VAR%` 目标不误删、存在时用 canonicalize 比较 junction/短路径、删除失败按错误类型记录日志、顺手清理 `StartupApproved\Run\Pane`。
