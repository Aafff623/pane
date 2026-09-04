# Pane Provider Quota Verification

## 当前验收范围

本轮交付验证的是“provider family → provider instance → query adapter → Snapshot → 页面/托盘/HTTP API”这条边界。没有把 ccSwitch 的 SQLite、UniversalProvider、脚本沙箱或 OAuth 多账号中心带进来。

### API Key 查询覆盖与多账号覆盖的区别

- API Key 查询的定稿范围是 5 个必做专属能力（Kimi For Coding、StepFun、SiliconFlow、OpenCode Go、Novita）加 1 个可选 ZenMux 专属能力，以及 1 个通用 Custom Balance 能力；后者覆盖 9 家 OpenAI 兼容 billing 中转站，因此必做主覆盖为 14 家厂商，ZenMux 纳入后再额外扩展。
- OpenCode Go 不单独创建 `opencodego`；现有 `src-tauri/src/providers/opencode.rs` 已负责官方 `https://opencode.ai/zen/go/v1/usage` 查询和本地 `opencode.db` 回落，Settings Key 只是其已有凭据来源之一。
- 多账号实例化是另一条能力边界，目前开放给 DeepSeek、Kimi Code、StepFun、SiliconFlow、Novita、Custom Balance 六个 family；Kimi 的 CLI OAuth 仍是主卡来源，不混入 `accounts/kimi.json`，OpenCode Go、其他 OAuth 和本地 CLI 账号也不混入 `accounts/<family>.json`。

### 14 家主覆盖的实现映射

| 覆盖对象 | pane family | 查询实现 | 备注 |
| --- | --- | --- | --- |
| Kimi For Coding | `kimi` | Pane 中有 Coding API Key 时优先，否则使用 CLI OAuth | 保留双来源；状态只显示当前生效来源 |
| StepFun | `stepfun` | 官方余额接口 | 支持主 Key 和额外 Key |
| SiliconFlow | `siliconflow` | 官方余额接口 | 支持主 Key 和额外 Key |
| OpenCode Go | `opencode` | 官方 usage API，失败时回落 `opencode.db` | 不新增 `opencodego` family |
| Novita AI | `novita` | 官方余额接口 | 支持主 Key 和额外 Key |
| DMXAPI、PackyCode、Micu、CrazyRouter、SudoCode.chat、XycAi、E-FlowCode、CherryIN、AICodeWith | `relaybalance` | 通用 OpenAI 兼容 billing 接口 | 通过各自 Base URL + API Key 区分实例 |

因此，当前必做主覆盖是 5 个专属能力加 9 个通用 billing 中转站，共 14 家；ZenMux 仍作为独立可选扩展，不影响这条主链路的完成判定。

### ZenMux 的动态调整

已核对 ccSwitch 源码：ZenMux 的实现位于 coding-plan 服务，通过用户提供的查询端点和 Bearer API Key 工作。Pane 当前没有 `zenmux` provider family，而是以原生 provider 模块直接产出 `Snapshot`。本轮不把这条可选路径强行改造成新的原生 provider；否则还需要同步扩展 provider catalog、设置 UI、图标、错误测试和真实凭据验收。当前记录的是可复用的查询契约和扩展边界，不把 ZenMux 的可选实现计入主交付完成度。

## 已完成的静态与单元验证

| 项目 | 结果 |
| --- | --- |
| `npm run build` | 通过；TypeScript 与 Vite 构建成功 |
| `src-tauri/target/parse-tests` | 68 passed, 0 failed |
| `cargo +stable-x86_64-pc-windows-gnu check` | 通过 |
| `cargo +stable-x86_64-pc-windows-gnu check --tests` | 通过 |
| `cargo +stable-x86_64-pc-windows-gnu clippy --tests --message-format=short` | 命令完成；剩余告警来自既有死代码、不可达代码和未修改模块，六个本轮余额 provider 的无意义借用已清理 |
| Rust/TypeScript provider catalog 一致性 | 26/26 family、API Key 能力位和 6 个额外 API Key family 一致 |
| Git 冲突标记扫描 | 未发现 `<<<<<<<`、`=======`、`>>>>>>>` |
| `git diff --check` | 通过；仅有 Windows 换行提示 |

### 对外文档与设置入口

- `README.md` 的 Providers 表已列出 StepFun、SiliconFlow、Novita 和 Custom Balance，并说明各自凭据及查询结果。
- `docs/providers.md` 已记录 Kimi Code API Key 回落、OpenCode Go Settings Key、StepFun 双域名、SiliconFlow 双域名、Novita 单端点和 Custom Balance 两种 billing 路径。
- `docs/privacy.md` 的完整网络请求表已补充 StepFun、SiliconFlow、Novita，并单列用户配置的 Custom Balance relay origin，明确 API Key 只发送到对应查询端点。
- `index.html` 的 API Keys 设置区已接入 Kimi Code、OpenCode Go、StepFun、SiliconFlow、Novita；Custom Balance 单独提供 Base URL + API Key。
- `src/i18n.ts` 已补齐新增设置项和 credential hint 的 English、简体中文、Русский 文案；`npm run build` 已通过模板和类型检查。

完整 Tauri 测试二进制曾在默认 GNU 环境下返回 `STATUS_ENTRYPOINT_NOT_FOUND`；本轮补齐 GNU target，并强制 Tauri 使用 GNU toolchain 和 `D:\Tools\mingw64\bin` 后，Tauri 已成功编译并启动 `pane.exe`。本地 API `GET http://127.0.0.1:6736/v1/usage` 返回 200，响应包含当前 provider quota 数据；Vite 页面 DOM 也确认 Settings、API Keys、One/New API 区块可加载。真实 `src-tauri` lib test 即使使用相同 GNU target 仍在启动阶段返回 `STATUS_ENTRYPOINT_NOT_FOUND`，不是断言失败；因此 provider 行为断言继续使用 parse-tests harness。默认 MSVC 启动仍因缺少 `link.exe` 失败。

## 隔离运行实验

稳定 ID 改动后的最近一次真实启动中，`pane.exe` 窗口进程响应正常；本地 API 返回 200，并返回 6 个 provider 投影。

本次复核再次使用 GNU Tauri 开发实例启动成功，运行日志确认
`codex`、`copilot`、`zai`、`antigravity`、`kimi`、`stepfun` 均产出快照；
随后通过 Ctrl+C 正常停止，确认 `pane.exe` 已退出且 1420/6736 端口均已释放。
启动期间重新读取桌面控制状态仍为 `apps=[]`，因此原生窗口和托盘点击验收的
证据边界没有改变。

在 catalog 增加 `supportsApiKey` 能力位后再次启动，`pane.exe` 仍保持响应，
`GET http://127.0.0.1:6736/v1/usage` 返回 200，HTTP 投影包含 6 个 provider，
日志再次确认上述 provider 均完成快照；退出后进程和 1420/6736 端口均已清理。

曾尝试用 `src-tauri/target` 下的临时 fixture、假 API Key 和本地 mock relay 运行完整 `run_usage_fetch`。Windows 的 `dirs::config_dir()` 使用系统 Known Folder，不接受本次进程的 `APPDATA` 覆盖；直接启动二进制只得到初始空 API，使用真实 `tauri dev` 则读到用户现有配置而不是 fixture。为保护用户配置，没有继续增加生产代码的可注入配置目录开关，也没有把这次实验当成通过；本轮可执行的独立实例证据由 relaybalance mock 单元测试提供。

## 逻辑矩阵

| 场景 | 后端预期 | 页面/托盘预期 | 当前证据 |
| --- | --- | --- | --- |
| 主 Key 成功 | 使用 family 主卡，写入 family 级 last-good | 显示正常额度 | provider 解析测试、Tauri 启动/API 200 |
| 额外 Key 成功 | 使用稳定 `family@<fingerprint>`，独立 baseline/cache | 生成独立账号卡，不覆盖主卡 | parse-tests、relay mock 双卡测试；Kimi 命名身份测试 |
| 同 Key 重复添加 | `account_add` 拒绝重复实例 | 不产生重复卡片 | Rust 端候选 ID 去重逻辑 |
| 删除或重排账号 | 只影响对应 fingerprint；不猜测迁移旧 `family@n` | 清理失效布局和禁用状态 | accounts 单测、前端静态链路审计 |
| Kimi 有 API Key 与 OAuth 文件 | 使用 Kimi Coding API Key，不读取 OAuth 作为当前来源 | 状态只显示 API key；主卡可显示账号总数 | `preferred_source` 单测、刷新逻辑审计 |
| Kimi 无 API Key 但 OAuth 有效 | 使用 Kimi Code CLI OAuth | Moonshot 钱包按既有规则折叠，并显示 OAuth 凭据 | `preferred_source` 单测、刷新逻辑审计 |
| Kimi 额外 API Key | 只查 Coding plan，不折叠全局 Moonshot wallet | 生成独立 `kimi@<fingerprint>` 卡 | 命名快照身份测试 |
| OpenCode Go | 走 `opencode.rs` 官方 usage API；必要时回落本地数据库 | 保持 OpenCode 单卡 | `USAGE_URL` 源码核验 |
| One/New API 多 Key | 独立 `onenewapi@<key-id>` 和 generation 保护 | 不被六个 API Key family 清理逻辑误删 | generation/清理测试 |
| 429/5xx/超时 | 复用 last-good 并标记 stale/warning | 不闪成“新鲜成功” | `guarded`、缓存逻辑源码和测试 |
| 401/403/空响应 | 保留确定性 error，不伪装旧额度 | 明确显示失败状态 | provider 错误解析测试 |
| Disabled | 不启动对应查询 future；动态卡片按 family 规则处理 | 卡片与托盘不展示被禁用项 | `card_is_disabled` 测试、调度审计 |
| 窗口隐藏 | Rust 后台循环继续刷新并发出 `usage-updated` | 恢复窗口后采用最新投影 | Tauri 后端启动/API 证据；原生周期待验收 |
| Custom Balance URL | 公网只允许 HTTPS；HTTP 仅允许私有、回环或链路本地 IP | 不会因“能输入 http”而把 Key 发往公网明文地址 | `relaybalance` URL 校验测试 |

## 额外 API Key 的前后端链路审计

源码级审计确认六个多账号 family 的完整链路已经接通，入口不是孤立的后端命令：

1. `src/providerCatalog.ts` 的 `supportsApiKey` 和 `supportsExtraAccounts` 是前端能力判断；`renderAccountsBlock` 只在 family 主卡上渲染，`deepseek@<fingerprint>` 等实例卡不会重复出现管理入口。
2. `Customize → ⚙` 打开 family 配置时调用 `account_list`，列表只接收稳定非敏感的 `id`、`label`、`hasKey`、脱敏后的 `maskedKey` 和 Custom Balance 的 `baseUrl`。
3. Add-account 的 Test 复用 `test_api_key`，只有探针成功才解锁 Add；编辑 Key、标签或 relay base URL 会重新锁定 Add。
4. Add/Remove 分别调用 `account_add`、`account_remove`。Rust 端再次校验 family、非空 Key、URL scheme 和 relay base URL；持久化使用 `accounts/<provider>.json`，没有把完整 Key返回前端。
5. 下一轮 `fetch_usage` 从 catalog 枚举六个 family 的账号文件，为每条记录生成稳定的 `family@<fingerprint>` 与独立展示名，并把查询结果交给对应的命名 adapter；成功、错误、缓存和余额 baseline 都以该实例 ID 为边界。Kimi 额外卡片只查询 Coding plan，不合并全局 Moonshot wallet。
6. 刷新完成后现有 `usage-updated` → `lastSnapshots` → 页面/托盘投影链路消费这些实例卡；Add/Remove 后都会触发一次强制刷新，因此新卡会出现、删除卡会消失。
7. Test 请求使用前端 generation ledger；修改输入、重复发起探针或请求乱序返回时，旧结果不会解锁当前内容的 Add/Save。非成功结果始终保持按钮锁定。
8. 账号列表回写布局时，如果新增账号表单仍处于打开状态，不重绘整个 drawer，避免清掉用户输入或让异步 Test 结果更新到脱离 DOM 的旧节点；表单关闭后的下一轮刷新仍会收敛布局。
9. 对升级前遗留的 `family@1` 位置账号 ID 不做猜测迁移；布局、provider 配置和 disabled 列表会在前端收敛时删除这些旧投影，避免把旧状态挂到错误的 Key。
10. 合并回归复核清理了 `index.html` 中重复的 `key-kimi` 输入节点，以及 Tauri handler 中重复的 `fetch_spend` / `set_api_key` 注册；当前 HTML `id` 与 invoke command 均无重复项。
11. Custom Balance 的保存、Test 和额外账号查询都经过同一个 Base URL 校验；公网 HTTP、userinfo、query 和 fragment 均会被拒绝。
12. Rust `set_api_key` 的 provider 白名单和前端 API Key 设置入口都从 catalog 的 `supportsApiKey` / `supports_api_key` 派生，新增或移除 Key provider 时不会只改一侧。

这项是静态源码链路证据，不等同于原生 GUI 点击验收。当前环境缺少 Codex 的 `sky` Trusted RPC，无法自动点击真实 Tauri 窗口；普通 Chrome 只验证了页面静态 DOM，不能替代 Tauri 的 `invoke/listen`、托盘和隐藏窗口行为。

### Provider 与身份

- 主 Key：DeepSeek、StepFun、SiliconFlow、Novita、Custom Balance。
- 额外 Key：六个 family 使用稳定的 `family@<fingerprint>` 实例 ID；查询入口接收实例 ID和展示名。
- `credit_meter`：额外 Key 使用实例 ID作为 baseline key；不会再把同一 family 的账号合并到 family baseline。
- 独立查询：本地 mock relay 同时验证两个 Key 返回不同 limit/usage，最终分别得到两个稳定实例的 `$9.00` 和 `$10.00`，ID、名称和余额均未串号。
- 展示元数据：账户卡 family 名称直接读取 Rust provider catalog，未知 family 保留原始 ID 作为 fallback。
- One/New API：继续使用自己的动态 `onenewapi@<key-id>` 卡片生成和 generation 保护。
- Kimi：Pane API Key 优先、无 Key 时使用 CLI OAuth；额外 Kimi 卡片独立查询 Coding plan。

### 状态与投影

- 成功：写入实例级 last-good snapshot，并由同一轮结果驱动页面、托盘和 HTTP API。
- 瞬时失败：复用已有快照并按 grace/cache 窗口标记 stale/warning。
- 确定性失败：保留 error，不伪装成新鲜成功额度。
- 禁用：不启动对应 provider future；One/New API family 禁用仍覆盖其动态 key 卡片。
- 遥测：只按 `family_of(id)` 聚合，不发送账号位置、Key 或 token。
- 隐藏窗口：后端自动刷新继续运行，前端通过 `usage-updated` 接收结果；空结果也会清掉旧页面状态。

## 仍需原生窗口控制能力完成的验收

当前进程启动和后端 API 已验证；本环境的 CUA 能控制浏览器，但原生 `@oai/sky` 的 `list_apps()` / `initialize()` 返回 `Trusted RPC service is not configured: sky`，没有原生窗口/托盘控制，以下交互验收仍需在具备原生窗口控制能力的桌面环境中完成：

1. 配置一个主 Key 和同 family 的两个额外 Key，连续观察三张卡片的 `fetchedAt`、余额和错误状态。
2. 隐藏窗口至少两个刷新周期，再打开窗口，确认页面和托盘均采用最新一轮结果。
3. 修改 `refreshMinutes`，确认后台下一周期重新读取配置；手动 Refresh 与后台刷新不产生并发重复请求。
4. 分别制造 401/403、429/5xx、超时和空响应，确认确定性失败与瞬时失败的展示不同。
5. 检查暗色托盘下 Copilot 重染色、MiniMax/Novita 反色，以及缺少 SVG 时的 initials fallback。

## 当前未覆盖风险

- 稳定 ID 当前由 Key 派生；未来如果支持“更换 Key 但保留账号身份”，需要增加显式 AccountId 字段和迁移策略。本轮没有改变账号文件格式。
- 原生按钮、托盘、多账号卡片、隐藏窗口刷新周期和错误恢复尚未完成 GUI 级自动化；普通 Chrome 页面缺少 Tauri runtime，不能作为这些项目的替代证据。原生控制阻塞的具体原因是 Codex 环境未配置 `sky` Trusted RPC service。
- `src-tauri/Cargo.toml` 的 `crate-type = ["rlib"]` 仍是本地调试值，不能带入上游 PR。

## 后续 PR 边界

1. 托盘后台刷新修复；不包含本地 `Cargo.toml` 调试值。
2. 六个 API Key provider 的实例级查询和余额 baseline 隔离。
3. Provider capability catalog，以及 Kimi/One-New API 路由保持。
4. 图标 registry 和界面整理。
5. OAuth 多账号管理单独立项。

当前工作区只完成代码和验收记录，不执行 commit、push 或正式提 PR。
