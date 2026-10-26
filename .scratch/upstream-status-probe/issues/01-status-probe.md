Status: done (2026-10-26)
Type: task
Blocked by: —

# 01 · upstream_status 模块 + 探测 + Snapshot 字段 + UI 归因行

## 做什么

1. providers/upstream_status.rs：纯函数（indicator→文案四值+none/未知不显示；TTL 状态机；Statuspage JSON 解析）+HTTP 探测（GET Accept json only，6s 超时，复用 http() 代理链，限 body，无 token）。
2. 缓存 static Mutex Entry{result,probed_at_ms}：TTL 5min；失败 last-good 30min；fail-soft。
3. lib.rs:3475-3488 merge 循环收集目标（stale||error+表内+config 开+缓存不 fresh）→临界区外 spawn→attach 写 Snapshot.upstream_status（serde default+skip none，四构造函数补 None）。
4. main.ts staleHelp()（:397-414）warning 行后插归因行（major/critical/minor 分级文案+description+updated_at）。
5. 端点表仅 claude；config upstreamStatusProbe（Settings 隐私区可关）；privacy.md 补 status.claude.com 行。
6. parse-tests 4 组断言。

## 验收

spec 验收节全项；探测不进 IN_FLIGHT 不拿锁（与 #54 零交集）；关闭开关后零出网（手测）。
