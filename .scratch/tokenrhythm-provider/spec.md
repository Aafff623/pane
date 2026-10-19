# 基元律动（tokenrhythm.studio）Provider：网页端登录态认证

**Status:** spec（2026-10-19；owner 拍板「实在没有 key 途径就采用网页端认证，登录态保存本地」）
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

## Solution（owner 已选方向）

**Paine 自开登录 webview，用户真人登录；会话持久化在 Paine 自己的
WebView2 配置（EBWebView）里，重启仍在；取数用站内 fetch（cookie 由
webview 自动携带，不经过 Paine、不落任何明文）。**

1. **登录命令** `tokenrhythm_login`：在交互桌面开登录窗
   （`WebviewWindowBuilder`，URL `https://tokenrhythm.studio/login`，
   `withGlobalTauri`），注入初始化脚本轮询 `/api/auth/me` 直至 200，
   通过 `window.__TAURI__.event.emit('tokenrhythm:ready')` 通知 Rust，
   Rust 关闭窗口。用户在此窗完成手机号+验证码登录（验证码真人过）。
2. **取数命令** `tokenrhythm_snapshot`：惰性创建一个**隐藏** webview
   （同 origin，承载cookie），`eval_js` 执行
   `fetch('/api/wallet/summary',{credentials:'include'})`，
   结果经 `tokenrhythm:data` 事件回传 Rust，解析成 Snapshot；webview
   保持存活供下次刷新复用（会话过期时事件带回 401 → 卡片转「需重新
   登录」，设置页提供重新登录入口）。
3. **卡片**：`tokenrhythm` family，余额型展示（可用额度/冻结额度，
   人民币）；接入既有 refresh 循环（`schedule_*` 模式，仿
   workbuddy_web 的 off-loop 探针）。
4. **Settings**：provider 行内「网页登录」按钮（替代 API Key 输入）；
   登录态指示（已登录/未登录/会话过期）。

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

- [ ] 真人登录一次后，卡片显示真实余额（响应结构以首登实测为准）
- [ ] 重启 Paine 后会话仍在（WebView2 配置持久化），无需重新登录
- [ ] 会话过期（24h 空闲）后卡片明确转「需重新登录」，一键重新登录
- [ ] 取数全过程不把 cookie/token 写入任何 Paine 文件或日志
- [ ] parse-tests 全绿 + tsc/vite/i18n 门禁过；UI 由 owner 实机验收

## 已知边界 / 待确认

- **响应结构未实测**：`/api/wallet/summary`、`/api/api-keys/usage` 的
  JSON shape 需要 owner 首登后才能抓（解析器先按字段名防御式实现，
  首登后校准）。
- 验证码：登录窗内真人过，Paine 不碰验证码逻辑。
- 若站方改登录流程（如加设备验证），登录窗内仍可人工完成，Paine 只认
  `/api/auth/me` 的最终状态。
