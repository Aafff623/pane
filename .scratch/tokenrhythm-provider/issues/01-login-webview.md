# 01: 登录 webview + ready 事件 + 会话探活

**What to build:** `tokenrhythm_login` Tauri 命令：在交互桌面开登录窗
（`https://tokenrhythm.studio/login`），用户完成手机号+验证码登录；注入
初始化脚本轮询 `/api/auth/me` 直至 200，经
`window.__TAURI__.event.emit('tokenrhythm:ready')` 通知 Rust 后关窗。
会话持久化在 Paine 自己的 WebView2 配置（EBWebView），重启仍在。

**Blocked by:** None

**Status:** implemented（2026-10-19；首登全链路实测通过——密码登录 → 2 秒内窗口自动关闭 → 按钮变「重新登录/清除会话」→ 卡片出「可用额度 ¥142.31」，重启后仍在。**实现与初稿有出入**：不用 webview 事件回传、不用隐藏 webview 取数，改走 WebView2 cookie manager + reqwest，理由见 spec「落地偏差」。UI 验收待 owner）

## 要点

- 窗口必须走交互桌面启动（WinSta0\Default，参考 lib.rs 既有启动路径），
  不能 `Start-Process`/裸 spawn
- ~~`WebviewWindowBuilder` + `with_global_tauri(true)` +
  `initialization_script`（轮询 + emit）；`eval_js` 不需要回值~~ ——
  **实装未用**：登录窗是 REMOTE origin，Tauri 默认拒绝 remote webview 发
  app command（"not allowed"），给 capability 授权又要在 build.rs 声明
  AppManifest（会给全局所有命令打开 ACL 门禁，`open_panel_window` 当场被拒）。
  改由 **Rust 侧轮询** `/api/auth/me`：200 + 信封 `code` 0 即登录完成，
  窗口自动关闭。登录窗本身不再需要任何 IPC。
- ~~关窗前再 emit 一次带 `/api/auth/me` 的 user 摘要~~ —— 不适用（无事件通道）
- 登录窗关闭后不杀 webview 进程组之外的东西；只关这一个 window
- ~~事件名集中定义（Rust 常量 + JS 字符串常量），避免漂移~~ —— 不适用（无事件通道）

## 验收

- [x] 真人登录后窗口自动关闭 —— 2026-10-19 实测：密码登录后 2 秒内自动关闭
- [x] 重启 Paine 后会话仍在 —— 实测重启后余额仍在（会话在 Pane 自己的
  WebView2 profile 里，随浏览器进程持久化）
- [x] 未登录/取消登录不产生僵尸窗口；parse-tests 不因此变红 —— 885:0:13 全绿
- [ ] **「已登录」必须是网站确认的，不是有 cookie 就算**（2026-10-19 用户
  实测暴露的缺陷，已在 `fix(provider)` 提交里修：登录页在认证前就落下
  `_c_WBKFRo` / `tr_csrf`，初稿的「store 非空即已登录」会把访客 cookie 当
  会话，导致点登录立刻显示已登录、窗口不弹、卡片被 401 拒。现三处一律以
  `/api/auth/me` 为准）—— **待 owner 首登后复验**
