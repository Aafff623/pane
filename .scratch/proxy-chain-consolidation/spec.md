# Spec: 代理链收拢

**Status:** implemented (2026-10-26) — ticket 01 shipped; optional three-mode `proxy.mode` remains deferred.
**Issue:** https://github.com/Aafff623/pane/issues/74
**Evidence:** temp/research/20261025-final-sweep-7repos.md §1-0（推翻过程+缺口+不能开特性的理由）

## 背景（发现被推翻的过程）

TokenTracker 调研称「reqwest 未开 system-proxy 特性=Windows 系统代理失效」是 P0。深挖证伪：Pane 已于 2026-10-09（commit 3b8e9f0「feat(http): follow the Windows system proxy automatically」）落地完整链路——注册表探测（windows.rs:448-494 system_proxy/parse_win_proxy，认 `host:port`/`http=x;https=y`/裸 URL 三形态）+ 单点注入（providers/mod.rs:332-334 outbound_proxy_url=config→系统→env 兜底；:336-352 http_builder 含 NoProxy(localhost,127.0.0.1,::1)；http_direct 全局 no_proxy）+ updater 同步（lib.rs:5808-5812）+ 三语设置（main.ts:19983-19988）。docs/project-facts.md:510-514 有记录。
**且确认不能补开 system-proxy 特性**：reqwest 一旦显式 `.proxy()` 就关闭 auto_sys_proxy（client.rs:1416），开了特性只会多一条互不协调的注册表路径；hyper-util 的注册表解析不认 `http=x;https=y`（Clash 默认形态），会让显式链与隐式链互相覆盖。Pane 手写解析反而更正确，且无上游 30s TTL——换端口下次请求即生效。

## 范围（票 01 必修 + 票 02 可选）

**票 01 收拢（~2h）**：
1. cursor_oauth.rs:207-210 裸 builder（15s 超时）改走 `crate::providers::http()`（已含 20s/5s 超时与全代理链；如需保留 15s 在 http_builder 加超时参数）——Cursor 轮询跟随 config/系统代理（目标 api2.cursor.sh 是外网）。
2. antigravity.rs:157-162 LanguageServer 客户端（只打 127.0.0.1）加显式 `.no_proxy()`——把「回环永不走代理」从惯例变契约。
3. windows.rs:443-447 注释修正（称「Read once per app run」，实际每 client lookup 重读——成本可忽略）。
4. 「新客户端必须走 http()/http_direct()、回环不走代理」写进 project-facts proxy 段。

**票 02 三档模式（可选，~半天）**：config `proxy.mode: "system"|"manual"|"direct"`（读时回退旧 enabled：true→manual，否则 system；direct=强制直连，弥补「系统代理开着但想直连」无法表达）。落点：providers/mod.rs:314 proxy_url() 加 direct 短路返 None；main.ts:18533-18545 开关改三选一；i18n 三语；lib.rs:195 播种补 "mode":"system"。

## 验收

票 01：parse-tests 全绿；`cargo check` 绿；本机实测（owner）：a) 系统代理开+env 清空→海外 provider 刷新成功、/v1/usage 的 config.proxy.source 报 system；b) config 手动档→报 config 且成功；c) 全直连→报 none；d) Cursor 登录轮询在系统代理开+env 清空下可完成（缺口回归）；e) 注释与实现一致（grep 无失实）。
票 02：三档切换实测+旧 config 无缝回退。

## 不做

不开 system-proxy 特性；不动 updater 链；不引入 TTL 负缓存（现状无 TTL 更优）。
