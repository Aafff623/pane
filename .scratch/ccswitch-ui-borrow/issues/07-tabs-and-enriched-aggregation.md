Status: superseded by #64 stats page (2026-10-26)
Type: task
Blocked by: 05

# 07 · 请求数与缓存四分量聚合 + 供应商/模型 tab

## 做什么

1. spend.rs：DayMap 值扩为四 token 分量（fresh_input/output/cache_read/cache_write）+ request_count——ccswitch 源先行（库里有列），其他源尽力（无 cache 拆分的源记 0 并标注口径）；注意与 #56 定价时间线的迁移协调（聚合结构变更走 PERSIST_VERSION）。
2. Hero 后两格：请求数 + 缓存命中率（cache_read÷(fresh+write+read)，usage_stats.rs:49 口径）；点格切换图表指标。
3. 供应商 tab（ProviderSpend 直用）与模型 tab（ModelSpend/daily models[] 前端聚合）：列=请求数/成本/tokens/命中率；页签计数徽标；沿用旧值直到新值到达（防抖动，28a2ebc0 语义）。
4. 速度/首字列为后续项（需新聚合 SQL， Pane 日志 latency 未聚合——本票不做，留扩展）。

## 验收

- Hero 四格与卡片/tab 数字对账一致；命中率在纯 fresh 源显示 —（分母 0 不造 0%）。
- parse-tests 聚合断言；`cargo check`+`pnpm build` 绿。
