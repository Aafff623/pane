# Spec: dev 构建与安装版互不干扰 + 显示名统一

**Status:** implemented 2026-11-03 — 待构建/实机验证与 owner 验收 · **Owner:** Aafff623

## 现象

- 开机后 Pane 窗口显示 WebView2 的 `ERR_CONNECTION_REFUSED`（`127.0.0.1:1420`）。
- 点桌面快捷方式「没反应」：快捷方式指向安装版 0.12.2，但开机自启项指向的是 debug 版
  `D:\code\pane\src-tauri\target\debug\pane.exe`，单实例把第二次启动转发给了那个跑坏的实例。
- 桌面快捷方式、安装包、GitHub 资产名都带 `Pane-Aafff623` 后缀，用户可见。

## 根因

1. 身份迁移块（`lib.rs` setup）：只要“当前 exe ≠ 上次记录的 exe”，且配置里 `autostart=true`，
   就调用 `autolaunch().enable()`，把自启指向**当前正在运行的可执行文件**。dev 与安装版交替运行
   时自启项被来回改写，最后一次是 debug。
2. `set_autostart` 命令与托盘 autostart 分支没有 dev 门禁（启动时的 re-assert 有）。
3. debug 与 release 共用 identifier `com.aafff623.pane`：单实例互斥量/消息窗口名、
   WebView2 数据目录都相同 → 互相转发、互相抢缓存。
4. productName `Pane-Aafff623` 同时决定安装目录、快捷方式名、安装包与 GitHub 资产名。

## 决定

1. **dev 不写系统自启**：新增 `autostart_manages_os_entry()`（`!cfg!(debug_assertions)`），
   统一守住迁移块、`set_autostart`、托盘分支、启动 re-assert 四处；dev 只持久化偏好，
   由安装版下次启动应用（偏好为 false 时安装版启动会 `disable()`）。
2. **dev 独立 identifier**：`src-tauri/build.rs` 在 `PROFILE=debug` 时把
   `{"identifier":"com.aafff623.pane.dev"}` 合并进 `TAURI_CONFIG`（同时 `cargo:rustc-env`
   给 `generate_context!`），隔离单实例、WebView2 目录（`%LOCALAPPDATA%\com.aafff623.pane.dev`）、
   托盘身份。配置目录 `%APPDATA%\PaneAafff623` 仍共享——dev 继续用真实账号调试。
3. **显示名统一（只改显示层）**：安装包/GitHub 资产在 release job 里
   `Pane-Aafff623_*` → `Pane_*`（`latest.json`、winget 正则、Gitee 镜像都读这些文件名，
   重命名必须先于清单生成）；快捷方式改名放在应用首次启动（`rename_install_shortcuts`）——
   GUI 安装的桌面图标在完成页才创建，晚于 NSIS POSTINSTALL，钩子改不到。
   NSIS `installerHooks` 只负责卸载：`$UpdateMode <> 1` 且 `IsShortcutTarget` 命中本安装时
   才删除 `Pane.lnk`（更新时保留，旧上游安装的入口不误删）。
   identifier、安装目录、卸载键、WinGet 包 ID 一律不动。

## 边界

- 不处理本地 API 端口 6736 的 dev/release 冲突（第二个实例只是 API 关闭，面板不受影响）。
- 不改进 WebView2 错误页本身（dev origin 守卫已存在）。
- 快捷方式改名不覆盖已存在的 `Pane.lnk`；旧上游安装的入口保持原样。
- 首次启动前（安装完成但未打开）快捷方式仍是 `Pane-Aafff623`。
- 安装器窗口标题仍显示 `Pane-Aafff623`（属于安装身份）。

## 验证

- `pnpm build`、`cargo check`、`parse-tests`。
- debug 构建：窗口类名应为 `com.aafff623.pane.dev-sic`，`%LOCALAPPDATA%\com.aafff623.pane.dev` 生成，
  运行 dev 不触碰 `HKCU\...\Run\Pane-Aafff623`。
- dev + 安装版同时运行：互相不转发，Alt+2 各自唤出自己的窗口。
- NSIS hook：`makensis` 语法检查；正式安装包行为随下一次 release dry-run 验证。
- 站点：`site npm test`。
