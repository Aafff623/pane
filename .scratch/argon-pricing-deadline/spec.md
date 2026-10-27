# Spec: Argon 定价 12-01 变价落地

**Status:** ready · **Priority:** P1 · **Area:** Spend
**Issue:** https://github.com/Aafff623/pane/issues/103
**Evidence:** temp/research/20261026-closing-audits.md §二-2 + 20261025-refrepo-13-refresh-round.md openusage 节

## 问题（时间敏感）

openusage 与 cockpit 双源实证：Antigravity 的 `gemini-4-argon` 定价 **2026-12-01 起变价**——现价 $2 输入/$0.10 缓存读/$10 输出（每百万 token），12-01 起 $4/$0.20/$20。**距今约 5 周**。若不在变价前落地，12-01 后的 Argon 消耗全部按旧价少算 50%。 cockpit 侧已立项 #63-06（Argon 周窗解析）；本票管**定价数据与时效**，两者协同。

## 范围（1 票）

1. **现价落地（立即）**：supplement pricing 补 `gemini-4-argon` 现价三档（input $2/cache_read $0.10/output $10）+ Argon 周窗解析依赖 #63-06（无周窗则用量无从归属，两票协同）。
2. **12-01 变价预案**：#56 时间线机制已落地——变价本质是「观察时间线新 pin」，无需特殊机制；本票负责：① 12-01 前把新价写进 supplement 生效队列（或确认届时 catalog/上游会带新价则只留验证）② 在日历/issue 留 12-01 验证提醒（变价当天本机实测 Argon 卡片金额）。
3. **验证**：parse-tests 补 Argon 定价断言（三档+占位模型 -default 处理已由 #53-01 覆盖，此处只补价额）；本机有 Argon 消耗则实测对账。

## 验收

现价进 supplement 且 parse-tests 断言绿；12-01 验证提醒落 issue 评论/日历；变价当天实测金额正确（或确认上游 catalog 自动带新价）。

## 协调

依赖 #63-06（Argon 周窗解析——无窗口则 token 归属不全）；#56 已落地（变价=新 pin，不追溯）；与 #68 owners 信任分级无重叠（Argon 是一手模型）。

## 12-01 验证清单（owner）

- [ ] 2026-12-01 当天启动 Pane，确认 Antigravity 的 `gemini-4-argon` 新事件按
      $4 输入 / $0.20 缓存读 / $20 输出计价；11-30 及以前的历史金额不变。
- [ ] 若当天上游 catalog 已带新价，确认 baked fallback 未抢先（catalog 优先）。
- [ ] 若金额仍为旧价，导出 `pricing/schedules.json` 检查是否在 12-01 生成了
      `basis = "scheduled"` 的 observation。

实现落点：`pricing.rs` 的 `argon_price_at` / `builtin_price`，`spend.rs` 的
`observe_scheduled_price_change`；测试 `pricing::tests::argon_price_changes_on_december_first`。
