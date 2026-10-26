Status: ready
Issue: https://github.com/Aafff623/pane/issues/73
Priority: P2
Area: API

# /v1/limits 本地 API 契约（openusage.limits.v1 三家互认）

## 问题

openusage/QuotaBar 都有机器可读 limits（`openusage.limits.v1`：稳定资源键+原始标量+显式单位）；Pane 只有 /v1/usage（Mac parity UI 行格式）——须新增独立路由而非扩展。

## 方案（详见 spec.md，证据 temp/research/20261025-final-sweep-7repos.md §1-7）

- 票 01：limits.rs 信封（schema/generatedAt/providers/errors 严格四字段，无值省略不造零）+ 路由接线（错误码复用）+ 投影（progress→consumption）+ **描述符 opt-in 闸**（provider_catalog 静态表，(family,label) 双键，action/UI-only 行无描述符不可进——结构性防泄漏）+ :id（精确卡 ID 或 family ID，家族匹配只在新路由）+ 测试 8 组+docs 更新。
- 票 02：Metric 增 `quota: Option<QuotaReading>`（独立于 UsageReading 防污染账本）；onenewapi/relaybalance/aihubmix/codex/devin 五家族附着（detail 字符串字节不变）；balance 投影；CHANGELOG 注互认。

## 验收

8 组测试全绿；curl 三形态符合信封；/v1/usage 字节不变（Mac parity）；五家族余额可机读且 UI 行不变；docs/local-http-api.md 更新。
