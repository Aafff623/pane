# API key、套餐与额度接口核验

核验日期：2026-10-16。面向需要在 Pane 添加账号、判断失败原因的用户，以及维护查询适配器的开发者。

本页对照当前 provider 目录、查询实现和第一方文档。**文档核验不等于持有账号实测**；没有订阅、没有对应权限、没有真实 key 的平台，本轮没有执行付费推理或真实账号查询。官方没有公开承诺的接口明确标为“未公开核实”，不能因 Pane 已写适配器就承诺每个新账号可用。

额度查询和模型推理是两条独立通道。成功列出模型只证明该请求通过；成功查询额度也不证明某个模型当前可以推理。订阅额度、钱包余额、限速窗口、历史消费及本机 tokens 必须分开理解。

## 官方已公开的查询能力

| Provider | 官方查询接口与凭据 | 能查询什么、没有套餐会怎样 | 第一方依据 |
|---|---|---|---|
| MiniMax | `GET /v1/token_plan/remains`，Token Plan key；官方订阅页示例主机为 `www.minimax.io` | Token Plan 滚动窗口。普通按量 key 不保证有订阅额度；没有套餐不能显示为全新可用套餐。Pane 还兼容旧 `coding_plan/remains` 路径 | [Token Plan 官方页](https://platform.minimax.io/subscribe/coding-plan)、[Anthropic 接入](https://platform.minimax.io/docs/api-reference/text-anthropic-api) |
| Z.ai／智谱 | 官方插件使用 `{api.z.ai或open.bigmodel.cn}/api/monitor/usage/quota/limit`，对应地区的 Coding Plan key | 个人 Coding Plan 配额；官方插件明确仅限 Personal plan。按量付费钱包与 Coding Plan 独立，不应互换推理 URL 或用 OAuth token 代替套餐 key | [官方用量插件](https://docs.z.ai/devpack/extension/usage-query-plugin)、[官方查询脚本](https://github.com/zai-org/zai-coding-plugins/blob/main/plugins/glm-plan-usage/skills/usage-query-skill/scripts/query-usage.mjs) |
| OpenRouter | `GET https://openrouter.ai/api/v1/key`：当前 key；`GET /api/v1/credits`：**Management key** | 前者提供 key 已用、限额和 `limit_remaining`；后者提供账号总购入和总消费。普通 key 的账号 credits 被拒绝时，不能据此判定 key 无效；Management key 不能用于推理 | [当前 key](https://openrouter.ai/docs/api/api-reference/api-keys/get-current-key)、[credits](https://openrouter.ai/docs/api/api-reference/credits/get-credits)、[Management key](https://openrouter.ai/docs/guides/overview/auth/management-api-keys) |
| DeepSeek | `GET https://api.deepseek.com/user/balance`，Bearer API key | 总余额、赠送余额、充值余额、`is_available`。余额为零是有效数据，不能当查询失败 | [Get User Balance](https://api-docs.deepseek.com/api/get-user-balance/) |
| Novita | `GET https://api.novita.ai/openapi/v1/billing/balance/detail`，Bearer API key | `availableBalance`、`cashBalance`、`creditLimit`、`pendingCharges`、`outstandingInvoices`；金额字符串单位均为 **1/10000 USD**。旧 `/v3/user/balance` 的长期可用性本轮未证实 | [官方余额接口](https://docs.novita.ai/api-reference/basic-get-user-balance) |
| ElevenLabs | `GET https://api.elevenlabs.io/v1/user/subscription`，`xi-api-key` | 字符已用、上限、重置、订阅状态和声音槽位。key 要有相应读取权限；不能把文本字符额度套成模型 token 额度 | [Get user subscription](https://elevenlabs.io/docs/api-reference/user/subscription/get) |
| Deepgram | `GET https://api.deepgram.com/v1/projects/{project_id}/balances`，`Authorization: Token <API_KEY>` | 可访问项目的余额列表。先能列项目，再有账单读取权限；并非任意只可转写的 key 都有管理接口权限 | [Project balances](https://developers.deepgram.com/reference/manage/billing/list)、[API 概览](https://developers.deepgram.com/reference/deepgram-api-overview) |
| OpenAI API | `GET https://api.openai.com/v1/organization/costs`，**Admin API key** | 组织历史费用；不是 ChatGPT／Codex 订阅额度，也不是预付钱包余额。普通项目 key 不能代替 Admin key | [官方 Costs 示例](https://developers.openai.com/cookbook/examples/completions_usage_api)、[Admin API](https://developers.openai.com/api/reference/python/resources/admin/subresources/organization) |
| Venice | `GET https://api.venice.ai/api/v1/api_keys/rate_limits`，Bearer API key | USD／DIEM 余额、API tier、访问许可及各模型速率上限；速率上限本身不等于剩余量 | [Rate Limits and Balances](https://docs.venice.ai/api-reference/endpoint/api_keys/rate_limits) |
| Poe | `GET https://api.poe.com/usage/current_balance`，Bearer Poe API key | 当前可用 points，包含套餐和附加 points；不是 tokens。官方另有最多 30 天历史 | [余额](https://creator.poe.com/api-reference/getCurrentBalance)、[Usage API](https://creator.poe.com/docs/resources/usage-api) |
| Groq | 推理响应中的 `x-ratelimit-limit-requests`／`remaining-requests` 为 **RPD**；`limit-tokens`／`remaining-tokens` 为 **TPM** | 官方 headers **没有 `-day` 后缀**。tokens 是每分钟，不是每日。`GET /openai/v1/models` 能验证模型列表请求，不保证附带推理限额；不能因没 headers 判定 key 无效 | [官方 Rate Limits](https://console.groq.com/docs/rate-limits) |
| Tavily（MCP） | `GET https://api.tavily.com/usage`，Bearer Tavily key | 当前 key 的 usage／limit、账户 plan usage／limit、PAYG usage／limit；账户池和单 key 限额分开，多 key 不能把同账户池重复相加 | [Usage](https://docs.tavily.com/documentation/api-reference/endpoint/usage) |
| Firecrawl（MCP） | `GET https://api.firecrawl.dev/v2/team/credit-usage`，Bearer Firecrawl key | 团队 `remainingCredits`、`planCredits`、账期起止。剩余包括其他 credits 时可能高于套餐量；旧 v1 路径本轮未证明已废弃 | [Credit Usage](https://docs.firecrawl.dev/api-reference/endpoint/credit-usage) |
| Brave Search（MCP） | `GET https://api.search.brave.com/res/v1/web/search?q=...`，`X-Subscription-Token` | 官方响应 headers 提供秒窗口和月窗口额度。**成功探测是一笔真实搜索，会计费／消耗额度**；使用缓存降低消耗，并向用户说明 | [认证和请求](https://api-dashboard.search.brave.com/api-reference/web/search/get)、[Rate Limiting](https://api-dashboard.search.brave.com/documentation/guides/rate-limiting) |
| sub2api | 自部署 Base URL + API key，`GET /v1/usage` | 上游源码明确该路由只验证身份，允许过期／耗尽 key 查询自身用量，跳过推理计费执行检查。具体套餐、钱包和限速字段取决于部署版本 | [第一方鉴权源码](https://github.com/Wei-Shaw/sub2api/blob/main/backend/internal/server/middleware/api_key_auth.go) |
| APIGOTO | `GET https://api.apigoto.com/v1/usage`，Bearer APIGOTO key | 第一方集成页公开了查询脚本及 `remaining`／`quota.remaining`／`balance`、`is_active`。套餐窗口仍以实际响应为准 | [官方 CC Switch 集成](https://apigoto.com/apidoc/guide-ccswitch.html) |

## 已接入但必须保留的边界

这里的“未公开核实”只表示本轮没有找到足以确认**精确额度接口和权限**的第一方材料；不是宣称平台绝无该功能。现有兼容读取可保留，但 UI 应如实提示，禁止把任意 key 宣传为已保证能查到套餐。

| Provider | Pane 的查询通道／凭据 | 官方能证实的部分、尚未证实的部分 | 依据／核验入口 |
|---|---|---|---|
| SiliconFlow | 历史 `/v1/user/info` 使用 API key | **官方已宣布 2026-08-14 停用 `/user/info`**；截至本轮查看的 10-16 公告未见替代账户 API。可用 `GET https://api.siliconflow.cn/v1/models` + Bearer 验证 key；必须显示“密钥有效，余额接口不可用”，不能伪造余额，也不应不停要求换 key | [停用公告](https://docs.siliconflow.cn/docs/release-notes/overview)、[Models 官方接口](https://docs.siliconflow.cn/docs/api/models-get) |
| OpenCode Go | `https://opencode.ai/zen/go/v1/usage`，Go key | 官方确认订阅、专用 key、多窗口及不同模型的不同推理协议；**该精确 usage 路径本轮未从第一方公开文档证实**。Go key 与其他供应商 key 独立，不应固定一份套餐额度代替接口返回 | [Go 官方文档](https://dev.opencode.ai/v2/docs/console/go/) |
| Kimi Code | `https://api.kimi.com/coding/v1/usages`，Coding key／本地 OAuth | 官方确认会员共享额度、Coding 与 Open Platform key／Base URL 分离、CLI `/usage`；也公开了本地 server 的 OAuth usage API。当前直连云端 `/usages` 的完整契约本轮未公开核实，不能凭短窗口有余额就忽略月总量冻结 | [接入要求](https://www.kimi.com/code/docs/en/kimi-code-cli/guides/getting-started)、[会员边界](https://www.kimi.com/code/docs/en/kimi-code/membership.html)、[本地 server API](https://www.kimi.com/code/docs/en/kimi-code-cli/reference/server-api.html) |
| Kimi API／Moonshot | `api.moonshot.ai`／`.cn` 的 `/v1/users/me/balance`，Open Platform key | 普通 API 与 Coding 会员计费明确分开。本轮余额文档抓取超时，保留当前适配器但不计入本轮完整官方契约核验 | [第一方余额文档入口](https://platform.kimi.com/docs/api/balance)、[官方 key／Base URL 对照](https://www.kimi.com/code/docs/en/kimi-code-cli/guides/getting-started) |
| ClinePass | `https://api.cline.bot/api/v1/users/me/plan/usage-limits`，Cline API key | 官方源码文档确认订阅、同一 Cline key、`cline-pass/*` 模型和 5 小时／周／月窗口；**精确 usage-limits 路径未公开核实**。普通 Cline PAYG 账号无 Pass 时，不应显示全额 Pass | [Cline 官方文档源码](https://github.com/cline/cline/blob/main/docs/getting-started/clinepass.mdx) |
| StepFun（按量） | `api.stepfun.com`／`.ai` 的 `/v1/accounts`，API key | 第一方账户总览确认余额与赠送余额；精确公开余额 API 契约本轮未找到。不能据此承诺 API key 可读取 Step Plan | [账户总览](https://platform.stepfun.com/account-overview) |
| StepFun Step Plan | 控制台 Dashboard RPC，`Oasis-Token` 会话令牌对，或账号密码续期 | **不是普通推理 key**。精确套餐 RPC 属兼容适配，本轮未找到公开 API 契约；会话、身份冲突、过期要提示补齐／重新登录，网络失败不能归咎密码错误 | [官方平台](https://platform.stepfun.com/)；实际通道见 `src-tauri/src/providers/stepfun_plan.rs` |
| AihubMix | `/v1/dashboard/billing/subscription` + `/usage`，API key | 官方确认 OpenAI／Claude 推理及钱包充值；精确 billing 兼容接口的公开契约本轮未找到 | [第一方接入说明](https://docs.aihubmix.com/en/clients/%E5%9C%A8%20avante.nvim%20%E4%B8%AD%E4%BD%BF%E7%94%A8) |
| Codebuff | `/api/v1/usage` + `/api/user/subscription`，API key／本地 CLI token | 官方支持 SDK API key，CLI 可查 credits；精确额度 REST 契约未公开核实；不能假定任意 SDK key 都有完整订阅读取权限 | [官方 SDK](https://www.codebuff.com/docs/advanced/sdk)、[官方产品说明](https://www.codebuff.com/docs/tips/what-makes-codebuff-unique) |
| Kilo | `app.kilo.ai/api/trpc/user.getCreditBlocks,kiloPass.getState`，key／CLI 会话 | 官方明确 Personal 和 Organization credits 不共享，并公开 Gateway key；该 tRPC 额度通道与 Gateway key 的权限关系未公开核实 | [Credits and Billing](https://kilo.ai/docs/getting-started/faq/credits-and-billing)、[Gateway](https://kilo.ai/docs/gateway/api-reference) |
| Command Code | `/alpha/billing/credits` + `/subscriptions`，key／本地登录 | 官方确认 CLI 与 Provider API 使用同一 key，计划有滚动窗口；精确 billing API 以及 key 是否都有读取权限未公开核实。Go plan 不包含 Provider API | [Max plans](https://commandcode.ai/docs/plans/max)、[Provider API](https://commandcode.ai/docs/provider) |
| Droid／Factory | `/api/billing/limits` + `/api/organization/subscription/usage`，Factory key | 官方确认 5 小时／7 日／30 日窗口与 `/limits`；公开管理 API 接受 Factory key 或 JWT，但不能由此推断 billing 路径权限均相同 | [官方套餐](https://docs.factory.ai/pricing/individuals)、[公开 API 鉴权示例](https://docs.factory.ai/api-reference/machine-templates/get-a-machine-template) |
| Chutes | `/users/me/subscription_usage`，Chutes key | 官方确认 API key 可被限制到指定 chute，不能承诺任何推理 key 都能管理账号。套餐用量路由完整契约本轮未公开核实；普通账户不能凭无窗口响应显示套餐全新 | [官方 Getting Started](https://docs.chutes.ai/docs) |
| Warp | GraphQL `GetRequestLimitInfo`，Warp key | 官方支持个人／团队 API key，额度可能属于不同池；该 GraphQL 查询的公开稳定契约未核实，不能套用固定请求数量为最新套餐上限 | [Usage and billing](https://docs.warp.dev/support-and-community/plans-and-billing/credits)、[API keys](https://docs.warp.dev/agents/cli/oz-cli/api-keys) |
| Hugging Face | `whoami-v2` + `settings/billing/usage-v2`，HF access token | 官方支持 token 和推理账单面板；该 billing REST 完整契约及细粒度 Billing read 范围本轮未公开核实。不能把普通 inference 权限等同账单权限 | [官方 Billing](https://huggingface.co/docs/hub/billing)、[Inference Providers](https://huggingface.co/docs/inference-providers/index) |
| MiMo | Token Plan usage／PAYG 余额，或浏览器 `api-platform_serviceToken` | 官方确认 Token Plan 使用专用 key 和 Base URL，并公开推理；当前 usage／balance 的完整 key 读取契约未公开核实。serviceToken 是浏览器会话，不能标作普通 API key | [官方平台与套餐](https://platform.xiaomimimo.com/)、[官方推理示例](https://platform.xiaomimimo.com/docs/en-US/usage-guide/passing-back-reasoning_content) |
| Qwen Code／百炼 | Coding Plan 控制台查询；失败回退本地 `.jsonl` 用量 | 官方明确 Token Plan、Coding Plan、PAYG 的 key 与 URL 相互隔离；没有证明普通推理 key 可访问控制台用量接口。本地统计只覆盖本机记录，不能称为账户实时剩余额度 | [官方 key 说明](https://help.aliyun.com/en/model-studio/get-api-key)、[套餐隔离](https://help.aliyun.com/zh/model-studio/token-plan-team-quickstart)、[官方用量 CLI](https://help.aliyun.com/en/model-studio/cli/usage-quota) |
| Custom Relay | 用户 Base URL + key，兼容 `/dashboard/billing/*` | 没有统一官方站点；只有部署确实实现兼容账单接口才可用。OpenAI 推理兼容不代表存在历史 billing 接口，不能给未知站点预设剩余额度 | 以站点管理员文档为准；当前兼容逻辑见 `src-tauri/src/providers/relaybalance.rs` |
| One/New API | 站点 URL + API key；订阅需后台 Access Token，部分版本需 `New-Api-User` | 每个部署的版本、权限、倍率与订阅字段不同；API key 和后台 Access Token 是两种权限，不能假定仅一个 key 覆盖全部功能 | [New API 第一方项目](https://github.com/QuantumNous/new-api)、[官方文档](https://docs.newapi.pro/en/docs/guide/wiki/changelog) |
| BochaAI（MCP） | `https://api.bochaai.com/v1/fund/remaining`，Bocha key | 当前适配器存在，本轮没有取得精确余额接口的第一方公开契约；不计入新账号实测。查询失败需保留账号并提示具体状态，不能隐藏卡片 | [官方平台入口](https://open.bochaai.com/) |

## 推理协议与内置 URL

以下是官方材料明确给出的协议入口；**Pane 额度测试应优先调用上面的只读额度接口**。本表不是要求每次额度刷新都发一条推理请求。若用户要独立验证推理，需说明可能计费、选择账号有权访问的模型，并分别反馈“凭据有效”“额度读取成功”“模型推理成功”。

| Provider | 协议／官方入口 | 约束与第一方来源 |
|---|---|---|
| MiniMax | Anthropic SDK Base URL `https://api.minimax.io/anthropic`，Messages 路径为 `/v1/messages` | SDK 会补路径，不能重复 `/v1`；地区与套餐 key 要配套。[官方 SDK 文档](https://platform.minimax.io/docs/api-reference/text-anthropic-api) |
| Z.ai／智谱 | 官方用量插件识别 `https://api.z.ai/api/anthropic` 与 `https://open.bigmodel.cn/api/anthropic` | 当前额度测试不需要构造 Messages 请求。个人套餐与 PAYG 通道应分开。[第一方插件](https://github.com/zai-org/zai-coding-plugins/blob/main/plugins/glm-plan-usage/skills/usage-query-skill/scripts/query-usage.mjs) |
| Kimi Code | Anthropic-compatible Base URL `https://api.kimi.com/coding/` | 由 SDK 补 `/v1/messages`，使用 Coding key。[Getting started](https://www.kimi.com/code/docs/en/kimi-code-cli/guides/getting-started) |
| Kimi API | Open Platform Base URL `https://api.moonshot.cn/v1` | PAYG key，不能与 Coding key 混用。[官方平台对照](https://www.kimi.com/code/docs/en/kimi-code-cli/guides/getting-started) |
| OpenCode Go | 官方现行 Base `https://opencode.ai/inference/go/v1`；按模型分别走 `/chat/completions`、`/responses`、`/messages` 或 Gemini `/models/{id}` 通道 | **不能对所有 Go 模型统一使用 Chat Completions**；模型、入口和 SDK 配对以官方 Endpoints 表为准，Go 使用范围也受官方客户要求约束。[Go Endpoints](https://dev.opencode.ai/v2/docs/console/go/) |
| OpenRouter | `https://openrouter.ai/api/v1/chat/completions`；官方 API 目录另列 Responses、Anthropic Messages | 普通推理 key；Management key 不支持推理。只内置实际采用且核对过的协议。[Chat API](https://openrouter.ai/docs/api/api-reference/chat/send-chat-completion-request)、[Management key 限制](https://openrouter.ai/docs/guides/overview/auth/management-api-keys) |
| ClinePass | OpenAI Chat Completions `https://api.cline.bot/api/v1/chat/completions` | 模型必须使用 `cline-pass/*` slug，且账号有有效 Pass；同一 key 的非 Pass 模型可能走 PAYG。[官方源码文档](https://github.com/cline/cline/blob/main/docs/getting-started/clinepass.mdx) |
| Novita | OpenAI-compatible Chat Completions `https://api.novita.ai/openai/v1/chat/completions` | 直接 HTTP 使用完整路径；OpenAI SDK Base URL 设为 `https://api.novita.ai/openai/v1`，不能漏掉 `/openai`。[官方 Chat API](https://docs.novita.ai/api-reference/model-apis-llm-create-chat-completion) |
| SiliconFlow | `https://api.siliconflow.cn/v1/chat/completions`；另支持 `/v1/messages` | key 与地区配套；模型参数支持范围依文档，不能将公开 Models 请求成功等同推理有余额。[官方公告和接口导航](https://docs.siliconflow.cn/docs/release-notes/overview) |
| AihubMix | `https://aihubmix.com/v1/chat/completions`；Claude 原生 `/v1/messages` | 模型支持和权限依站点；替换 Base URL 不代表所有参数完全等价。[官方接入](https://docs.aihubmix.com/en/clients/%E5%9C%A8%20avante.nvim%20%E4%B8%AD%E4%BD%BF%E7%94%A8) |
| Kilo | `https://ai-gateway.kilo.ai/api/v1/chat/completions` | 账户／组织 key 的策略可能限制模型；402 余额与403策略不同。[官方 Gateway API](https://kilo.ai/docs/gateway/api-reference) |
| Command Code | `https://api.commandcode.ai/provider/v1/messages`；`.../chat/completions` | Premium models 用 Messages，open models 用 Chat Completions；Go plan 无 Provider API。[官方计划及协议](https://commandcode.ai/docs/plans/max) |
| APIGOTO | `https://www.apigoto.com/v1/responses`、`/v1/chat/completions`、`/v1/messages` | 官方 Messages 示例使用 **Authorization Bearer**，不能照搬 Anthropic 的 `x-api-key`。[Responses](https://www.apigoto.com/apidoc/api-responses.html)、[Chat](https://www.apigoto.com/apidoc/api-chat-completions.html)、[Messages](https://www.apigoto.com/apidoc/api-messages.html) |
| Poe | `https://api.poe.com/v1/chat/completions`；Anthropic SDK Base URL `https://api.poe.com` | Anthropic 通道仅官方 Claude 模型；其他模型用 OpenAI-compatible／Poe SDK。[官方 SDK 对照](https://creator.poe.com/api-reference/client-sdks) |
| Hugging Face | OpenAI SDK Base URL `https://router.huggingface.co/v1` | 需 inference 权限；OpenAI-compatible 客户端不用于图片生成。[官方 Inference Providers](https://huggingface.co/docs/inference-providers/index) |
| MiMo | OpenAI SDK Base URL `https://api.xiaomimimo.com/v1` | 这是官方 PAYG 示例，Token Plan 使用专用入口，不能将两个 URL 互换。[官方推理文档](https://platform.xiaomimimo.com/docs/en-US/usage-guide/passing-back-reasoning_content)、[Token Plan 平台说明](https://platform.xiaomimimo.com/) |
| 百炼 Coding Plan | `https://coding.dashscope.aliyuncs.com/v1`，Coding Plan 专用 key | Token Plan、Coding Plan、普通按量三种通道隔离，国际区应按实际地区文档选择。[官方隔离说明](https://help.aliyun.com/zh/model-studio/token-plan-team-quickstart) |

其余 provider 若本轮没有取得公开推理入口的第一方依据，本页不猜测 endpoint。语音、搜索、爬虫以及 CLI Agent 不应强制套进 OpenAI Chat／Responses／Anthropic／Gemini 任一协议。

## 添加与修复时如何判断

- **401／明确 invalid key**：凭据被拒绝；引导替换该平台所需类型的 key／会话。
- **403**：权限、套餐、组织策略或会话范围问题；保留上游错误语义，不能一律改写为“密码错误”。
- **429、5xx、DNS、连接超时**：限流、服务或网络故障；提示重试，不能清空凭据或伪报额度耗尽。
- **key 有效但没有套餐／没有公开额度 API**：保留启用的账号，明确查询能力边界；模型连通和额度可读分开显示。
- **额度为零**：查询本身成功，显示额度耗尽；**查询失败**：没有新额度数据，显示查询异常，旧快照必须标明过期。

保存失败账号是用户明确需要的能力。测试失败不能让表单变成无法保存；用户开启后总览和详情应出现该账号并提供修复入口。账户数量按实际启用项计数，可用数量只能来自成功且未耗尽的数据；同账号多 key 共享池不能被误计为额外账户额度。
