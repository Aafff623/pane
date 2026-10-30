# Spec: 退役旧身份的 Pane 自启项

**Status:** implemented 2026-10-29 — tests and real-machine migration verified; release pending · **Owner:** Aafff623

## 现象

用户点了“更新到 0.12.0”，更新器也装好了新版本，但每次开机仍提示更新到 0.12.0。实测机器上同时存在：

- 旧版 `Pane` 0.10.1：`D:\OneDrive\Desktop\tools\Pane\pane.exe`，`HKCU\...\Run\Pane` 指向它；
- 新版 `Pane-Aafff623` 0.12.0：`C:\Users\Lenovo\AppData\Local\Pane-Aafff623\pane.exe`；
- 更新清单（官网/GitHub）版本 0.12.0。

开机拉起旧版 → 旧版检查到 0.12.0 → 提示更新；点更新只装新身份，旧文件与旧自启项不动 → 下次开机重复。

## 根因

身份迁移（#105）把产品名从 `Pane` 改为 `Pane-Aafff623`。`setup()` 里已有迁移逻辑：读到旧配置的 `autostart=true` 时启用**新**自启项，并写 `identity-migrated` 标记；但它**从不删除旧的 `Run\Pane`**，且标记一旦写入就不会再执行迁移。旧值于是一直保留。

## 决定

1. 新增 `src-tauri/src/legacy_autostart.rs`：
   - 纯函数解析 Run 值里的 exe 路径（引号/带参数/无引号），判断它是否是 `pane.exe`，以及是否指向**当前 exe 之外**的路径；
   - Windows 下读取并删除 `HKCU\...\Run\Pane`，仅当目标确实是另一个 `pane.exe`；幂等、无标记。
   - 旧版会在自己的 `setup()` 末尾把自启项写回来，所以启动时先删一次，再在 12s / 42s / 132s 复检（累计）；删到一次就停。
2. 在 `setup()` 里每次启动调用，位于既有身份迁移块之前；不动 `Pane-Aafff623` 自启项，不动其他程序的 Run 值。
3. 日志：删除时 `eprintln!` 旧值，便于支持排查。
4. 不处理 HKLM（需要管理员权限）与启动文件夹快捷方式，写入 spec 边界。

## 验证

- 单元测试覆盖：带引号/带参数/无引号、大小写、路径分隔符、其他程序、当前 exe 自身。
- `cargo check` + `parse-tests` 全绿。
- 真机：临时写一个指向旧路径的 `Run\Pane`，启动新构建，确认被删除且 `Pane-Aafff623` 不变。
- 不发布、不推送；修复随下一个版本。
