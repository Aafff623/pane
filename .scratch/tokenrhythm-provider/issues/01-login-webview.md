# 01: 登录 webview + ready 事件 + 会话探活

**What to build:** `tokenrhythm_login` Tauri 命令：在交互桌面开登录窗
（`https://tokenrhythm.studio/login`），用户完成手机号+验证码登录；注入
初始化脚本轮询 `/api/auth/me` 直至 200，经
`window.__TAURI__.event.emit('tokenrhythm:ready')` 通知 Rust 后关窗。
会话持久化在 Paine 自己的 WebView2 配置（EBWebView），重启仍在。

**Blocked by:** None

**Status:** implemented（2026-10-19；真人首登待 owner——窗口内过手机号+验证码，会话落入 Paine WebView2 配置）

## 要点

- 窗口必须走交互桌面启动（WinSta0\Default，参考 lib.rs 既有启动路径），
  不能 `Start-Process`/裸 spawn
- `WebviewWindowBuilder` + `with_global_tauri(true)` +
  `initialization_script`（轮询 + emit）；`eval_js` 不需要回值
- 关窗前再 emit 一次带 `/api/auth/me` 的 user 摘要（可选，用于设置页
  展示登录账号）
- 登录窗关闭后不杀 webview 进程组之外的东西；只关这一个 window
- 事件名集中定义（Rust 常量 + JS 字符串常量），避免漂移

## 验收

- [ ] 真人登录后 `tokenrhythm:ready` 到达、窗口自动关闭
- [ ] 重启 Paine 后 `/api/auth/me` 在同 origin 隐藏 webview 仍 200
- [ ] 未登录/取消登录不产生僵尸窗口；parse-tests 不因此变红
