# 01 · 启动时清理旧 `Run\Pane`

Status: done 2026-10-29 | Priority: P0 | Depends: —

## Scope

- `src-tauri/src/legacy_autostart.rs`：Run 值路径解析、`pane.exe` 判定、是否退役（目标 ≠ 当前 exe）、Windows 下删除 `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\Pane`。
- `src-tauri/src/lib.rs`：`mod legacy_autostart;`，在 `run()` 的 `setup()` 最前面调用并记录旧值。
- `parse-tests/src/lib.rs`：用 `#[path]` 挂载模块，让单元测试跑在真实源码上。
- 边界：只处理 HKCU Run 的 `Pane`；不动 `Pane-Aafff623`、不动 HKLM/启动文件夹。

## Done when

- 纯函数单测覆盖引号、参数、大小写、分隔符、非 Pane、当前 exe 自身。
- `cargo check` 与 `parse-tests` 通过。
- 真机临时写旧值后启动一次即被删除，新自启项保持原样。
