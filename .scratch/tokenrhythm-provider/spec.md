# 基元律动（tokenrhythm.studio）Provider：网页端登录态认证

**Status:** implemented（3/3 票，2026-10-19；首登全链路实测通过——密码登录 → 2 秒内窗口自动关闭 → 按钮变「重新登录/清除会话」→ 卡片出「可用额度 ¥142.31 / 冻结额度 ¥0.00」，重启后仍在；parse-tests 885:0:13、tsc/vite、51 node、38 site 门禁全绿。**UI 验收待 owner**——本批收尾时登录态已清空，等 owner 首登后复验）
**前置调研**：`temp/research/20261019-tokenrhythm-api-quota.md`（两轮深扒 + 真 key 实测）

## Problem Statement

基元律动是多模型 AI API 聚合站（OpenAI/Anthropic 协议兼容）。owner 在 ZCode
里配了该 provider（Base URL `https://tokenrhythm.studio/v1`，key 形如
`sk_tr_...`）。诉求：Pane 显示它的账号额度/余额。

**真 key 实测结论（2026-10-19）**：
- `/v1/models` 带真 key → 200（key 可正常调模型，响应带模型价格表）
- 最小对话调用 → 200，响应带标准 `usage` 字段（per-request 用量可见）
- `/api/wallet/summary`、`/api/api-keys/usage` 等**所有**控制台端点带真
  key → 401「未认证或登录已过期」（与无鉴权同结果）
- 文档全量端点穷举 + 隐藏路径猜测（/v1/usage、/v1/balance…）+ OpenAPI
  探测：key 命名空间内无任何用量/额度端点

即：**API Key 只能调模型、看单次用量，账号余额/总额度服务器端只给登录
会话**。故采用网页端登录态认证。

## 已扒清的登录机制（前端 bundle 静态分析 + 探针）

- 控制台 API 前缀 `/api`（runtime-config.js `apiBaseUrl`）
- 相关端点：`/api/wallet/summary`（余额汇总）、`/api/api-keys/usage`
  （按 Key 用量）、`/api/call-logs/page`（调用日志）、`/api/auth/me`
- 登录 API：`POST /api/auth/sms/send` {phone, captchaVerifyParam} →
  challengeId；`POST /api/auth/sms/verify` {challengeId, code}；
  另有密码登录 `/api/auth/login`
- **生产环境鉴权 = 纯 Cookie**（bundle 实锤：Bearer 分支仅 mock 模式走；
  认证判断读 `document.cookie` 里的 `tr_csrf`；变更请求加
  `X-CSRF-Token` header，值取自同名 cookie；会话 cookie 为 httpOnly）
- 短信发送带阿里云验证码参数 → 全自动登录会被验证码墙拦，必须真人过登录
- 空闲超时 24h（runtime-config `idleTimeoutMinutes: 1440`）

## Solution（owner 已选方向；**落地版与初稿有出入，见下方「落地偏差」**）

**Paine 自开登录 webview，用户真人登录；会话持久化在 Paine 自己的
WebView2 配置（EBWebView）里，重启仍在。**

1. **登录命令** `tokenrhythm_login`：在交互桌面开登录窗
   （`WebviewWindowBuilder`，URL `https://tokenrhythm.studio/login`），
   用户在此窗完成登录（验证码真人过）。**登录完成的检测由 Rust 侧轮询
   `/api/auth/me` 负责**，窗口自动关闭。
2. **取数** `tokenrhythm_snapshot`：**读 WebView2 的 cookie manager 取会话
   cookie**（`platform::read_webview_cookies`），缓存进 OS 凭据库后用
   reqwest 带 `Cookie` + `X-CSRF-Token` 调 `/api/wallet/summary`。
3. **卡片**：`tokenrhythm` family，余额型展示（可用额度/冻结额度，
   人民币）；接入既有 refresh 循环（`schedule_*` 模式，仿
   workbuddy_web 的 off-loop 探针）。
4. **Settings**：provider 行内「网页登录」按钮（替代 API Key 输入）；
   登录态指示（已登录/未登录/会话过期）。

### 落地偏差（初稿 → 实装，2026-10-19 首登实测后改）

初稿设想「注入初始化脚本轮询 + `tokenrhythm:ready` 事件回传 + 隐藏
webview 承载 cookie 做站内 fetch」。实装两处都改了，原因如下：

1. **不用 webview 事件、也不用隐藏 webview 取数，改走 cookie manager +
   reqwest**。三个硬约束：(a) 登录窗是 REMOTE origin，Tauri 默认拒绝
   remote webview 发 app command（"not allowed"）；给 capability 显式授权
   又要在 build.rs 声明 AppManifest，那会给全局所有命令打开 ACL 门禁
   （`open_panel_window` 当场被拒），副作用太大。(b) 浏览器进程把 profile
   的 `Cookies` SQLite 以**无共享标志独占锁**住，应用运行期间任何文件读
   必然失败（os error 32 / unable to open database file），而会话 cookie 是
   **httpOnly**，页面 JS 同样读不到——两条路都拿不到会话。只有 WebView2
   的 cookie manager（`ICoreWebView2Profile5`）能读。(c) 既然 cookie 已经
   拿到，站内 fetch 就没必要再靠 webview 承载。
2. **「已登录」必须由网站确认，不能只看有没有 cookie**。登录页在任何人
   认证之前就会落下自己的 first-party cookie（`_c_WBKFRo` / `tr_csrf`），
   初稿的「store 非空即已登录」会把访客 cookie 当会话——用户点「网页登录」
   立刻显示已登录、登录窗不弹、卡片每次请求被 401 拒。现在三处都以
   `/api/auth/me` 的回答为准（`cache_profile_session` 存盘前先问网站；
   `login()` 缓存会话先过网站验证才返回 Ok，过期的一次性清掉；
   `snapshot_for` 卡片吃到 401 时清掉死会话，行状态自纠正）。

## User Stories

1. 作为基元律动用户，我要在 Pane 里一键网页登录（手机号+验证码，和网页
   端体验一致），这样不用手动复制任何 token/cookie。
2. 登录一次后，Pane 重启仍认这个会话，我不用每天重新登录（空闲 24h 内）。
3. 我要在卡片上看到可用额度和冻结额度，会话过期时明确提示重新登录。

## 工单拆分

- 01：登录 webview + ready 事件 + 会话探活（`tokenrhythm_login`）
- 02：隐藏 webview 取数桥（`tokenrhythm_snapshot` + 事件回传 + 401 处理）
- 03：provider 卡片 + Settings 登录按钮 + i18n 三字典 + 票仓收尾

## 验收标准

- [x] 真人登录一次后，卡片显示真实余额（响应结构以首登实测为准）——2026-10-19 owner 账号首登实测：密码登录 → 2 秒内窗口自动关闭 → 按钮变「重新登录/清除会话」→ 卡片出「可用额度 ¥142.31 / 冻结额度 ¥0.00」。响应结构已固化为测试用例（`the_real_me_body_reads_as_signed_in` / `the_real_wallet_body_yields_the_available_balance`）
- [x] 重启 Paine 后会话仍在（WebView2 配置持久化），无需重新登录——同上实测，重启后余额仍在
- [x] 会话过期（24h 空闲）后卡片明确转「需重新登录」，一键重新登录——卡片请求吃到 401 时 `snapshot_for` 立即清掉死会话，行状态自纠正（不必等下一轮刷新）；`a_guest_or_error_body_is_not_signed_in` 覆盖访客/错误体不算已登录
- [x] 取数全过程不把 cookie/token 写入任何 Paine 文件或日志——会话经 `secretstore::save_session` 落 OS 凭据库（scope `web/tokenrhythm`），仅在凭据库写失败时才回退到 `tokenrhythm_web.json`（与项目既有的三处明文例外同一约定）；日志与状态接口均不携带 cookie
- [x] parse-tests 全绿 + tsc/vite/i18n 门禁过；UI 由 owner 实机验收——885:0:13、tsc 无错、51 node、38 site 全绿（2026-10-19 收尾实测）；**UI 验收仍待 owner**（登录态已清空，等首登后复验）

## 已知边界 / 待确认

- ~~**响应结构未实测**~~ —— **已实测（2026-10-19 owner 首登）**：
  `/api/auth/me` 返回信封 `{code, message, data}`，`data` 含 `id` / `name` /
  `status`；`/api/wallet/summary` 同信封，`data.availableBalanceCny` 是
  十进制字符串（实测 `142.31105772`），另有 `giftAvailableCny` /
  `giftLockedCny` / `rechargeBalanceCny` / `currency`。两者的真实结构已固化为
  测试用例。`/api/api-keys/usage`（按 Key 用量）**仍未实现**——当前卡片只
  展示钱包余额。
- 验证码：登录窗内真人过，Pane 不碰验证码逻辑。
- 若站方改登录流程（如加设备验证），登录窗内仍可人工完成，Pane 只认
  `/api/auth/me` 的最终状态。
- **卡片图标仍是波形占位标**，待换基元律动正主标。
- **UI 验收待 owner**：本批收尾时登录态（会话缓存 + 浏览器 profile）已清空，
  等 owner 首登后复验；「解锁后拖着走」等弹窗交互也需 owner 真鼠标确认
  （agent 环境无法把按住的真实鼠标送进 WebView2）。
