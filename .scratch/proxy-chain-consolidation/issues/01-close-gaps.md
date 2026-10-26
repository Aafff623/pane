Status: done (2026-10-26)
Type: task
Blocked by: —

# 01 · 代理缺口收拢（cursor_oauth / 回环 no_proxy / 注释 / 纪律）

## 做什么

1. cursor_oauth.rs:207-210：删裸 Client::builder()，改 `crate::providers::http()`（15s 超时诉求在 http_builder 加参数或包一层）。
2. antigravity.rs:157-162：LanguageServer 客户端加显式 `.no_proxy()`。
3. windows.rs:443-447：注释改为「每次 client lookup 重读（成本可忽略，换端口下次请求即生效）」。
4. project-facts.md proxy 段补纪律：「新客户端一律走 http()/http_direct()，禁止裸 builder 打公网；回环永不走代理」。

## 验收

parse-tests+cargo check 绿；本机四实测（system/config/none + Cursor 轮询回归）；grep 无失实注释。
