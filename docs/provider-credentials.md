# Provider 凭据与查询能力

截至 2026-10-16，Pane 目录包含 **69 个 provider 家族：编程 63 个、MCP 5 个、生产力 1 个**。编程组也包含模型 API、中转站、语音 API 和本机工具；分组名称不代表它们都能查询云端剩余额度。

以下严格按三种接入方式列出。同一个 provider 支持多种方式时会跨表出现，不能将行数相加当作 provider 总数。`~` 是用户主目录，Windows 通常为 `%USERPROFILE%`。`%APPDATA%` 和 `%LOCALAPPDATA%` 是 Windows 环境变量；其他系统的路径由平台配置目录决定。

“兼容适配，未验证”表示已存在查询实现，但本轮没有取得完整的第一方公开接口契约或没有对应真实账号实测。它不承诺任意新 key 均可查询。已公开的接口、权限和协议依据见 [API 核验记录](provider-api-verification.md)；下面的能力范围以当前源码为准，不能把文档核验当作所有平台均已联网验收。

## 探测状态与凭据来源（#109）

首次引导和设置里的「来源扫描」只读本机信息，不执行 CLI、不出网：

- `installState`：`installed` / `missing` / `unknown`。`unknown` 表示该 family 没有安装提示，不能当成未安装。
- `credentialState` / `credentialSource`：凭据是否存在，以及来源是 `env:<变量名>`、Pane 保存的密钥/账号列表、本机登录还是 `none`。多账号 family（ClinePass、APIGOTO 等）按账号列表判断，不会因为 key 不在 family 级文件里就误报无凭据。
- 环境变量会重新读取进程环境 + `HKCU\Environment` + `HKLM\...\Environment`；`setx` 后点「重新扫描」即可看到，不需要重启。Windows 下变量名按大小写不敏感匹配，注册表 Path 中的 `%VAR%` 会展开，机器级和 npm 全局安装的 CLI 都算安装证据。
- 探测是只读的：只判断凭据是否存在，不迁移、不删除、不改写密钥文件；单密钥 family 会查询系统凭据库确认存在性，但值不会被返回、记录或发送。
- `ANTHROPIC_API_KEY`、`OPENAI_API_KEY`、`GEMINI_API_KEY`、`GOOGLE_API_KEY`、`AZURE_OPENAI_*` 等通用 key 会在检测结果中显示为“已发现”，但 Pane 目前没有对应的额度读取器；发现 key 不等于有额度数据。

「已安装但未登录」和「未安装」是两个不同状态：卡片和首启清单会分别给出登录/安装动作，不会用同一句“没有找到登录”糊过去。

## 表一：以 API key 接入

这类通常不用导入浏览器 Cookie。key 必须属于对应平台、地区和计费产品；账单权限与模型推理权限可能不同。备注名只是 Pane 中的显示名称，不改变账号归属。

| 分组 | Provider（family ID） | 粘贴什么、是否有额外条件 | 可查询项与主要边界 |
|---|---|---|---|
| 编程 | MiniMax（`minimax`） | Token Plan／旧 Coding Plan key | 套餐会话、周窗口和重置；普通 PAYG key 不保证有套餐数据。也能复用本地配置，见表三 |
| 编程 | Z.ai／智谱（`zai`） | 对应国内／国际站的 Coding Plan key | 套餐窗口及接口返回的 MCP 用量；官方用量插件支持个人计划。普通按量钱包与套餐不同 |
| 编程 | Kimi Code（`kimi`） | Kimi Coding 专用 key | Session、Weekly、接口返回的月总量／月编程量；月总量触顶可冻结短窗口。云端直连接口完整公开契约未核实，不能与 Kimi API key 混用 |
| 编程 | Kimi API／Moonshot（`moonshot`） | Open Platform API key | 钱包余额、Cash、Vouchers；与 Kimi Code 会员分开。余额接口本轮未重新完整核实，未持有新账号实测 |
| 编程 | OpenCode Go（`opencode`） | OpenCode Go 专用 key，有有效订阅 | 套餐会话、周、月窗口；普通其他供应商 key 无法替代 Go key。精确用量接口公开契约未核实；本机消费属于另一种来源 |
| 编程 | OpenRouter（`openrouter`） | 普通 API key；账户总 credits 需 **Management key** | 普通 key 查自身已用、限额和剩余；Management key 查总购入和总消费，不能用于推理。没有设置 key 限额时不能捏造上限 |
| 编程 | DeepSeek（`deepseek`） | DeepSeek API key | 账户余额、赠送与充值余额、可调用状态；零余额是成功查询结果 |
| 编程 | StepFun 按量（`stepfun`） | StepFun API key | Balance、Cash、Vouchers；**不能因此查询 Step Plan**。当前余额兼容接口的完整公开契约未核实 |
| 编程 | SiliconFlow（`siliconflow`） | 对应地区 API key | 验证模型列表接口的凭据连通性。官方已停用旧余额 `/user/info`；当前不能保证账户余额查询，不能把无余额 API 当成 key 错误 |
| 编程 | Novita AI（`novita`） | Novita API key | 可用余额、充值余额及接口返回的账单数据；官方金额单位为 `1/10000 USD`。是 PAYG 钱包，不是 Coding 套餐 |
| 编程 | AihubMix（`aihubmix`） | AihubMix API key | 额度上限、当月消费、使用比例；账单兼容接口未取得完整公开契约，也可复用 OpenCode key |
| 编程 | ElevenLabs（`elevenlabs`） | 有订阅读取权限的 API key | 字符已用／上限、重置、声音槽位及 Overage；字符额度不是模型 tokens |
| 编程 | Deepgram（`deepgram`） | 能列项目并读取账单的 API key | 可访问项目的余额；仅有音频转写权限不保证能查账单，当前适配器最多读三个项目 |
| 编程 | ClinePass（`clinepass`） | Cline API key，账户有有效 Pass | 5 小时、周、月套餐窗口；Cline PAYG 与 Pass 使用同一 key 时由产品／模型区分。精确额度 REST 契约未公开核实，无套餐不能显示全额 Pass |
| 编程 | Codebuff（`codebuff`） | Codebuff API key，或表三的 CLI token | Credits、余额、订阅／周窗口；**兼容适配，未验证所有 SDK key 的账单权限** |
| 编程 | Kilo（`kilo`） | Kilo key，或表三的 CLI 会话 | Credits、Kilo Pass；Personal 与 Organization 的池分开。tRPC 账单接口与普通 Gateway key 的权限关系未公开核实 |
| 编程 | Command Code（`commandcode`） | Command Code API key | 会话／周窗口、月 Credits、Extra Credits；精确账单接口权限未公开核实。能查账单不等于套餐允许 Provider API，Go plan 无该推理权限 |
| 编程 | APIGOTO（`apigoto`） | APIGOTO API key | 订阅、Balance、Points／Credits、有效期及响应实际提供的窗口；不同 key 的计费模式可能不同 |
| 编程 | Droid／Factory（`factory`） | Factory API key | 5 小时、7 日、30 日窗口或旧 allowance、Extra Usage；套餐规则官方已公开，精确账单 REST 权限仍属兼容适配未验证 |
| 编程 | Chutes（`chutes`） | 有账户读取权限的 Chutes key | 响应实际提供的 4 小时／月窗口；限定单个 chute 的 key 不保证有账户管理权限。精确套餐契约未公开核实 |
| 编程 | Venice（`venice`） | Venice API key | USD／DIEM 等余额、tier 和接口返回的限速；固定速率上限不是当前剩余次数 |
| 编程 | Poe（`poe`） | Poe API key | 当前剩余 points，包括套餐和附加 points；不能折算为统一 tokens |
| 编程 | Warp（`warp`） | Warp API key | 请求／Credits、重置、奖励；当前 GraphQL 用量通道完整公开契约未核实，账号与团队池的范围需以实际响应为准 |
| 编程 | Groq（`groq`） | Groq API key | key 连通性；有官方限速 headers 时读取 Requests/day（RPD）、Tokens/minute（TPM）。模型列表未返回 headers 时不能声称已读额度或 key 无效 |
| 编程 | Hugging Face（`huggingface`） | HF access token；账单读取需相应权限 | 当月推理消费、包含额度、应计费消费、请求数；inference 权限不等于 Billing 权限。当前账单通道公开契约未完整核实 |
| 编程 | OpenAI API（`openai-api`） | **Admin API key**，普通项目 key 不够 | 今日与最近 30 天组织消费；**不是预付余额，也不是 Codex 会员额度** |
| 编程 | MiMo（`mimo`） | 对应 PAYG／Token Plan 的 API key；浏览器会话方式见表二 | 尝试套餐月额度或 PAYG 余额；精确读取契约未公开核实，不能承诺普通 key 可查套餐 |
| 编程 | Qwen Code／百炼（`qwen`） | 对应 Coding Plan 的专用 key | 尝试云端套餐额度；控制台可能还要求网页会话。失败可读本机账本，**不能把本机消费当账户剩余额度** |
| 编程 | Amp（`amp`） | Amp API key，或已登录 CLI | Amp Free、个人／工作区 Credits；API 为内部兼容查询，未取得稳定公开额度契约。CLI 方式见表三 |
| MCP | BochaAI／博查（`bocha`） | Pane MCP 密钥管理中添加 Bocha key | 每把 key 的余额；精确余额接口第一方公开契约本轮未取得，属于兼容适配未验证 |
| MCP | Tavily（`tavily`） | Pane MCP 密钥管理中添加 Tavily key | key usage／limit、账户套餐 usage／limit、PAYG usage／limit；多把 key 可能共享同一账户池，不能累加为额外套餐 |
| MCP | Firecrawl（`firecrawl`） | Pane MCP 密钥管理中添加 Firecrawl key | 团队剩余 Credits、套餐 Credits、账期；赠送／充值可能使剩余高于套餐量 |
| MCP | Brave Search（`brave`） | Pane MCP 密钥管理中添加 Brave key | 响应 headers 中的月额度、剩余与重置；**探测会发起真实搜索并消耗成功请求额度**，当前有 12 小时缓存 |

## 表二：专用凭据、浏览器授权或部署信息

这类不能统一称为“贴一个普通 API key 就行”。支持 Pane 登录中心的账号可以直接走引导授权；手工 Cookie／会话会过期。自部署站点必须填写真实 Base URL，不存在通用内置主机。

| 分组 | Provider（family ID） | 需要什么、如何取得 | 可查询项与主要边界 |
|---|---|---|---|
| 编程 | StepFun Step Plan（`stepfun-plan`） | 控制台完整 `Oasis-Token` 会话令牌对；或账号密码供 Pane 登录／续期；或 API Key（受限模式） | 网页登录读会话／周窗口、Credit、Top-up；API Key 只监控本地 Token 并估算 Credits，不显示余额/进度条，精确额度必须网页登录。控制台 RPC 兼容适配未公开验证 |
| 编程 | SenseNova（`sensenova`） | 控制台 `access_token`，最好同时有 `refresh_token`；浏览器 PKCE 流程 | 默认池／Flash-Lite 池的 5 小时／7 天额度与 Grants；普通推理 `sk-` key 不够，控制台兼容接口未公开验证 |
| 编程 | ClawsGO（`clawsgo`） | 网页 localStorage 中的 `clawsgo_token` | 团队 Credits、月额度与 30 天用量；不是普通模型 API key，网页会话兼容适配未公开验证 |
| 编程 | LongCat（`longcat`） | `longcat.chat` 完整 Cookie 或受支持的会话 token | Token 包、使用量、待生效包；不是 LongCat 推理 key，网站会话接口兼容适配未公开验证 |
| 编程 | Mistral（`mistral`） | `admin.mistral.ai` Cookie，包含 `ory_session_*` | 当月 tokens、Vibe、Credits；普通推理 key 无法代替这条后台查询会话，兼容适配未公开验证 |
| 编程 | Perplexity（`perplexity`） | 网站 session token 或完整 Cookie | 周期、购买、奖励 Credits 与使用；不是 Sonar API key，网站兼容接口未公开验证 |
| 编程 | MiMo（`mimo`） | 浏览器 `api-platform_serviceToken` | Token Plan 月额度的网页通道；可作为表一 key 路径之外的方式，兼容适配未公开验证 |
| 编程 | Volcengine Ark（`volcengine`） | `AccessKeyId` + `SecretAccessKey`，表单可按 `AK:SK`；需要目标资源的查询权限 | Coding／Agent Plan 5 小时、周、月及档位窗口；**不是方舟推理 API key**。云端签名查询与普通模型调用分开 |
| 编程 | AWS Bedrock（`bedrock`） | AWS IAM／临时凭据（AK、SK、必要时 Session Token），或已登录 profile／SSO | Cost Explorer 的 Bedrock 当月消费；要有费用读取权限。预算为用户额外设置，**不提供模型剩余额度** |
| 编程 | Custom Relay（`relaybalance`） | **Base URL + API key** | 兼容 billing 接口的消费、限额、余额；站点只支持模型推理时不保证有账单接口，必须按站点实际能力判断 |
| 编程 | sub2api（`sub2api`） | **部署 Base URL + 该站 API key** | key 用量、钱包、订阅日／周／月或速率窗口、requests／tokens；部署版本决定字段。官方源码允许耗尽／过期 key 查询自身用量，不能因推理失败阻止只读额度查询 |
| 编程 | One/New API（`onenewapi`） | **站点 URL + key** 查钱包；订阅需后台 Access Token，部分版本需用户 ID／`New-Api-User` | key 账单、订阅、窗口／到期；各部署、分支与倍率不同，不保证仅 key 即可读全部数据 |
| 编程 | Codex（`codex`） | Pane 浏览器／设备授权登录；也可复用本地 CLI | Session、Weekly、附加窗口、Credits；和 OpenAI Admin key 不是同一账号通道 |
| 编程 | GitHub Copilot（`copilot`） | Pane GitHub OAuth；或本地 GitHub／Copilot 登录 | Premium requests 等上游实际返回的配额与重置；token 需要目标账号权限，内置查询为兼容接口 |
| 编程 | Grok（`grok`） | Pane xAI 登录；或本地 Grok CLI OAuth | 使用窗口、重置、接口提供的 reset credits；不是普通 xAI PAYG API key |
| 编程 | Cursor（`cursor`） | Pane 专用浏览器授权／账号会话，或读取编辑器登录 | Plan usage、Requests、Credits、On-demand；Team Total usage 不能直接当个人套餐上限，后台兼容适配 |
| 编程 | CodeBuddy（`codebuddy`） | Pane 登录中心授权国际版账号 | Credits、Reserved、周期重置；目前不通过通用本地配置导入，不能与国内版会话互换 |
| 编程 | CodeBuddy CN（`codebuddycn`） | Pane 登录中心授权国内版账号 | Credits、Reserved、周期重置；国内外分开，后台查询兼容适配 |
| 编程 | WorkBuddy（`workbuddy`） | Pane 登录中心；显式粘贴 Cookie／启用浏览器导入；或读取可用桌面会话 | Credits、Reserved、资源用量；不会自行扫描所有浏览器。厂商加密会话不能直接解密时改用登录中心 |
| 编程 | Windsurf（`windsurf`） | Pane 登录中心授权 | Daily／Weekly、Overage；需有效账号会话，后台查询兼容适配 |
| 编程 | Qoder 国际版（`qoder`） | Pane 登录中心；或本地国际版登录 | Credits、模型包、到期；国际版与国内版互不替代，兼容适配 |
| 编程 | Trae 国际版（`trae`） | Pane 登录中心；或本地国际版登录 | Credits、Plan；本地过期会话需要在原应用重新登录，兼容适配 |
| 编程 | Kiro（`kiro`） | Pane 授权，取得 token 和 profile ARN；或读取 CLI 登录 | Credits、Overage、费用与重置；不是 AWS Bedrock 的普通 AK／SK 查询通道 |
| 编程 | Zed（`zed`） | Pane 浏览器登录 | 这条路径只读账号／套餐身份；**没有实现云端额度查询**。本机 tokens 另见表三 |
| MCP | GIPHY（`giphy`） | 开发者后台的 dashboard `access_token` | 各 app 的完整日统计、7 天调用数；明确 beta app 才有折算日上限。**应用 API key 不够，历史折算不等于实时小时剩余**；后台兼容接口未公开验证 |

## 表三：自动读取本机文件、凭据存储或服务

“自动读取”指复用本机已有登录／用量，**不代表离线**。读凭据后联网查套餐，与直接统计本机账本，是两种原理。Pane 自己保存的 key 也在本地，但那不是“自动发现其他客户端配置”；四个搜索 MCP provider 当前仅使用 Pane 中添加的 key，不扫描其他 MCP 客户端。

| 分组 | Provider（family ID） | 具体来源 | 能查询什么、原理和边界 |
|---|---|---|---|
| 编程 | Claude／Claude Code（`claude`） | `~/.claude/.credentials.json`；身份来自 `~/.claude.json`；支持 `CLAUDE_CONFIG_DIR` 与候选账号目录 | 读取 CLI OAuth 后联网查 5 小时／周／模型窗口、Extra usage；文件存在不等于登录有效。后台 usage 接口为兼容适配 |
| 编程 | Codex（`codex`） | `~/.codex/auth.json` 或 `CODEX_HOME/auth.json` | 复用 OAuth，联网查 Session／Weekly／附加窗口／Credits；普通 API key 登录不等同订阅登录 |
| 编程 | Cursor（`cursor`） | `%APPDATA%/Cursor/User/globalStorage/state.vscdb`（SQLite 数据库） | 读取编辑器会话后联网查用量；和直接读文本 `.json` 不同，后台接口可能随编辑器变化 |
| 编程 | GitHub Copilot（`copilot`） | `%LOCALAPPDATA%/github-copilot/apps.json`、`hosts.json`；`gh/hosts.yml` 与系统凭据存储 | 选 `github.com` 对应登录后联网查配额；不应误用其他 GitHub 主机的 token |
| 编程 | Grok（`grok`） | `~/.grok/auth.json` | 读 CLI OAuth 后联网查窗口，可刷新有效会话；网络失败不等于登录失效 |
| 编程 | Antigravity（`antigravity`） | 正在运行的 IDE `language_server` 进程／本地接口；系统凭据 `gemini:antigravity` 作为候选回退 | 读取模型额度与重置；部分路径要求 IDE 正在运行。**不是某个固定 JSON 文件，也不是离线额度** |
| 编程 | Devin（`devin`） | `%APPDATA%/devin/credentials.toml`、`%LOCALAPPDATA%/devin/credentials.toml`、`~/.local/share/devin/credentials.toml` | 读取登录后联网查日／周窗口；候选目录取决于平台与安装方式，兼容适配 |
| 编程 | OpenCode（`opencode`） | `~/.local/share/opencode/auth.json` 的 `opencode-go`；同目录 `opencode.db`（SQLite） | key 联网查 Go；数据库统计本机消费，不能代表跨设备总量；查询时处理数据库 WAL，避免把未落盘记录漏算 |
| 编程 | OpenRouter（`openrouter`） | OpenCode `~/.local/share/opencode/auth.json` 的 `openrouter` | 复用对应 key 后联网；权限边界同表一，不会因来自 OpenCode 就获得 Management 权限 |
| 编程 | AihubMix（`aihubmix`） | OpenCode `~/.local/share/opencode/auth.json` 的 `aihubmix` | 复用对应 key 后联网读兼容账单；不是自动读取所有供应商 key |
| 编程 | MiniMax（`minimax`） | `~/.minimax/config.yaml` 的 `provider.minimax.options.apiKey`；`~/.minimax/sqlite.db`、`~/.minimax/v2/sqlite/runtime-state.sqlite` | YAML key 联网查套餐；SQLite 提供本机消费。两类数据不可当作同一剩余额度 |
| 编程 | Z.ai／智谱（`zai`） | `~/.config/zai/key.json`；附加赠额可读 `~/.zcode/v2/credentials.json` | 套餐 key 联网查额度；ZCode 会话只用于其支持的赠额通道，不等于 Coding key |
| 编程 | Kimi Code（`kimi`） | `~/.kimi-code/credentials/kimi-code.json`、`~/.kimi/credentials/kimi-code.json`，支持 `KIMI_CODE_HOME` | OAuth 联网读套餐与身份；失败时可回退已保存 Coding key；月总量和短窗口都应保留 |
| 编程 | Qwen Code（`qwen`） | `~/.qwen/usage/token-usage-YYYY-MM.jsonl`（逐行 JSON 账本） | 读本机请求／tokens，云端额度不可读时回退；**不是全账号钱包或跨设备套餐余额** |
| 编程 | Codebuff（`codebuff`） | `~/.config/manicode/credentials.json` | 复用旧 CLI 兼容目录的登录 token 后联网查 Credits／订阅；本轮未验证所有安装版本均使用此目录 |
| 编程 | Kilo（`kilo`） | `~/.local/share/kilo/auth.json` | 复用 CLI 会话后联网读 Credits／Pass；个人与组织数据范围按会话决定 |
| 编程 | Command Code（`commandcode`） | `~/.commandcode/auth.json` | 复用本地 key 后联网查窗口与 Credits；没有文件时可手工填 key |
| 编程 | Amp（`amp`） | 执行已经安装并登录的 `amp usage` | 由 CLI 查询 Free／个人／工作区 Credits；**不是直接解析某个配置文件**，需命令可用且登录有效 |
| 编程 | Qoder CN（`qodercn`） | `%APPDATA%/com.qodercn.app.stable/auth.v1.dat` + 同目录 `Local State` | 使用本机加密材料解密会话后联网查 Credits／模型包／到期；国内外 token 不通用 |
| 编程 | 讯飞星火（`xfyun`） | Pane 自己的隔离 WebView2 登录窗，cookie 存 `login-accounts/xfyun.json` + OS vault | 只读网页登录态后查 Spark-X2.6 免费额度；不提供 API Key 输入槽（API Key 只能推理） |
| 编程 | Qoder 国际版（`qoder`） | `%APPDATA%/com.qoder.app.stable/auth.v1.dat` + 同目录 `Local State` | 原理同国内版；路径是当前适配器候选，本轮未以所有国际版安装实测，也可用 Pane 登录 |
| 编程 | Trae CN（`traecn`） | `%APPDATA%/Trae CN/User/globalStorage/storage.json` | 解开本地登录数据后联网查 Credits／Plan；不与 IDE 争抢 token 刷新，过期需打开原应用登录 |
| 编程 | Trae 国际版（`trae`） | `%APPDATA%/Trae/User/globalStorage/storage.json` | 读取国际版登录后联网查询；返回主机可随区域不同，不能拿国内版文件代替 |
| 编程 | WorkBuddy（`workbuddy`） | `%LOCALAPPDATA%/CodeBuddyExtension/Data/Public/auth/workbuddy-desktop.info` | 读取可用桌面会话后联网查资源；厂商启用凭据加密时 Pane 不直接解密，使用表二的授权／显式 Cookie |
| 编程 | Kiro（`kiro`） | `%LOCALAPPDATA%/kiro-cli/data.sqlite3`、`%APPDATA%/kiro-cli/data.sqlite3`，支持 `KIRO_DATA_DIR` | 读取数据库中的登录后联网查 Credits／Overage；不每轮启动 CLI，避免更新等副作用 |
| 编程 | AWS Bedrock（`bedrock`） | AWS 环境凭据；否则需设置 `AWS_PROFILE`，由已安装的 AWS CLI 读取 profile／SSO（通常涉及 `~/.aws/config`、`~/.aws/credentials`） | Pane 调用 `aws configure export-credentials` 解析指定 profile，再签名请求 Cost Explorer；不自行解析 AWS 文件，也不从文件计算消费，需费用查询权限 |
| 编程 | Vertex AI（`vertexai`） | `%APPDATA%/gcloud/application_default_credentials.json`，支持 `CLOUDSDK_CONFIG`、`GOOGLE_CLOUD_PROJECT` | 读取 gcloud ADC 并检查账号／项目；**当前只显示身份与项目，没有额度或账单查询** |
| 编程 | Doubao／豆包（`doubao`） | `%APPDATA%/Doubao/User Data/Default/Network/Cookies`（无后缀 SQLite）；`Local State`；Pane 缓存 `%APPDATA%/Pane/doubao_cookies.json` | 提取桌面 Cookie 后联网查订阅窗口、重置卡；首次读取可能需完全退出豆包释放数据库锁，缓存 Cookie 也会过期 |
| 编程 | JetBrains AI（`jetbrains`） | `%APPDATA%/JetBrains/<IDE>/options/AIAssistantQuotaManager2.xml` | **直接读 IDE 缓存**的月 Credits、Top-up、订阅到期；新鲜度由 IDE 更新决定，非独立实时云查询 |
| 编程 | Zed（`zed`） | `%LOCALAPPDATA%/Zed/threads/threads.db` | **本机统计**累计／30 天 tokens、近期模型；只覆盖可识别的 Zed 托管模型。表二登录卡只显示身份 |
| 编程 | Hermes（`hermes`） | `%LOCALAPPDATA%/hermes/state.db` | **本机统计**tokens、会话、模型、消费估算；不提供供应商云端剩余额度 |
| 编程 | MiniMax Code（`mcode`） | `~/.minimax/v2/sqlite/runtime-state.sqlite`、`~/.cc-switch/cc-switch.db` | **本机账本**的 tokens／消费；没有独立云端额度适配器，和表一 MiniMax 套餐卡不同 |
| 编程 | Ollama（`ollama`） | 本地 `http://127.0.0.1:11434` 服务 | 本地模型清单与运行模型；不需 key，也没有云端订阅额度。服务没启动时提示本地连接问题 |
| 生产力 | 闪电说（`shandianshuo`） | Windows 凭据管理器 `auth_token.Shandianshuo Safe Storage` | 读取桌面 token 后联网查语音分钟、助手 Credits、重置；**不是离线读取普通配置文件**，配置存在不等于登录有效 |

用户在 Pane 中手工填写的凭据会由 Pane 自身的受控存储保存；它们不应与自动发现第三方客户端文件混为一谈。只要用户保存并开启账号，即使测试失败也应保留卡片，标明“查询异常”并提供修复入口；“缺少凭据”“凭据被拒绝”“网络／限流故障”“平台暂不提供额度 API”和“额度耗尽”需要分别说明。
