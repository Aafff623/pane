# 02: 隐藏 webview 取数桥

**What to build:** `tokenrhythm_snapshot`：惰性创建隐藏 webview（同
origin 承载 cookie），`eval_js` 执行站内
`fetch('/api/wallet/summary',{credentials:'include'})`，JSON 经
`tokenrhythm:data` 事件回传 Rust；401 → 卡片转「需重新登录」态。

**Blocked by:** 01

**Status:** implemented（2026-10-19；**实现与初稿有出入**——初稿的「隐藏 webview 承载 cookie 做站内 fetch」整条路走不通（profile 的 Cookies SQLite 被浏览器进程无共享标志独占锁，读文件必失败；会话 cookie 又是 httpOnly，页面 JS 读不到），改为经 **WebView2 cookie manager**（`platform::read_webview_cookies` → `ICoreWebView2Profile5`）读出会话，缓存进 OS 凭据库（scope `web/tokenrhythm`），reqwest 带 `Cookie` + `X-CSRF-Token` 直接调 `/api/wallet/summary`。401 → 需重新登录（并清掉死会话）。响应结构已按首登实测校准）

## 要点

- ~~隐藏 webview 惰性创建、常驻复用；失败/过期时可销毁重建~~ ——
  **不适用**：改走 cookie manager + reqwest，不再需要隐藏 webview 承载 cookie
- ~~fetch 全程在 webview 内，cookie 不经过 Paine、不落盘~~ —— 改由 reqwest
  直发请求；会话经凭据库存放（凭据库写失败时回退到
  `tokenrhythm_web.json`，与项目既有的明文回退约定一致）
- ~~事件回传带 path 标识（wallet / keys-usage 可分多次取）~~ —— 不适用
  （无事件通道）；当前只取 wallet，`/api/api-keys/usage`（按 Key 用量）未实现
- 防御式解析：字段名以首登实测为准，未命中字段记 `待确认` 不猜数 ——
  已按 2026-10-19 实测结构实现（`data.availableBalanceCny` 十进制字符串），
  未识别结构返回「无行」而非臆造数字
- 超时与取消：reqwest 请求要有上限（参考既有 httpapi/onenewapi 的超时常量级）

## 验收

- [x] 已登录态取回真实余额 JSON 并解析成功 —— 2026-10-19 实测出
  「可用额度 ¥142.31 / 冻结额度 ¥0.00」；真实结构已固化为测试用例
  （`the_real_wallet_body_yields_the_available_balance`）
- [x] 会话过期（401）时 Snapshot 明确表达「需重新登录」而非错误卡片 ——
  `snapshot_for` 遇 `CallErr::Auth` 返回 `no_credentials` 并清掉死会话，
  行状态自纠正，不必等下一轮刷新
- [x] 取数无 cookie/token 写入 Pane 文件或日志 —— 会话经
  `secretstore::save_session` 落 OS 凭据库；日志与 `LoginStatus` 接口均不
  携带 cookie（`LoginStatus` 只有 `signed_in` / `saved_at`）
- [ ] **UI 复验待 owner**：收尾时登录态已清空，等首登后复验卡片与设置行
