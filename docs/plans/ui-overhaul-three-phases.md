# 三期 UI/UX 改造施工规划（左侧栏 · 齿轮配置 · OAuth · 多账号）

> 自用改造方向（已确认放弃"小 PR 上游"路线）。目标：把 Pane 的配置体验对齐 ccSwitch。
> 每期完成后跑验收（见 §4），通过再进下一期。
> 环境硬约束见 §5，动码前必读。

## Phase 1：左侧栏改造 + 问号说明 + 测试后保存

### 1.1 侧边栏 trail 图标化

- 现状：`index.html:90` `<nav id="trail">`；渲染在 `src/main.ts:2438`（`trailCards()` + `renderTrail()`），每个卡片渲染成 `<button class="trail-tick" data-trail="{i}">`（宽度随卡片高度拉伸的"音符条"），fisheye 效果在 `:2472 setupTrailFisheye()`。
- 改为：每卡渲染对应 provider 的官方图标（复用 `PROVIDER_ICONS`，main.ts:48）；无图标的 provider 回退为现有 tick 样式。结构：`<button class="trail-tick trail-icon" data-trail="{i}" title="{name}">{svg}</button>`。icon 从 main.ts 现有 import 体系走（`?raw` 内联），**新图标资产从 `/tmp/cc-switch-src/src/icons/extracted/` 复制到 `src/assets/providers/`**（映射见 §1.4）。
- 交互不变：点击滚动到对应卡片（现有 data-trail 委托）、fisheye 保留（icon 缩放跟随）。滚动联动高亮（`updateTrailActive`）保留。
- CSS：`.trail-icon svg` 尺寸钳制（约 14-18px），暗色适配（ccSwitch 图标多为深色底设计，必要时套 `filter: invert(1)` 或亮度调整——逐个目检）。

### 1.2 侧边栏按钮层级

- 现状：`index.html:91-96` `.sidebar-buttons` 顺序 = theme/refresh/customize/settings。
- 改为：`#theme-btn`、`#refresh` 移到 `#trail` **上方**（logo 圆环之下）；底部只留 customize + settings。纯 HTML 位置调整 + `.sidebar-buttons` 拆成上下两组（`.sidebar-buttons.top` / `.bottom`），CSS 对应调整。

### 1.3 自定义抽屉：问号说明按钮

- 位置：每行 ⚙ 齿轮左侧，`?` 按钮（`mini-btn`，i18n tooltip `customize.credInfo`）。
- 点击展开只读信息面板（与齿轮面板同款折叠样式），内容三行 + 当前状态：
  - **默认自动读取**：该 provider 无需配置就会探测的本地凭据（来自各 `src-tauri/src/providers/*.rs` 顶部 docstring 的事实，逐个核实后写成常量表 `PROVIDER_CRED_INFO` 放 main.ts）；
  - **支持的方式**：粘贴 API key / OAuth / 仅本地登录（三选多）；
  - **当前状态**：已存 key 与否（`stored_api_key` 有无——需要 §1.5 的新命令）；本地凭据文件是否存在（同命令返回）。
- 事实表初稿（实施时逐个对 docstring 校正）：claude=读 Claude Code 登录（支持多账号目录）；codex=读 Codex CLI 登录；cursor=本地 Cursor 配置；opencode=读 `auth.json` 的 opencode-go + 可贴 key；copilot=gh/Copilot 登录；grok=`~/.grok/auth.json`；devin=`%APPDATA%\devin\credentials.toml`；minimax=Settings key/env/CLI config；openrouter=Settings key 或 OpenCode auth.json；zai=Settings key/env/Z.ai CLI；antigravity=本地 language server / Credential Manager；deepseek=Settings key；moonshot=Settings key/env；elevenlabs=Settings key；ollama=本地服务无凭据；codebuff=codebuff 登录文件或 key；kilo=Kilo CLI 登录或 key；aihubmix=Settings key 或 OpenCode；qwen=Settings key/env；hermes=本地 ledger 无凭据；kimi=优先使用 KIMI_CODING_API_KEY/Settings key，没有 API key 时才用 CLI OAuth；stepfun/siliconflow/novita/relaybalance=纯 Settings key（relaybalance 另需 base URL）。

### 1.4 新 provider 图标资产

从 ccSwitch `src/icons/extracted/` 复制（PNG/SVG 混合，SVG 优先；`?raw` 只支持文本格式，PNG 走 `?inline` 的 base64 体系——参照 pane-logo 的 import 方式）：chatgpt→codex、claude、chatglm→zai、kimi、deepseek、siliconflow、stepfun、minimax、novita、openrouter、opencode-logo-light→opencode、copilot、grok、xai（备用）、qwen、ollama、hermes、aihubmix-color→aihubmix。缺图的（cursor/devin/antigravity/elevenlabs/codebuff/kilo/moonshot/relaybalance）沿用现状（无图标回退 tick/无 icon 槽），不强造。

### 1.5 齿轮面板：测试 → 保存 两步

- 新后端命令 `test_api_key(provider, key, base_url: Option<String>) -> Result<TestResult, String>`：
  - `TestResult = { ok: bool, metrics: usize, message: String }`；
  - 实现：为 16 个 key 白名单 provider 增加 `pub async fn snapshot_with_key(key: &str) -> Snapshot`（把现有 `fetch()` 重构为读 key 后调它——纯机械改造，逻辑不动）；`test_api_key` 按 provider 分发调用，**不落盘**；
  - relaybalance 的 base_url 走参数而非存储。
- 前端齿轮面板：按钮组改为「测试连接」+「保存」；测试成功（ok=true）才允许点保存（保存按钮禁用态直到测试通过）；测试结果显示在面板内一行（成功=绿色"N 项数据"，失败=红色错误原文）。key 变更时重置保存禁用。
- 验收：`npm run build` 通过；paste 一个真 key（如 StepFun）→ 测试 → 面板显示成功 → 保存 → 卡片出现。

## Phase 2：齿轮面板丰富化（凭据状态 + 面板重构）

### 2.1 新后端命令 `get_credential_status(provider) -> { storedKey: bool, envKey: bool, localCli: bool, localCliPaths: [String] }`

- `stored_api_key` 拆出纯探测函数（现 :372 已有 env 回落逻辑，拆开即可）；`localCli` 检测复用各 provider 现有 find_key 路径里"非 Settings 来源"的探测（逐 provider 提供 `pub fn local_credential_hint() -> Option<String>`，返回人话描述如 "已检测到 Claude Code 登录 (默认 profile)"）。没本地来源的返回 None。

### 2.2 面板重构（行内折叠，不弹窗）

- 齿轮面板改为两段：**状态区**（问号面板的内容合并进来：自动读取事实 + 实时状态 chips）+ **操作区**（key 输入 + 测试 + 保存）。⌗ 齿轮/问号合并：问号内容并入齿轮面板顶部，行上只留 ⚙ 一个按钮（? 按钮删除，避免双按钮拥挤——用户原话允许合并语义）。
- OAuth 类 provider 面板显示登录引导文案（Phase 3 前先只有文案）。

### 2.3 多账号展示（为 Phase 3 铺垫，只做展示）

- 状态区列出"已保存的账号/key"（含 label 与来源），删除按钮本期不做（Phase 3 一起）。

## Phase 3：OAuth（device code）+ 多账号

### 3.1 OAuth device-code（参照 ccSwitch `src-tauri/src/proxy/providers/codex_oauth_auth.rs` 与 `commands/auth.rs`）

- 范围：**codex + xai(grok) 两家**（ccSwitch 已验证的 device-code 端点；copilot 走 gh CLI 已够用；Claude 系是回调式 OAuth，本期不做）。
- 常量（照抄 ccSwitch `codex_oauth_auth.rs:36-67`）：CODEX `auth.openai.com/api/accounts/deviceauth/{usercode,token}` + verification `auth.openai.com/codex/device`；xAI 对应文件 `xai_oauth_auth.rs`。
- 新后端：`oauth_start(provider) -> {device_auth_id, user_code, verify_url}`（reqwest POST，不依赖 CLI）；`oauth_poll(provider, device_auth_id) -> {done, account_label?}`（前端 3s 轮询， token 成功后存 `%APPDATA%\Pane\oauth\<provider>.json`，结构含 access/refresh/expires）；`oauth_logout(provider)`。
- provider 接入：codex.rs/xai(grok).rs 的凭据解析加一级回落/并列来源（顺序：CLI 文件 → Pane 自有 OAuth → Settings key），刷新逻辑复用（token refresh 端点与 CLI 相同，写回 Pane 自己的文件，不动 CLI 的）。
- 前端：齿轮面板 OAuth 区块加「在浏览器登录」按钮 → `oauth_start` → 打开 verify_url（复用 `open_link`）→ 显示 user_code → 轮询至完成 → 刷新状态。
- 安全：token 只落本地盘；不进日志。

### 3.2 多账号（API key 维度）

- 存储：`%APPDATA%\Pane\accounts\<provider>.json` = `[{ "label": "...", "apiKey": "..." }]`（label 默认 "账号 N"）。
- 后端：`fetch_usage` 对配置了多账号的 provider，为每个账号 spawn `<provider>@<n>` 卡（复用 `snapshot_with_key`；完全复刻 claude 多账号 `claude@<hash8>` 的先例，lib.rs:1050 附近）。单账号保持现有 `<provider>.json` 行为不变（零迁移）。
- 前端：齿轮面板加「添加账号」→ label + key + 测试 + 保存；账号列表每项带删除；`<provider>@n` 卡在主页正常渲染、可拖拽排序。
- 范围：先覆盖 API key 余额类（deepseek/kimi/stepfun/siliconflow/novita/relaybalance），OAuth 账号多开后续再说。

## 4. 每期验收清单

1. `npm run build`（tsc+vite）零错误；
2. `cd src-tauri/target/parse-tests && cargo +stable-x86_64-pc-windows-gnu test` 全绿（涉及后端时）；
3. 重启 dev：`taskkill //IM pane.exe //F` 后 `RUSTUP_TOOLCHAIN=stable-x86_64-pc-windows-gnu npm run tauri dev`（后台）；
4. 功能逐条实测（API 可用 `http://127.0.0.1:6736/v1/usage` 核对）；
5. 用户截图目检 UI 细节。

## 5. 环境硬约束（动码前必读）

- 无 MSVC：构建一律 `RUSTUP_TOOLCHAIN=stable-x86_64-pc-windows-gnu`；
- `src-tauri/Cargo.toml` 的 `crate-type = ["rlib"]` 是本地调试临时改动，**不许还原、不许提交**；
- 不做 git 操作；
- 前端热重载偶尔会脏状态：改完 Rust 后 `taskkill //IM pane.exe //F` 让 tauri dev 自动重启；
- 测试 harness：`src-tauri/target/parse-tests`（`#[path]` 直接编译 provider 源文件）。
