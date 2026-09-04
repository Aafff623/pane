# Handoff：API Key 查额度（quota/balance）能力 PR

> 面向执行者的自包含实施文档。目标仓库：`D:\code\pane`（ItsJazii/pane 的 fork，本地 remote `Aafff623/pane`）。
> 所有行号基于 2026-09-02 的 main 分支核实；行号可能漂移，锚点以函数名/代码片段为准。

## 0. 背景与目标

Pane 是 Windows 托盘 AI 用量追踪器（Tauri 2 + 无框架 TS 前端）。目标是仿照 ccSwitch 的"粘贴 API key 就能查余额/套餐用量"能力，给 Pane 增加一批纯 API key 查询的 provider，最终 PR 回上游 ItsJazii/pane。

已定稿范围（与用户确认）：**6 个专属工程项 + 1 个通用工程项，覆盖 14 家厂商**，外加 1 个 zai.rs 修复项：

| # | 工程项 | 类型 | 端点 |
|---|--------|------|------|
| 1 | Kimi For Coding | 改造 `kimi.rs`（加 key 回落） | `GET api.kimi.com/coding/v1/usages` |
| 2 | StepFun（.com/.ai 合一） | 新文件 | `GET {host}/v1/accounts`（CNY 余额） |
| 3 | SiliconFlow（.cn/.com 合一） | 新文件 | `GET {host}/v1/user/info` |
| 4 | OpenCode Go | 改造 `opencode.rs`（接 Settings key，**不是新文件**） | `GET opencode.ai/zen/go/v1/usage` |
| 5 | Novita AI | 新文件 | `GET api.novita.ai/v3/user/balance`（单位 0.0001 USD，÷10000） |
| 6 | ZenMux | 可选，见 §7 | ccSwitch `coding_plan.rs` 有内置查询可参考 |
| 7 | 通用自定义余额查询 | 新文件 + 新设置区块（PR2） | `GET {base}[v1]/dashboard/billing/subscription` + `/usage` |
| 8 | **zai.rs 修复：GLM 5 小时窗口不显示重置时间** | 改造 `zai.rs` | 复用现有 `quota/limit` 响应里的 `nextResetTime` |

明确排除（不要顺手做）：火山（要控制台 AK/SK 签名）、仅站点访问令牌的中转站（RunAPI/ClaudeCN/Code0/TeamoRouter/Sudocode.us/APIKEY.FUN）、实测 404 的中转站（ZetaAPI/TheRouter/PPIO/AICodeMirror/PatewayAI/七牛云/Longcat/ModelScope/小米 MiMo 等）、云厂商与 OAuth 类。Pane 已有的不做：DeepSeek、OpenRouter、智谱、MiniMax、Moonshot、AiHubMix。

**PR 策略**：拆两个 PR。PR1 = 任务 1–5 + 8（纯模板克隆 + 两处小改造，低风险）；PR2 = 任务 7（通用 billing，引入了"用户填 base_url"的新配置面，和 Pane"每个厂商一个预设"的家规有张力，需要上游先认可）。

## 1. 架构事实（动手前必读）

Provider 没有 trait，是约定式：每个 provider 文件暴露 `pub async fn snapshot() -> Snapshot`。

- 核心结构 `src-tauri/src/providers/mod.rs`：
  - `Metric::progress(label, used_percent, detail)` :41；`Metric::text(label, value)` :54；`.with_reset(resets_at_ms, period_ms)` :66
  - `Snapshot::ok / no_credentials / error` :89 / :102 / :115
  - `http()` 共享 reqwest client :153；`json_body(resp, max_bytes, what)` 先读体再解析（区分读体中断与解析失败）
  - `credit_meter(provider, sign, balance)` :276 / `credit_meter_labeled(...)` :284——余额类进度条：按"历史最高余额"做高水位计费（自动处理充值），并接入 Almost Out 通知
  - `stored_api_key(provider, &[env_vars])` :372——读 `%APPDATA%\Pane\<provider>.json` 的 `apiKey`，回落环境变量
- 注册点（`src-tauri/src/lib.rs`）：
  - `fetch_usage` 里的 provider vec（约 :1004–1030），格式：`("id", Box::pin(guarded("id".into(), "Name".into(), providers::xxx::snapshot())))`
  - **`set_api_key` 硬编码白名单 :1427**——现有名单 `openrouter|zai|minimax|deepseek|moonshot|elevenlabs|codebuff|kilo|aihubmix|qwen`。新 provider 忘了加这里，Settings 保存会报 `unknown provider`。这是最容易漏的一处。
  - 新 tauri command 才需要登记 `invoke_handler`（约 :1737）；本次只有 PR2 可能需要
- 前端（`src/main.ts`）：
  - `ALL_PROVIDERS` :264（`[id, 显示名]` 对，决定 Customize 列表）
  - `Metric`/`Snapshot` TS 接口 :67–91
  - `saveApiKey(provider)` :3093——`#key-<id>` 输入框 + `[data-save]` 按钮自动接线（:3490 附近按 `data-save` 属性绑定），所以 index.html 只要按惯例加行即可
  - **重置时间渲染已内置**：`renderMetric` :948——`resets_at` 在未来才显示；且"≤6h 滚动窗口仍是满额 = 窗口未开始（首次使用才开窗），显示 not-started 而不是假倒计时"（i18n 键 `card.notStarted`/`card.resetsIn`/`card.resetsAt` 已存在）
- 模板文件：`providers/deepseek.rs`（69 行，最简完整模板：读 key → 无 key 返回 `no_credentials` → Bearer 调端点 → 出 `credit_meter` 条）。双域名依次尝试参考 `moonshot.rs`（`ENDPOINTS` 数组）或 `minimax.rs`（`ENDPOINTS` + `last_error` 循环 :84–100）。
- 错误语义约定：`Err(String)` = 瞬时网络错误（前端重试并保留上次成功值）；`Snapshot::error`/`no_credentials` = 确定性失败（空 key、401、解析失败）。

### 每个新 key-based provider 的完整改动清单（6 处）

1. 新建 `src-tauri/src/providers/<id>.rs`（照抄 deepseek.rs 骨架）
2. `src-tauri/src/providers/mod.rs:1` 区域加 `pub mod <id>;`
3. `lib.rs` 的 `fetch_usage` vec 加注册行（约 :1016 附近，按现有格式）
4. **`lib.rs:1427` `set_api_key` 白名单加 `"<id>"`**
5. `src/main.ts:264` `ALL_PROVIDERS` 加 `["<id>", "<Name>"]`；`index.html` API keys 手风琴区加一行（现有最后一行是 qwen，约 :278–285，照抄其结构：`<div class="key-row"><label for="key-<id>">…</label><input id="key-<id>" type="password" data-i18n-placeholder="settings.keyPh<X>" /><button data-save="<id>" data-i18n="settings.save">Save</button></div>`）
6. `src/i18n.ts` 三语补 `settings.keyPh<X>`（en :109 附近 / zh :418 附近 / ru :720 附近）+ `docs/providers.md` 加一节 + `README.md`  providers 表格（:152–180）加一行

每个 provider 附 `#[cfg(test)]` 解析单测（fixture JSON → 断言 Metric），仓库有测试惯例，上游 review 会看。

## 2. 任务 1：kimi.rs 加 API key 回落（~30 行）

现状：`kimi.rs` 的 `fetch()` :92 只在 `cred_path()` 找到 CLI OAuth 凭据文件时才工作，否则 `no_credentials("Kimi Code sign-in not found. Run kimi login…")`。`fetch_usages(access)` :141 对 `GET api.kimi.com/coding/v1/usages` 做 Bearer 认证；ccSwitch 已验证**同一端点直接接受 API key**（返回同样的 `limits[]`（5h 窗口）+ `usage`（周限）结构）。

改动（API key 优先、无 key 时使用 OAuth，现有 OAuth 用户仍兼容）：

1. `fetch()` 先尝试 `stored_api_key(ID, &["KIMI_CODING_API_KEY"])`：有 key 就直接走既有解析（`parse_snapshot`），跳过 `load_access`/刷新/写回整条 OAuth 路径；没有 key 才读取 CLI OAuth；两者都没有才返回 `no_credentials`。

   **实施后修正（实机调试发现）**：Kimi 的凭据来源按实际接入方式区分——Pane 中存在 Kimi Coding API key 时直接使用 API key；没有 API key 时才读取 CLI OAuth。OAuth 失效回落仍保留在兼容路径中，但状态面板只显示当前生效来源，不把 OAuth 文件存在误报为当前登录方式。
2. key 路径的 401 → `Snapshot::error`（key 无效，确定性的；不要走 OAuth 的刷新重试逻辑）。
3. `parse_snapshot` 里 `user.membership.level` 可能缺失（key 路径未验证有该字段）——plan 回落为 `"Kimi Coding"`。
4. 环境变量只用 `KIMI_CODING_API_KEY`：**不要**加 `KIMI_API_KEY`（那是 moonshot.rs 的钱包 key，两个平台不同）。
5. `lib.rs:1427` 白名单加 `"kimi"`；`index.html` 加 kimi key 行；`main.ts` 不用动（`["kimi", "Kimi Code"]` 已在）；i18n 加 `settings.keyPhKimi` 三语；更新 `kimi.rs` 顶部 docstring（:1–9）和 `docs/providers.md` Kimi Code 一节（:193–210）说明两种凭据。
6. 测试：把"凭据解析"与"用 token 拉取"拆成可单测的纯函数，测 key 回落优先级和 401 分支。

## 3. 任务 2/3/5：stepfun.rs / siliconflow.rs / novita.rs（各 ~70 行）

同一份配方，照 `deepseek.rs` 骨架 + `minimax.rs` 的多端点循环：

- **StepFun**：`ENDPOINTS = ["https://api.stepfun.com/v1/accounts", "https://api.stepfun.ai/v1/accounts"]`（.com 优先）。响应取 `balance`（CNY，另有 `total_cash_balance`/`total_voucher_balance` 可作 text 行）。`credit_meter(ID, "¥", balance)`。env：`STEPFUN_API_KEY`。
- **SiliconFlow**：`ENDPOINTS = ["https://api.siliconflow.cn/v1/user/info", "https://api.siliconflow.com/v1/user/info"]`（.cn 优先）。响应 `data.totalBalance`（另有 `data.chargeBalance`）。env：`SILICONFLOW_API_KEY`。
- **Novita AI**：单端点 `https://api.novita.ai/v3/user/balance`。响应 `availableBalance` **单位 0.0001 USD，必须 ÷10000**。env：`NOVITA_API_KEY`。

公共要点：`stored_api_key(ID, &[...])` → 无 key 返回 `no_credentials("Paste a <Name> API key in Settings (gear icon).")`（照抄 deepseek/zai 的措辞风格）→ `http().get(url).bearer_auth(key)` → 401/403 返回确定性 `error`（"API key was rejected — check it in Settings"，照抄 zai.rs :49）→ 体解析用 `super::json_body` → `Metric::text` 显示余额 + `credit_meter` 出进度条。6 处改动清单见 §1。

## 4. 任务 4：OpenCode Go——改 opencode.rs，**不要新建 provider**（重要纠偏）

大发现：Pane 的 `opencode.rs` **已经实现了完整的 OpenCode Go 官方查询**——`USAGE_URL = "https://opencode.ai/zen/go/v1/usage"` :15，`fetch_official(key)` 解析都在，本地 `opencode.db` 只是回落。唯一缺口是 key 来源写死了：`fetch()` :149 只读 OpenCode 编辑器 `auth.json` 的 `opencode-go` 条目（`auth_entry_key` :38），没装 OpenCode 编辑器的用户没法用。

改动（极小）：

1. `fetch()` 里 key 解析改为多来源回落：先 `auth_entry_key("opencode-go")`，再 `stored_api_key(ID, &["OPENCODE_GO_API_KEY"])`（顺序建议保持 auth.json 优先，理由同 kimi：现有用户零影响）。注意没有 auth.json 时 `:141` 的早退也要放宽——auth.json 不存在但 Settings 有 key 时应继续走 key 路径，而不是直接 `no_credentials`。
2. `no_credentials` 提示语补"or paste a Go key in Settings"。
3. `lib.rs:1427` 白名单加 `"opencode"`；`index.html` 加 opencode key 行；i18n 三语 `settings.keyPhOpencode`；更新 `opencode.rs` docstring（:1–20）与 `docs/providers.md` OpenCode 一节（:92–112）。

id 保持 `"opencode"`（ALL_PROVIDERS 已有），**不要**新建 `opencodego` 之类的新 id。

## 5. 任务 8（修复项）：zai.rs 的 5 小时窗口重置时间

用户痛点：GLM（智谱）卡的 5 小时额度条不显示什么时候重置，ccSwitch 里能看到。

结论（官方文档 + ccSwitch 源码注释 + 用户实测三方互证）：GLM Coding Plan 是 **5 小时滚动窗口 + 每周限额双周期**（z.ai 官方 FAQ 明示两个周期并存）；5 小时窗口在上一个窗口结束后**不会自动开新窗，首次使用才开始计时**——所以闲置账号的 5 小时桶**根本没有重置时间可显示**（`nextResetTime` 缺省），这不是 bug，显示"未开始"才对。真正的 bug 是 zai.rs **根本没把 `nextResetTime` 传进 Metric**。

现状：`zai.rs` 的 `collect_quota_metrics` :77 是容忍式解析。`TIME_LIMIT`（月度搜索额度）分支已经带了 `.with_reset(resets_at, Some(30 * 86_400_000))`（:94–105）；但 `TOKENS_LIMIT` 走的一般分支（:110–140）只出 `Metric::progress(&label, p, None)`——**没有 `resets_at`，也没有 `period_ms`**，前端自然没东西可显示。另外 `nice_label` :146 对 `"TOKENS_LIMIT"` 原样输出，两条桶会同名。

改动（`zai.rs` 为主 + 前端一处补漏：原结论"前端零改动"在实施后被修正——`renderMetric` 的"未开始"分支原本要求 `resets_at` 非空，GLM 闲置窗口压根没有 reset 时间可传，已在 main.ts 补上"`resets_at` 为 null + ≤6h 窗口 + 用量 ≤1% ⇒ 显示未开始"的分支；全仓只有 zai.rs 传 `with_reset(None, …)`，影响面锁定）：

1. 一般分支里读 `nextResetTime`（ms epoch，`> 0` 才有效）。
2. 按 `unit` 字段分类窗口（照 ccSwitch `classify_zhipu_window` 的实测结论）：`unit == 3` → 5 小时窗口（label "Session"，`period_ms = 5 * 3_600_000`）；`unit == 6` → 每周窗口（label "Weekly"，`period_ms = 7 * 86_400_000`，`number` 7 和 1 都实测存在，**只锚定 unit**）。
3. **严禁按 `nextResetTime` 升序排序来猜窗口**——周期末尾周窗会比 5h 窗先重置，排序必然标反（ccSwitch issue #3036 的教训）。`unit` 缺失时的兜底：无 `nextResetTime` 的条目优先当 five_hour，其余按 reset 升序填空槽。
4. 老套餐（2026-02-12 前订阅）只回 1 条 `TOKENS_LIMIT`，自然降级为只显示 5 小时条。
5. 5h 桶闲置（0%、无 `nextResetTime`）→ `with_reset(None, Some(5h))`，前端自动显示"尚未开始"。
6. 测试：fixture 覆盖 双 TOKENS_LIMIT（unit 3+6）、单条老套餐、5h 桶无 reset、unit 缺失走兜底 四类形态。

## 6. 任务 7（PR2）：通用自定义余额查询

一个实现覆盖 9 家实测开放了 OpenAI 兼容 billing 口的中转站：DMXAPI、PackyCode、Micu、CrazyRouter、SudoCode.chat、XycAi、E-FlowCode、CherryIN、AICodeWith。（实测依据：无凭据 GET 返回 401 JSON = 接口存在；`/api/user/self` 那条路要的是站点后台访问令牌，不是推理 key，不做。）

设计要点：

- provider id 建议 `"relaybalance"`，存储 `%APPDATA%\Pane\relaybalance.json` 扩为 `{"apiKey": "...", "baseUrl": "https://..."}`。`stored_api_key` 只读 `apiKey`，需要加一个读同文件 `baseUrl` 的小 helper。
- `set_api_key(provider, key)` 需要能同时存 base_url：最小改法是加可选参数 `base_url: Option<String>`，前端 `saveApiKey` 对该 provider 多传一个输入框的值；白名单加 `"relaybalance"`。
- 端点依次尝试 `{base}/v1/dashboard/billing/subscription` 和 `{base}/dashboard/billing/subscription`（有 /v1 和没有 /v1 两形态实测都存在），配 `/usage`。响应是 OpenAI 形态：subscription 的 `hard_limit_usd`，usage 的 `total_usage`（**单位是美分，÷100**），剩余 = hard_limit − total_usage/100。参照 `aihubmix.rs` 的现有实现。
- UI：Settings 里需要新区块（base_url + key 两个输入）。`index.html` 插在手风琴 Network 组之后（:219–226 是 Network 组末尾锚点）；i18n 三语键加在 `settings.networkNote` 附近（en :96 / zh :405 / ru :707）。`main.ts` 的 `initSettings` :3176 里给 base_url 输入框做加载/保存接线。
- 无 base_url 或无 key → `no_credentials` 提示两个都要填。

## 7. 任务 6（可选）：ZenMux

ccSwitch `coding_plan.rs` 有 `query_zenmux(base_url, api_key)`（Bearer key 查套餐额度）。截图预设里没有 ZenMux，优先级最低。要做的话先浅克隆 `farion1231/cc-switch` 读 `query_zenmux` 的端点与解析，再按 §3 配方实现。不做不影响 PR1 完整性。

## 8. 流程与验收

1. **先开 issue**（CONTRIBUTING 家规）：到 ItsJazii/pane 说明动机与方案，再动码。
2. 分支：从 main 拉 `feat/api-key-quota`；PR1（任务 1–5 + 8）先提，PR2（任务 7）单独提。
3. 验收：`cd src-tauri && cargo test` 全绿；`npm run build`（tsc 门）通过；`cargo fmt --check`；手动 `npm run tauri dev` 各贴一次真 key 看卡片。
4. 文档同步：每个新/改 provider 更新 `docs/providers.md` 对应节、`README.md` providers 表、相关 `.rs` 顶部 docstring。
5. 本文件是工作文档，执行完可删；不要把它混进 PR。

## 9. 已核实的锚点速查（2026-09-02，main）

| 位置 | 内容 |
|---|---|
| `providers/mod.rs` | `progress` :41 / `text` :54 / `with_reset` :66 / `ok` :89 / `no_credentials` :102 / `error` :115 / `http` :153 / `config_dir` :211 / `credit_meter` :276 / `credit_meter_labeled` :284 / `stored_api_key` :372 |
| `lib.rs` | provider 注册 vec :1004–1030；`set_api_key` 白名单 :1427；`invoke_handler` :1737 |
| `main.ts` | `Metric`/`Snapshot` :67–91；`ALL_PROVIDERS` :264；`renderMetric`（重置渲染）:948；`saveApiKey` :3093；`initSettings` :3176；`[data-save]` 绑定 :3490 |
| `index.html` | Network 组末尾 :219–226；最后一个 key 行（qwen）:278–285；Advanced 组 :286 |
| `i18n.ts` | `settings.keyPhQwen` en :109 / zh :418 / ru :720；`settings.networkNote` en :96 / zh :405 / ru :707；`card.notStarted` 等重置键已存在 |
| `kimi.rs` | docstring :1–9；`fetch` :92（`cred_path` 早退 :93–99）；`fetch_usages` :141；`load_access` :164 |
| `opencode.rs` | `USAGE_URL` :15；`auth_entry_key` :38；auth.json 早退 :141–147；key 解析 :149；`fetch_official` :190 |
| `zai.rs` | `find_key` :7；`fetch` :28；`collect_quota_metrics` :77（TIME_LIMIT 分支 :90–108 已有 `with_reset`，一般分支 :110–140 缺 reset）；`nice_label` :146 |
| `README.md` | providers 表 :152–180 |
| `docs/providers.md` | OpenCode 节 :92–112；Z.ai 节 :163–168；Kimi Code 节 :193–210 |

## 10. 范围依据：为什么"能查的只有这 14 家"

ccSwitch 全部 78 个预设逐一核实过（读源码 + 无凭据探测，401 JSON=接口存在、404/200 HTML=没开）：

- **纯 API key 可查且 Pane 还没有** = 本方案的 6 专属 + 通用 9 站，没有遗漏；
- 查不了的三大类：OAuth 登录类 5 家（Claude/Codex/Copilot/Grok/Gemini，Pane 前 4 家已覆盖）、云厂商控制台类约 14 家（Bedrock/千帆/百炼/火山等，要控制台凭据或无公开接口）、实测 404 无查询接口的中转站 10 余家；
- 仅支持站点访问令牌（非推理 key）的中转站 6 家，不符合"API key 查询"的定义，排除。

结论：名单短不是因为漏了，是因为剩下的**真的查不了**。若日后发现新的可查询厂商，按 §3 的配方随时可加。

## 11. 实施与验证记录（截至 2026-09-03）

实现已覆盖 PR1（任务 1–5、8）和 PR2（通用 billing、多账号实例化、后台刷新修复、图标 registry）；工作区仍未提交。当前完整设计和证据以以下文档为准：

- `docs/plans/pane-provider-quota-design.md`：family / instance / adapter / projection 设计，以及 OAuth 与 API Key 的边界；
- `docs/superpowers/plans/2026-09-03-pane-provider-quota-architecture.md`：逐阶段实施记录，只有原生 GUI 验收 Step 3 仍未勾选；
- `docs/plans/pane-provider-quota-verification.md`：当前验收矩阵、14 家覆盖映射、风险和运行时证据。

最新验证：

- parse-tests harness（GNU 工具链，直接编译真实 provider 源文件）：**66 passed / 0 failed**；
- `npm run build`（tsc + vite）通过；GNU `cargo check`、`cargo check --tests` 和非严格 Clippy 均通过；
- Rust/TypeScript provider catalog：26/26 family、16 个 API Key 能力位和 5 个额外 API Key family 一致；
- 额外 Key 使用稳定的 `family@<fingerprint>` 实例 ID，独立查询、缓存、余额 baseline、布局和托盘投影；Custom Balance 的 Base URL 经过 HTTPS / 本地 HTTP 安全校验；
- GNU Tauri 开发实例已成功启动，`pane.exe` 响应正常，`GET http://127.0.0.1:6736/v1/usage` 返回 200，6 个 provider 快照完成，退出后进程与 1420/6736 端口均释放；
- 完整 `cargo test` 的 app 测试二进制仍因本机 GNU/Windows 启动环境返回 `STATUS_ENTRYPOINT_NOT_FOUND`，不是断言失败；默认 MSVC 仍缺少 `link.exe`；
- relaybalance 尚未使用真实中转站凭据联调，ZenMux 仍是独立可选扩展；
- 原生 Tauri 窗口、托盘点击、隐藏窗口两个刷新周期仍需具备 `sky` Trusted RPC 的桌面环境补验，不能用普通 Chrome 页面替代。
