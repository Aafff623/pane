<div align="center">
  <img src="src/assets/pane-icon.png" width="128" height="128" alt="Pane icon" />
  <h1>Pane</h1>
  <p><strong>Every AI quota. One calm command center.</strong></p>
  <p>一个常驻 Windows 托盘的 AI 用量看板：把不同厂商的额度、重置时间、账号池和本地消费，收进一个清晰的悬浮面板。</p>

  <a href="https://github.com/Aafff623/pane/releases/latest"><img src="https://img.shields.io/github/v/release/Aafff623/pane?style=flat&label=Release&color=2563eb&labelColor=172033" alt="Latest release" /></a>
  <img src="https://img.shields.io/badge/Windows-supported-0b1220?style=flat&logo=windows&logoColor=white" alt="Windows supported" />
  <img src="https://img.shields.io/badge/Tauri-v2-0b1220?style=flat&logo=tauri&logoColor=FFC131" alt="Tauri v2" />
  <img src="https://img.shields.io/badge/Data-local--first-0b1220?style=flat&labelColor=172033&color=2563eb" alt="Local first" />
  <a href="https://github.com/Aafff623/pane/blob/main/LICENSE"><img src="https://img.shields.io/github/license/Aafff623/pane?style=flat&labelColor=172033" alt="MIT license" /></a>
</div>

<p align="center">
  <a href="#为什么是-pane">为什么是 Pane</a> ·
  <a href="#它能做什么">它能做什么</a> ·
  <a href="#支持的来源">支持的来源</a> ·
  <a href="#快速开始">快速开始</a> ·
  <a href="#开发与验证">开发与验证</a>
</p>

> Pane 不替你调用模型，也不把数据上传到 Pane 服务。它只负责读取你已经授权的本地账号或 API key，向对应厂商查询额度，并把结果整理成一眼能读懂的状态。

## 📌 一句话说明

**Pane 是一个本地优先的 AI 订阅与额度看板。**

它把 Claude、Codex、Cursor、ClinePass、Copilot 以及更多 AI 工具的额度放进同一块悬浮面板：现在还剩多少、什么时候重置、哪个账号快到上限、今天花了多少，一次看清。

## 为什么是 Pane

AI 工具越来越多，真正打断工作流的常常发生在这些瞬间：

- 不知道当前订阅还剩多少，直到请求突然失败。
- 同一个 provider 有多个账号，却分不清哪张卡对应哪一个账号。
- 额度窗口、峰值时段、余额和本地消费分散在不同 CLI、网页和数据库里。
- 想快速确认状态，却要打开一堆设置页。

Pane 把这些信息压缩成一个安静的系统托盘工具。面板默认只展示最重要的状态，细节在需要时展开。

## 它能做什么

### 📊 配额总览

- 用绿色、黄色、红色状态快速区分可用、高峰和额度不可用。
- 统一展示 5 小时、日、周、月等滚动窗口，以及重置倒计时。
- 同一 provider 的多个账号显示为独立卡片，并在 provider 标题旁显示账号数量徽标。
- 查询失败会明确显示为错误状态，不再伪装成灰色的未知额度。
- 没有月额度的 provider 在月视图会回退到周额度，并明确标注回退状态。

### 🧾 本地消费

- 读取支持的本地 CLI 日志，计算今日、昨日和近 30 天消费。
- 支持美元与 token 视图，按模型展开明细。
- 当模型价格未知时保留 token 事实，并明确标注未计价项。

### 🧩 账号池

- API key provider 可以在设置中保存多个账号。
- 每个账号拥有稳定的本地卡片身份，删除或重排不会串号。
- 凭据只保存在当前 Windows 用户目录，并只发送到对应 provider 的官方或用户配置端点。

### 🎨 皮肤市场

- 在 Customize 上方打开皮肤入口，选择内置壁纸与吉祥物组合。
- 皮肤只增加视觉层，不改变原生主题、卡片数据和布局。
- 快速唤醒悬浮框时，吉祥物会围绕面板短暂出现。
- `Esc` 依次关闭预览、皮肤市场、Customize，最后交给现有面板隐藏逻辑。
- 皮肤市场支持一键还原原生皮肤；快捷键被系统占用时，设置页会显示冲突状态并保留上一次可用绑定。

## 支持的来源

Pane 当前覆盖 30+ 个 provider。下面列出最常用的几类，完整说明见 [Provider catalog](docs/providers.md)。

| 类型 | 来源 | 读取方式 |
| --- | --- | --- |
| 订阅额度 | Claude、Codex、Cursor、GitHub Copilot | 本地登录状态 + 官方 usage API |
| API key 额度 | DeepSeek、Kimi Code、StepFun、SiliconFlow、Novita | 设置中的 key、环境变量或 CLI 配置 |
| 多账号池 | ClinePass、One/New API、Custom Balance 等 | `accounts/<provider>.json` 中的独立账号 |
| 本地消费 | OpenCode、Claude Code、Codex CLI、Hermes 等 | 本地数据库或 CLI 日志 |
| 本地服务 | Ollama | `127.0.0.1:11434` 模型与服务状态 |

ClinePass 使用 `sk_` key 查询五小时、周、月三个滚动窗口；同一 provider 下的多个 key 会分别显示，并在卡片附近提示账号数量。

## 1 分钟快速开始

### 直接使用

1. 从 [Releases](https://github.com/Aafff623/pane/releases) 下载 Windows 版本。
2. 启动 Pane；它会进入系统托盘。
3. 使用全局快捷键或点击托盘图标唤醒悬浮面板。
4. 打开 Customize / Settings，按 provider 的提示完成本地登录或粘贴 API key。

Pane 不需要单独注册账号。首次运行后，数据会保存在当前用户的 `%APPDATA%\Pane\` 下。

### 从源码启动

```powershell
git clone https://github.com/Aafff623/pane.git
cd pane
pnpm install
pnpm dev
```

开发模式需要两个进程：Vite 在 `127.0.0.1:1420` 提供前端，`pane.exe` 在 `127.0.0.1:6736` 提供本地 usage API。完整的 Windows 启动和 WebView2 缓存说明见 [`docs/dev-startup.md`](docs/dev-startup.md)。

## 隐私边界

Pane 是 local-first 工具：

- token、cookie 和 API key 保存在 Windows 用户目录，不提交到 Git。
- 查询请求只发送到对应 provider 的端点；Pane 没有中心后端替你转发额度。
- 本地 HTTP API 只监听 loopback，并对敏感字段做脱敏处理。
- 遥测默认不承载额度、消费或 provider 凭据；可在设置中关闭。

详见 [`docs/privacy.md`](docs/privacy.md)。

## 开发与验证

### 前端

```powershell
pnpm build
```

这一步执行 TypeScript 检查并构建 Vite 生产资源。

### Rust

```powershell
cd src-tauri
$env:PATH = "D:\Tools\mingw64\bin;$env:PATH"
cargo +stable-x86_64-pc-windows-gnu check
```

完整 release 构建和可见桌面启动方式见 [`docs/dev-startup.md`](docs/dev-startup.md)。

### Provider 解析测试

```powershell
cd parse-tests
$env:PATH = "D:\Tools\mingw64\bin;$env:PATH"
cargo +stable-x86_64-pc-windows-gnu test
```

## 项目地图

| 位置 | 作用 |
| --- | --- |
| `src/main.ts` | 看板渲染、刷新循环、设置、皮肤市场 |
| `src/providerCatalog.ts` | 前端 provider 能力目录 |
| `src/providerVisuals.ts` | provider 图标、颜色和品牌视觉 |
| `src-tauri/src/lib.rs` | Tauri commands、快照缓存、账号交换保护 |
| `src-tauri/src/providers/` | 各 provider 的额度查询适配器 |
| `src-tauri/src/accounts.rs` | API key 账号池和稳定卡片身份 |
| `docs/` | 启动、隐私、provider 和设计文档 |

## 文档入口

- [开发启动指南](docs/dev-startup.md) — Windows 双进程、缓存和 pane.exe 启动
- [Provider catalog](docs/providers.md) — provider 读取方式与字段说明
- [皮肤市场设计](docs/skin-market-design.md) — 壁纸、吉祥物和退出交互
- [本地 HTTP API](docs/local-http-api.md) — `127.0.0.1:6736/v1/usage`
- [隐私说明](docs/privacy.md) — 凭据、网络请求和遥测边界
- [CONTEXT.md](CONTEXT.md) — 已验证的领域事实与工程约束

## 参与贡献

欢迎提交 issue、provider 适配、UI 改进和文档修订。请在提交前说明：

1. 改动影响的 provider 或用户路径。
2. 使用了哪些本地数据或外部 API。
3. 如何验证，哪些部分仍需要手动验收。

## License

[MIT](LICENSE)
