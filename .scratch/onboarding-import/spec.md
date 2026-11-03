# Spec: 首启引导第二步——从同类工具导入配置

**Status:** implemented 2026-11-03 — 待构建/实机验证与 owner 验收 · **Owner:** Aafff623
**分支：** `codex/onboarding-import`

## 现象 / 诉求

首启已有第一步：`source_probe` 只读扫描环境变量、本机登录态、已存密钥，引导弹窗的第 1 步列出检测到的工具并勾选上板。缺第二步：从**同类软件**（cc-switch 等管理 AI 订阅/密钥的工具）读取配置文件，提取 provider 名称与 API key，与 Pane 的 provider 目录动态匹配，先展示"识别到了什么 + 连通性检测结果"，再由用户勾选导入。

## 第一步现状核查（结论：可用，补两处展示）

- 检测入口：`source_probe::probe_all()`（`src-tauri/src/source_probe.rs:95`）——只读：进程环境 + Windows 用户/系统环境、`stored_credential_exists`（登录账号 → 多账号 → 已存密钥）、安装证据（可执行文件/目录）。不执行 CLI、不发网络、不回传密钥值。
- 引导入口：`main.ts` 的 onboarding 第 1 步（`onboardingSourceRowsHtml`），默认勾选 = 已安装或已有凭据；状态标签优先取真实查询结果（`lastSnapshots`），其次凭据存在性。
- 已具备的连通性原语：`test_api_key(provider, key, base_url)`（`lib.rs:4298`）→ `key_snapshot_for`（`provider_dispatch.rs:111`），覆盖 39 个 key 型 family，**只探测不落盘**。
- 补强 1：凭据来源展示改用友好标签（`env: X` / `已保存的密钥` / `本机登录`），不再直接显示原始 `credentialSource`。
- 补强 2：第 1 步的"检测到 N 个"统计与列表保持同源，避免探针失败时列表为空但计数非零。

## 第二步：从同类工具导入（本票主体）

### 适配器

v1 只做 **cc-switch**（Windows 上最常见、纯文本配置、确实是"同类软件"）：

- 配置：`~/.cc-switch/config.json`（v2）。顶层按应用分区：`claude` / `codex` / `gemini` / `grok` / `kimi` / `opencode` / `hermes` / `mcode` / `agy` / `omp` / `pi` / `openclaw`，每区 `{providers: {id → Provider}, current}`。
- Provider：`{id, name, settingsConfig, websiteUrl?, category?}`；`settingsConfig` 形状随分区不同：
  - claude：`env.ANTHROPIC_AUTH_TOKEN | ANTHROPIC_API_KEY` + `env.ANTHROPIC_BASE_URL`
  - codex：`auth.OPENAI_API_KEY` + `config`（TOML 字符串里的 `base_url = "…"`）
  - gemini：`env.GEMINI_API_KEY | GOOGLE_API_KEY` + `env.GOOGLE_GEMINI_BASE_URL`
  - 其余分区：已知键名递归兜底（`apiKey`/`api_key`/`API_KEY` 等 + `baseUrl`/`base_url`/`endpoint`）
- 限制：文件 ≤ 4 MB；候选 ≤ 100；只读，不写回。
- 架构留出 `adapter` 接口，后续可加 Cherry Studio / Cline 等（它们的库是加密/LevelDB，v1 明确不做）。

### 动态匹配

候选 → Pane family 的匹配顺序：

1. **base URL host**（主）：按 `providerMechanisms.ts` 的 hosts 维护一份 Rust 侧的导入匹配表（deepseek / moonshot / zai / minimax / stepfun / openrouter / siliconflow / novita / aihubmix / kimi / commandcode / groq / mistral / chutes / longcat / elevenlabs / deepgram / poe / venice / factory / mimo / huggingface / bocha / tavily 等）。
2. **名称别名**（次）：`DeepSeek`/`深度求索`、`Kimi`、`GLM`/`智谱`/`z.ai`、`MiniMax`、`阶跃`/`StepFun`、`硅基流动`、`OpenRouter` 等。
3. 两者都未命中 → `unsupported`，展示但不可导入；官方 Anthropic / OpenAI / Gemini 的裸 key 也不算命中（Pane 对应卡片走本机登录，不是 key）。

只接受 `key_snapshot_for` 能探测的 family（`provider_dispatch::supports_key_probe`），避免"能导入但永远查不到数"。

### 连通性

- 扫描只读本地，**不发网络**；网络探测是独立步骤 `import_probe`。
- 每行在后台探测：`key_snapshot_for(family, key, base_url)`，8 秒超时；结果状态：`ok` / `needs_setup` / `rejected` / `error` / `unverified`（未匹配或超时）。
- 仅 `ok` 默认勾选；`unverified` 可由用户手动勾；`rejected` 默认不勾。

### 安全

- 候选密钥永远不进入前端：DTO 只带掩码（前 4 + 后 4）与稳定 id；导入时后端按 id 从本次扫描缓存里取原值。
- 日志与错误信息不含密钥；导入复用 `save_api_key`（系统凭据库优先，失败才落明文文件）。
- 已存在同 family 同密钥 → `duplicate`，扫描阶段即标出，导入时跳过（不覆盖现有配置）。

### 交互

- onboarding 由 4 步扩为 5 步：欢迎 → 检测本机工具 → **从其他工具导入** → 连接确认 → 快捷键。
- 第 3 步（新）：按来源应用分组，行 = 勾选框 + provider 名 + 目标 family 图标/名 + 掩码 key + 主机名 + 状态徽标；底部"重新扫描"；无候选时给出空态（"没有发现可导入的配置"）。
- 设置 → 常规 → 来源扫描行新增"从其他工具导入…"按钮，复用同一列表与导入逻辑。
- 导入结果中文案：成功 N 项、跳过（已存在）M 项、失败的行保留原因。

### 不做

- 不导入 OAuth 会话（只导 API key / 订阅 key）。
- 不导入未匹配的中转站（"匹配没问题才导"是本次边界）；中转站归入后续 One/New API 批量导入。
- 不写任何第三方软件的配置。

## 验证（2026-11-03）

- Rust：7 条 `import_sources` 单测覆盖 claude/codex/gemini 解析、TOML base URL、缺键、host 优先、最长别名、掩码、匹配 family 可写可探；`parse-tests` 1214 通过 / 0 失败 / 18 ignored（两次并行跑分别出现 login_accounts、oauth 的偶发失败，单独重跑与串行跑均通过，属既有并行/环境抖动；wall 代理测试同为已知抖动）。
- 前端：`tsc && vite build` 通过；三语 key 对齐 5/5。
- 后端：`cargo check` / `cargo build` 通过；debug 版实机启动正常，与安装版并存。
- 评审：两轮 code review，HIGH/MEDIUM 全部修复，第二轮 APPROVE。
- 待 owner：首启第 3 步与设置入口的真实 UI 验收（本机未装 cc-switch，空态与夹具路径需实机确认）；真实 cc-switch 配置的扫描效果需在装有该工具的机器上验证。
