# 01: Windows / Linux / macOS / Android 开发与验证方案

Type: task
Status: backlog
Blocked by: None
Source: <https://github.com/Aafff623/pane/issues/36>

## What to build

一份四平台开发/验证规划，不改代码：

- **Windows**：保持本机开发与本机验收；现有 NSIS release 流程不变。
- **Linux**：WSL2 或 Docker 构建 + Ubuntu CI；用 WSLg 或 Linux VM 做 GUI 冒烟；Tauri 的 WebKitGTK/GTK/AppIndicator 依赖一次装齐。
- **macOS**：GitHub Actions macOS runner 构建（0.8.9 已验证能产出 arm64 dmg）；远程/CI 截图验收；签名与 notarization 单独评估；不在 Windows 上虚拟 macOS。
- **Android（探索）**：Tauri 2 mobile 最小技术验证；Android Studio + 模拟器 + 安卓真机；先做只读/展示页，托盘、悬浮窗等桌面能力另设计；不进入 0.8.x 主线。

## 范围外

- iOS、HarmonyOS 暂不评估（Tauri 无官方 HarmonyOS 目标，未来需单独路线）。
- 移动端 UI 重设计。
- 版本号与时间承诺；何时启动由 owner 决定。

## Acceptance

- [ ] owner 确认路线：Windows 主线、Linux/macOS 走 CI、Android 探索。
- [ ] 产出平台差异清单：Windows 专用 API、托盘/窗口/路径/单实例/签名。
- [ ] 拆出 Linux、macOS、Android 各自可执行 ticket，写明验收环境。
- [ ] 明确 GitHub Actions 配额、macOS 签名、Android 发布前置条件。

## Notes

- 0.8.9 的 macOS/Linux 包只是 CI 自动产出，未实机验收；Windows 是唯一承诺支持的平台。
- 本机已有 Android SDK / adb；Android Studio 标准路径未发现，需核验是否只装了 SDK。
- WSL 当前只有 `docker-desktop` 发行版，没有 Ubuntu 发行版；Linux 构建环境需补。
- 仓库是 public，0.8.9 已在 GitHub Actions 成功构建三平台；Student Pack / Actions 分钟数配额待核实，不假设具体额度。

## Comments

- 2026-10-22：owner 确认四平台范围（Windows / Linux / macOS / Android），Android 下个版本探索；iOS / HarmonyOS 暂不管；不着急，进度暂定。GitHub issue #36 为讨论源。
