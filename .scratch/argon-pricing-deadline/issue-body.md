Status: ready
Issue: https://github.com/Aafff623/pane/issues/103
Priority: P1
Area: Spend

# Argon 定价 12-01 变价落地（时间敏感，距今约 5 周）

## 问题

gemini-4-argon 定价 2026-12-01 起变价（$2/$0.10/$10 → $4/$0.20/$20 每百万 token），openusage+cockpit 双源实证。不在变价前落地则 12-01 后 Argon 消耗全部少算 50%。

## 方案（详见 spec.md，证据 temp/research/20261026-closing-audits.md §二）

1. 现价立即进 supplement（三档）+parse-tests 断言。
2. 12-01 变价预案：#56 时间线已落地（变价=新 pin 不追溯）；本票负责把新价排进生效队列+留变价当天验证提醒。
3. 依赖 #63-06（Argon 周窗解析）协同。

## 验收

现价断言绿；12-01 验证提醒在案；变价当天实测正确。
