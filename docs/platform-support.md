# Pane 四平台开发与验证方案

Status: planning only (no platform code in this batch)
Source: GitHub #36; owner confirmed the route on 2026-10-22.

## 结论

Windows 仍是唯一承诺支持、且唯一在本机开发和验收的平台。Linux 与 macOS
已经有 CI 自动产出的安装包，但没有实机验收；Android 只在下一个版本做技术
探索，不进入 0.8.x 主线。iOS 与 HarmonyOS 暂不评估。

这份文档只定义路线、差异和验收环境，不承诺版本号或时间。

## 平台差异矩阵

| 能力 | Windows | macOS | Linux | Android |
|---|---|---|---|---|
| 当前状态 | 本机开发 + 本机验收；NSIS | CI 产出 arm64 dmg；无实机验收 | CI 产出 AppImage/deb；无实机验收 | 未接入 |
| 构建 | 本机 GNU toolchain / CI windows-latest | GitHub Actions macos-latest | GitHub Actions ubuntu-22.04 | Tauri 2 mobile，需 Android SDK |
| WebView | WebView2 | WKWebView | WebKitGTK 4.1 | 系统 WebView |
| 托盘 | Tauri tray-icon；Windows 原生 | Tauri tray；菜单栏形态未验 | AppIndicator；需要桌面环境支持 | 无托盘概念 |
| 悬浮窗 | 无边框、置顶、skipTaskbar、拖动 | 同上，窗口层级未验 | 同上，合成器差异未验 | 无悬浮窗；需重新设计入口 |
| 单实例 | tauri-plugin-single-instance | 同左，未实机验 | 同左，未实机验 | 单 Activity |
| 开机启动 | tauri-plugin-autostart | LaunchAgent 形态未验 | .desktop autostart 未验 | 不适用 |
| 凭据 | Credential Manager + DPAPI | keychain（`security`） | Secret Service（`secret-tool`） | 需另设计 |
| 系统语言 | 显示语言 API | `defaults read -g AppleLanguages` | `LC_ALL` / `LC_MESSAGES` / `LANG` | 系统 Locale |
| 配置目录 | `%APPDATA%\PaneAafff623` | `~/Library/Application Support` | `~/.config` | 应用私有目录 |
| 本地数据 | `%LOCALAPPDATA%` | 同 Application Support | `~/.local/share` | 应用私有目录 |
| 窗口修饰 | 去掉 DWM 焦点描边 | 无此问题 | 无此问题 | 不适用 |
| 发布产物 | `*-setup.exe` | `*.dmg`（aarch64） | `*.AppImage` / `*.deb`（amd64） | 待定 |
| 签名 | 未配置代码签名 | 未配置签名/notarization | 未签名 | 待定 |
| 更新签名 | minisign（`TAURI_SIGNING_PRIVATE_KEY`） | 同上 | 同上 | 待定 |

## Windows

保持现有主线：

- 本机开发、本机验收，`temp/scripts/restart-pane-dev.ps1` 是唯一开发启动路径。
- NSIS 发布流程不变；`tauri.conf.json` 的 bundle targets 为 `all`，CI 只取
  `nsis`。
- 不把 Windows 专用实现（DPAPI、Credential Manager、DWM 描边、隐藏控制台）
  抽成跨平台抽象，除非另一平台确实需要同一职责。

## Linux

目标：可重复构建 + 至少一次真实桌面冒烟，不承诺完整功能对等。

- 构建：优先 WSL2/Docker + Ubuntu 22.04；CI 已用 `ubuntu-22.04`。
- 依赖：`libwebkit2gtk-4.1-dev`、`libayatana-appindicator3-dev`、
  `librsvg2-dev`、`patchelf`（release.yml 已装）。
- 冒烟环境：WSLg 或一台真实 Linux 桌面；检查托盘、悬浮窗、单实例、
  autostart、路径、Secret Service 回退。
- 当前缺口：本机 WSL 只有 `docker-desktop` 发行版，没有 Ubuntu；需要补一个
  Ubuntu 发行版或使用 Docker。
- 验收：至少一次真实桌面启动、托盘唤出、关闭/重开、AppImage 与 deb 各装一次。

## macOS

目标：CI 构建可重复，验收靠远程/CI 截图，签名单独评估。

- 构建：GitHub Actions `macos-latest`，产出 arm64 dmg；不尝试在 Windows 上
  虚拟 macOS。
- 冒烟：需要一台真实 Mac 或可信的远程会话；检查菜单栏托盘、窗口层级、
  keychain、单实例、autostart、Retina 缩放。
- 签名与 notarization：当前没有 Apple 证书；先确认是否公开发布，再决定
  是否购买 Developer ID。未签名 dmg 必须明确标注，不伪装成已签名。
- 更新：macOS updater 使用 `.app.tar.gz`，与 dmg 是两条产物，不能混用。

## Android（探索）

目标：Tauri 2 最小技术验证，不做桌面功能对等。

- 先做只读/展示页：读取一个假数据源或已有快照，证明 WebView、网络、存储、
  打包链路可用。
- 托盘、悬浮窗、置顶、skipTaskbar、单实例、autostart 在 Android 没有对应
  语义，另设计入口（通知、桌面小组件或应用内页）。
- 环境：本机已有 Android SDK/adb；Android Studio 标准路径未发现，需要核实
  是否只装了 SDK。需要模拟器或真机。
- 不进入 0.8.x 主线，不承诺发布时间。

## CI 与发布前置条件

- GitHub Actions 配额：仓库是 public，但 Student Pack / Actions 分钟数配额
  未核实；macOS runner 的分钟倍率更高。
- 现有 release.yml 已构建三平台；它只证明“能编出来”，不证明“能跑”。
- Windows 安装包未做代码签名；macOS 未签名/未 notarize；Linux 未签名。
- 发布资产命名以现有 release.yml 为准，不另造平台矩阵。
- 任何平台从“CI 产出”升级为“承诺支持”，都需要一次真实环境验收记录。

## 拆票

- `.scratch/platform-support/issues/02-platform-differences.md` — 逐项核对
  平台差异并在代码里标注 Windows-only 调用。
- `.scratch/platform-support/issues/03-linux-ci-and-smoke.md` — Linux 构建、
  依赖、WSLg/真机冒烟。
- `.scratch/platform-support/issues/04-macos-ci-and-signing.md` — macOS 构建、
  实机/远程验收、签名决策。
- `.scratch/platform-support/issues/05-android-exploration.md` — Tauri 2 最小
  只读验证。

## 不在本方案内

- iOS、HarmonyOS。
- 移动端 UI 重设计。
- 平台功能完全对等。
- 版本号、发布时间和商业承诺。
