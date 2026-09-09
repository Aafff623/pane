# ADR 0002 — 操作系统能力走 `platform/` 接缝

- 状态：已接受（2026-09-09）
- 决策人：仓库所有者 Aafff623

## 背景

Pane 从 Windows 托盘应用长出来：凭据读 Credential Manager，语言读
Win32 UI language，Antigravity 用 PowerShell 扫进程和端口，WebView 内存
靠 WebView2 COM。这些调用散落在 `lib.rs`、`i18n.rs`、`providers/mod.rs`
和各 provider 里。要在同一个仓库打 Windows / macOS / Linux 包，无条件
依赖 `windows` crate 会让另两端直接编不过。

## 决策

所有「问操作系统要答案」的代码集中到 `src-tauri/src/platform/`：

1. `mod.rs` 是唯一对外 API；调用方不再写 `#[cfg(windows)]`，也不直接
   `use windows::`。
2. Windows 实现在 `windows.rs`；macOS 与 Linux 共用 `unix.rs`
   （`security` / `secret-tool`、`ps`、`lsof`/`ss`）。
3. 某端答不上来时降级为 `None` / 空列表 / `false`，由调用方走下一数据源。
   卡片显示「这台机器上没有」而不是编译失败或运行时崩。
4. `windows` / `webview2-com` / `windows-core` 只放在
   `[target.'cfg(windows)'.dependencies]`。
5. 发布流水线按矩阵打三端包（Windows NSIS、macOS dmg、Linux AppImage/deb）。
   产品承诺仍以 Windows 托盘为准；另两端是第一次公开发包，托盘定位、
   开机启动文案、Dock/AppIndicator 还没按各端习惯做完。

## 后果

- 正面：Linux/macOS 能编过；Win32 不再泄漏到 provider。
- 代价：托盘点击坐标、menubar、AppIndicator 仍按 Windows 假设工作，
  另两端的窗口位置和自动启动文案会暂时别扭。
- 不做：不为了「看起来跨平台」改 `com.jazii.pane` 数据目录，也不在本机
  GNU 工具链上假装能签官方 Windows 包——官方包仍由 GitHub Actions 的
  MSVC runner 出。

## 否决的替代

- 各文件继续堆 `#[cfg]`：三端一改就漏。
- 先只出 Windows、把接缝留到「真要移植」再抽：发布矩阵一旦加上另两端，
  散落的 Win32 会立刻把 CI 打红。
