Status: ready
Issue: https://github.com/Aafff623/pane/issues/74
Priority: P2
Area: Network

# 代理链收拢（系统代理早已实现——补 3 个缺口）

## 背景

TokenTracker 调研曾报「reqwest 未开 system-proxy=系统代理失效（P0）」。深挖**证伪**：Pane 已于 2026-10-09（commit 3b8e9f0）落地注册表探测+单点注入+updater 同步+三语设置（providers/mod.rs:332-352、windows.rs:448-494、lib.rs:5808-5812、main.ts:19983-19988）。且确认**不能补开该特性**（显式 .proxy() 会关闭 auto_sys_proxy；hyper-util 解析不认 Clash 的 `http=x;https=y` 形态——Pane 手写解析更正确）。

## 范围（详见 spec.md，证据 temp/research/20261025-final-sweep-7repos.md §1-0）

- 票 01（必修，2h）：① cursor_oauth.rs:207-210 裸 builder 改走 providers::http()（外网目标 api2.cursor.sh 现在吃不到 config/系统代理）；② antigravity.rs:157-162 回环客户端显式 .no_proxy()；③ windows.rs:443-447 注释修正；④ 「新客户端必须走 http()/http_direct()」写进 project-facts。
- 票 02（可选，半天）：proxy.mode 三档（system/manual/direct）——direct 弥补「系统代理开着但想直连」。

## 验收

票 01：parse-tests/cargo check 绿；四项本机实测（system/config/none 三态 + Cursor 轮询回归 + 注释一致）。票 02：三档切换+旧 config 回退。
