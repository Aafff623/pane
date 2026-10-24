# 讯飞开放平台（语音资源包）Provider：console 网页登录态余量

**Status:** implemented (code + fixtures) — 待 owner 实登与控制台数字对照（GitHub #47）
**GitHub:** [#47](https://github.com/Aafff623/pane/issues/47)
**前置调研:** `temp/xfyun-console-quota-analysis-20261023.md`（2026-10-23 真实登录态实测，含响应样本与待确认清单）
**相关:** #38 / `.scratch/spark-xfyun-provider/`（星辰 MaaS 星火 provider，另一个域名，不重复）

## Problem Statement

owner 在**讯飞开放平台**（`www.xfyun.cn` / `console.xfyun.cn`）领到了五个语音资源包：在线语音合成、语音听写、语音评测（各 1 万次）和语音转写、实时语音转写（各 5 小时）。Pane 现在看不到这些余量，每次都要开网页控制台。

#38 接的是**星辰 MaaS**（`maas.xfyun.cn`，Spark-X2.6 的 token 池）。两者是同一家公司的两套体系：不同登录域、不同凭据（MaaS 是 `ak-…` API Key，开放平台是 APPID+APIKey+APISecret）、不同余量接口。实测确认 MaaS 网关上没有任何余量路由（`/v1|v2/usage|quota|balance|account` 全部 404），所以开放平台的余量**不能**并入 #38，必须新接。

核心矛盾和 #38 同类但更硬：**开放平台没有对外官方余量查询 API**，只能重放控制台自己的 `/dashboard/…` 接口；而这些接口 **HTTP 状态码恒为 200**，登录失效只体现在 body 的 `code=80000`。

## Solution

新增第三个 web-session provider，family 建议 `xfyun-speech`（显示名「讯飞开放平台语音」）：

1. 复用 #38 已落地的通用网页登录机制（`is_web_session` / `WEB_SESSION_FAMILIES` / 按 family 分发的登录命令面 / `login_accounts` + `secretstore`），只把登录域换成 `console.xfyun.cn`，用轻量认证接口确认登录态。
2. 一个账号下枚举应用（`app/list`），对每个 app 拉三类余量：次数类走 `meter/api-use-count`，录音转写走 `lfasr/getlfasrInfo`（数组，可能多张包），实时转写走 `get-rtasr-app-auth`（单位是秒）。
3. 卡片按服务分行展示「剩余 / 总量」和到期；把 `code=80000` 归到 `no_credentials`（重新登录），把 `flag=true, data=null` 归到「该服务无此计量」，两者都**不等于 0 额度**。

不新抽机制、不改前端分支结构：这一步只加一个 family 和一个取数模块。

## User Stories

1. 作为讯飞开放平台用户，我要在 Pane 一张卡上看到五个语音资源包各自的剩余量、单位和到期时间，不用逐个开控制台页面。
2. 作为用户，我要通过 Pane 自己打开的登录窗口完成登录，不手动粘贴 cookie 或 APIKey。
3. 作为用户，我要登录一次后重启 Pane 仍有效（观察该会话到期约一年）。
4. 作为用户，会话过期时我要看到「网页登录已失效，请重新登录」，而不是一张显示 0 的卡片。
5. 作为多应用用户，我要卡片把我账号下的每个应用的额度分别或汇总呈现，而不是只读默认应用。
6. 作为多账号用户，我要每个讯飞账号分别登录、分别显示、可单独重登或删除。
7. 作为隐私敏感用户，我要 cookie 和 uid 不进日志、不进 Git、不进公开状态接口；`certification/get` 返回的姓名/证件号绝不入库。
8. 作为想知道消耗的用户，我要看到试用池（threshold/remainCount）与套餐包（left）是两条不同的量，不要被我理解为同一个池子。
9. 作为维护者，我要新增这个 provider 时不复制第二套 `family === "…"` 特判。
10. 作为 owner，我要卡片数字和网页控制台一致之后才算验收，任何未经核实的字段含义标 `待确认`。

## Implementation Decisions

### Family 与目录

- family 建议 `xfyun-speech`；`display_name`「讯飞开放平台语音」；`query_kind: NativeSnapshot`（一个 adapter 产出多行指标，不是单一 balance 端点）；`supports_api_key: false`；`supports_extra_accounts: true`；`primary_unit: Requests`；`category` 由实现者按现有分组判断（Coding 还是 Mcp/Productivity）。
- 必须同时改的接线面（`#38` 已列过一轮）：`provider_catalog.rs`（definition + `WEB_SESSION_FAMILIES`）、`providers/mod.rs`（`pub mod`）、`provider_dispatch.rs`、`lib.rs` 的 web-session 命令面与 snapshot 路由、`auth_center.rs`、`login_accounts.rs`、前端 `providerCatalog.ts`（含 `webSession`）、`providerMechanisms.ts`、`providerOnboarding.ts`、`providerVisuals.ts`、`src/assets/providers/<family>.svg`、`src/i18n.ts`（en/zh/ru 三字典，i18n 配对有门禁）。
- 新 family 会落进 `default_disabled_families()`，需要 owner 手动启用——保持默认关闭，不要为它开例外。
- `WEB_SESSION_FAMILIES` 仍是硬编码常量列表。是否把它改成 catalog 上的 `web_session: bool` 字段由实现者权衡（#38 spec 原本期望能力驱动）；本需求不依赖这项改造。

### 会话与账号识别

- 登录窗口：incognito WebView2 打开 `https://console.xfyun.cn/`，由 owner 手动登录；Pane 不碰验证码、不存账号密码。会话 cookie 只从 Pane 自己的 WebView 取（`platform::read_webview_cookies`），不读浏览器 profile 的 SQLite。
- **登录判定不能只看有没有 cookie**：用 `/dashboard/user/profile/get` 或 `/dashboard/user/balance`，要求 `flag == true && code == 0`。
- 稳定账号 id：优先 `account_id` cookie（纯数字，等于消息接口里的 uid），次选 `/dashboard/user/certification/get` 的 `data.uid`。**只用 uid 这一个字段**，同响应里的 `idName / idNo / mobile` 是 PII，不落库、不打印。
- 会话存 `secretstore`（OS vault，失败按现有明文回退约定），scope 建议 `web/xfyun-speech`；多账号走 `login_accounts`，按 uid 去重。
- 只需要保存 `ssoSessionId` 一个 cookie 即可工作（实测最小集），但整包 jar 存下来更省事——两种都可，别把无关的分析类 cookie 当凭据。

### 请求

- `GET https://console.xfyun.cn/dashboard/…`，**只带 `Cookie` 就能过认证**（实测 `User-Agent`、`Referer`、`Origin` 均不影响结果，可不发）。
- 沿用 #38 的 `call_console` 形态：8s 超时、`MAX_BODY_BYTES` 上限、非 2xx → `Other`。
- **新增一个判定**：`flag == false && code == 80000` → `CallErr::Auth`；`code == 20003` → 参数问题（记为 `Other`，说明是我们拼错了）；`flag == true && data == null` → 正常但无此计量，不是错误。

### 取数与单位

- 应用枚举：`GET /dashboard/app/list?currPage=1&keyword=&type=`（参数名必须是 `currPage/keyword/type`，带 `pageSize` 会 `20003`），分页按 `totalCount` 翻；`app/default` 只给默认应用，不能用它代表全部。
- 次数类：`GET /dashboard/meter/api-use-count/get?businessId=<ch>&appId=<appid>` → `left`（套餐余量）、`threshold/remainCount`（试用池）、`usedCount`、`expireDate`（字符串 `YYYY-MM-DD HH:MM`，时区待确认）。渠道 id 只有 `tts/iat/ise` 实测有效；`lfasr/rta` 返回 `data:null`。**不要猜渠道**，按已实测清单来，其余标待确认。
- 录音转写：`GET /dashboard/lfasr/getlfasrInfo?appId=` → **数组**，每项 `count/leftCount`（小时）、`effectiveDate/dueDate`（字符串）、`status/convertStatus`（含义待确认）。多张包需按到期排序后再决定是否合并展示。
- 实时转写：`GET /dashboard/lfasr/get-rtasr-app-auth?appId=` → `leftCount`（**秒**）、`usedCount`、`maxConnection`（路数）、`expireTime`（**unix 秒**）。
- 单位映射到现有 `UsageUnit`：次数 → `Requests`；lfasr 小时 → `Minutes`（×60）；rta 秒 → `Minutes`（÷60）。不新增枚举值，除非实现者认为混淆成本更高。
- 三种到期编码（无时区字符串 / 带秒字符串 / unix 秒）必须分别解析；解析不出来就显示「未知」，不伪造日期（同 #38 规则）。

### 卡片与失败态

- 主信息是「剩余」，不是百分比：五个服务单位不同，不能汇成一个进度条。允许每个服务一行 `Metric::with_usage`（`Requests` / `Minutes`），外加一行「到期」文本；路数是独立事实，不要并进用量行。
- 会话过期 → `no_credentials` 语义 + 重新登录提示；**不能**渲染成 0%。全渠道 `data:null` → 正常空态而非错误。`flag=true` 但数字缺失 → 诚实显示「未取到」，不补 0。
- 试用池和套餐包分两行呈现，或明确标注哪个是哪个；把它们相加当成一个池子是错误表达。
- 到期不进 `resets_at`（同 #38 理由：一次性包，非周期重置）。
- 文案走 en/zh/ru 三字典。

## Testing Decisions

- 最高 seam 仍是 `snapshot/parse` 边界：解析器用纯函数 + 合成 JSON 测，不打真实网络。
- 必测：`flag/code` 信封四种组合（0+data、80000、20003、0+null）；`api-use-count` 正常与 `data:null`；`getlfasrInfo` 空数组 / 单张 / 多张乱序到期；`get-rtasr-app-auth` 的秒→分钟、unix 秒→日期、非法 `expireTime`；三种日期字符串的解析与不可解析时不伪造；单位换算不产生负数或 NaN；未登录时不出现「0 额度」。
- 会话测试：保存/加载/清除；vault 回退；按 uid 去重；公开状态接口不泄露 cookie 与 PII。
- 门禁沿用：`parse-tests`（`cd parse-tests; cargo +stable-x8_64-pc-windows-gnu test`）、`node --test tests/*.test.mjs`（含 `provider-credential-flow` 要求每个 provider 有 `MECHANISMS` 条目）、`tests/i18n-pairing.test.mjs`、`catalog_consistency.rs`（Rust family ⇒ `src/providerCatalog.ts`，并留意 disabled-count 断言会因新 family 变化）。
- 真实登录、控制台数字对照、重启后会话持久化由 owner 实机验收；agent 不得凭代码阅读宣称 UI 完成。
- 测试与 fixture 一律脱敏：不写真实 cookie、uid、APPID、订单号。

## Out of Scope

- 不做任何语音推理调用、不代理 TTS/ASR 流量、不碰 APPID/APIKey/APISecret 的签名调用（`webapi/getInfo`、`app/getHttpSign` 会返回明文密钥，本 provider 禁止调用）。
- 不做自动领取套餐、自动实名、自动过验证码；「未领取」只提示，不代点。
- 不做账单/充值/订单管理页；`order/list` 只在需要核规格时作为辅助，MVP 不单独成卡。
- 不做按天用量明细表、Excel 导出、并发量曲线（`get-rtasr-app-use-history` 留作后续）。
- 不读取浏览器 profile 的 cookie 数据库；不做手动粘贴 cookie 的正式交互。
- 不刷新会话（无 refresh 契约），过期即人工重登。
- 非 Windows 平台的登录窗口适配（沿用现有 WebView2 路径）。

## Further Notes

- 实测证据、响应样本、字段语义推断和 7 条待确认都在 `temp/xfyun-console-quota-analysis-20261023.md`；实现时把结论同步进 `docs/providers.md` 新段落，并把未核实项按 AGENTS 规则标 `待确认`。
- 本 provider 与 #38 是**两个 family**，共用机制但不共用登录域、不共用信封判定；把它们合成一个卡片的冲动要先按住。
- 已实测：本轮领取五个套餐后，各接口数字与控制台页面显示一致（合成余量 10000 + 试用池 491/500，因我打了 9 次真实合成）。真实数字属 owner 账号，不写进 Git 文件与 issue。
- 并发 12 次打余量接口无异常，但官方无速率限制承诺，仍按现有刷新节奏。

## 工单拆分

- 01：`xfyun-speech` family + 登录 `console.xfyun.cn` + uid 识别 + 会话持久化与多账号
- 02：三套余量接口解析（含信封失败态、单位换算、应用枚举）
- 03：卡片呈现 + i18n/图标/文档 + 门禁与 owner 验收

- 04（新开，不阻塞本需求）：把 `lib.rs` 的六处 web-session 命令臂收成一张注册表；本需求实现时确认
  加第三家仍要六处各加一行，属同一判断写六遍，但避免在一个未验收的新 family 上同时重构。

阻塞关系：01 → 02 → 03。02 是本需求主价值票；04 独立可并行。

## 实现期的判断记录（2026-10-23，主对话）

- 工单顺序改为 **02 → 01 → 03**：解析器与失败态用合成 JSON 就能全部验完，唯一需要人类动作的是「实登」，
  把它往后推可以避免整批等人。票面原顺序 01→02→03 的阻塞关系（02 依赖 01）在夹具层其实不成立。
- `web_session` 字段化的问题被重新定性：`is_web_session()` 已经是单一入口，真正的重复是 `lib.rs`
  的六处命令臂。收一张注册表是独立重构，另开票，不在本需求里顺手做。
- 成本实测校准：新增一个 web-session family 的真实接线面是 ~13 个代码文件 + 一个 400–500 行
  adapter（#38 主提交 17 文件 +573；tokenrhythm 20 文件 +904 且还需一次 387 行重写才落到
  `platform::read_webview_cookies`）。`web-session × NativeSnapshot` 这个组合本仓没有先例，
  reset/pace/趋势这条链只能在实登后观察。
