# Spec: 上游状态探测（Statuspage 故障归因）

**Status:** ready · **Priority:** P2 · **Area:** Refresh
**Issue:** https://github.com/Aafff623/pane/issues/70
**Evidence:** temp/research/20261025-final-sweep-7repos.md §1-4（放置/缓存/字段/UI/隐私全设）

## 问题

厂商故障期间，用户把红色/过期的额度行误读成「Pane 坏了」。Pane 的 should_mark_stale（providers/mod.rs:826，超 3min 宽限+连续≥2 失败）只有 stale 标记，没有「厂商服务状态」维度。

## 范围（1 票，~1 天）

1. 新模块 `providers/upstream_status.rs`（纯函数+HTTP 分离，进 parse-tests）。
2. 放置：lib.rs:3475-3488 merge 循环收集探测目标（stale||error 的 family、端点表内、config 未关、缓存不 fresh）→临界区外 spawn→attach(&mut s) 写 snapshot。**不进 IN_FLIGHT、不拿 usage_fetch_lock**（只读旁路，与 #54 零交集）。
3. HTTP：GET 仅 Accept: application/json，**无 token 无用户数据**，6s 超时，复用 http() 代理链，限 body。
4. 缓存（内存 static Mutex）：Entry{result,probed_at_ms}；TTL 5min 不重探；失败保留 last-good 30min 后自然过期 None；fail-soft 永不 throw 永不改刷新结果。
5. Snapshot 增 `upstream_status: Option<UpstreamStatus{indicator,description,updated_at}>`（serde default+skip none；四构造函数补 None；旧快照兼容）。
6. UI：stale chip 文案不变；staleHelp()（main.ts:397-414）warning 行后、fix 行前插「上游状态：Claude 故障（major）· 14:32\n{description}」；indicator→文案：**none→不插行**（上游正常=失败在 Pane/网络侧，插了误导）、minor→部分故障、major→故障、critical→严重故障、未知→不显示。
7. 端点表 `family → Option<&'static str>`，**初始仅 claude→status.claude.com/api/v2/status.json**（Statuspage.io 契约 status.indicator none/minor/major/critical）；其他厂商一律不预设，逐家实证后单独加票。
8. config 键 `upstreamStatusProbe`（默认 true，实际只对表内 family 生效）；Settings 隐私区可关，关闭后连缓存读取也停。
9. privacy.md:12 网络表新增 status.claude.com 行（匿名 GET 零用户数据）。

## 验收

- parse-tests 4 组：indicator 映射四值+none/未知不显示；TTL 状态机（fresh/last-good 30min/>30min None/成功重置）；Statuspage JSON 解析（正常/坏 JSON/缺字段/非 200 全 fail-soft）；serde 兼容 round-trip。
- `cargo check`+parse-tests+`pnpm build` 绿。
- 手测：mock/真实端点下 stale 卡 tooltip 显示归因行；none 不显示；关闭后零出网。
- privacy.md 已补；owner UI 验收（stale chip 视觉不变）。

## 协调

#63-01 Retry-After（warning=等多久在前、归因=为什么居中、fix 在后；none 不抢位）；#63-02 不同层；#54 闸门外只读；#55 只碰 tooltip 数值渲染零改动。
