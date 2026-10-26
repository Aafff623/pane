Status: ready
Issue: https://github.com/Aafff623/pane/issues/70
Priority: P2
Area: Refresh

# 上游状态探测：Statuspage 故障归因（Claude seeded）

## 问题

厂商故障期间用户把过期/红色额度行误读成「Pane 坏了」。Pane 只有 stale 标记（连续 2 次失败），没有「厂商服务状态」维度。

## 方案（详见 spec.md，证据 temp/research/20261025-final-sweep-7repos.md §1-4）

1. 新模块 providers/upstream_status.rs：失败/stale 时按 family 异步探 Statuspage（GET 无 token 无用户数据，6s 超时，fail-soft 永不 throw）。
2. 缓存：内存 TTL 5min+失败保留 last-good 30min；**不进 #54 闸门、不拿锁**（只读旁路）。
3. Snapshot 增 upstream_status 可选字段；UI 只在 stale tooltip 追加一行归因（indicator=none 不显示）。
4. **端点表只 seeded Claude 一家**（status.claude.com/api/v2/status.json）；其他厂商逐家实证后单独加票，不预设。
5. config upstreamStatusProbe 默认开可关；privacy.md 网络表补一行（唯一非厂商-API 出网新端点，匿名零数据）。

## 验收

parse-tests 4 组断言全绿；cargo check/pnpm build 绿；mock 下 stale tooltip 显示「上游故障：major」+description，none 不显示，关闭后零出网；owner UI 验收。

## 不做

不做厂商状态首页卡片/推送通知；不预设未实证端点；探测不影响刷新结果。
