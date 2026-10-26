# Spec: /v1/limits 本地 API 契约

**Status:** implemented (2026-10-26) — typed readings are the opt-in gate; the per-label descriptor table was superseded because free-text rows cannot carry a quota by construction. Family lookup, balance units, untouched-window reset omission and docs landed.
**Issue:** https://github.com/Aafff623/pane/issues/73
**Evidence:** temp/research/20261025-final-sweep-7repos.md §1-7（信封/投影/描述符/:id/脱敏/测试全设）

## 问题

openusage 与 QuotaBar 都用 `openusage.limits.v1` schema 暴露机器可读 limits（稳定资源键+原始标量+显式单位）；Pane httpapi 只有 /v1/usage（Mac parity 的 UI 行格式：progress 恒 limit=100、text 预格式化字符串）——混入 limits 会破坏既有消费者，须新增独立路由。

## 范围（2 票）

**票 01 契约+路由+投影（~1.5 天）**：
1. 新模块 limits.rs（纯函数+serde）；httpapi 加 LATEST_LIMITS，publish()（:50-71）同步预计算；route()（:500-536）接 GET /v1/limits + /v1/limits/:id；错误码原样复用（404 provider_not_found/404 not_found/405/403 forbidden_host/503 server_busy）。
2. 信封严格四字段：schema="openusage.limits.v1"（三家互认字串）/generatedAt/providers{卡ID:displayName,plan,fetchedAt,expiresAt,stale,resources{键:kind(consumption|balance),unit,used/limit/remaining/utilization(0-1)/available,resetsAt?,windowSeconds?,estimated?}}/errors[]；**无值省略不造零**；数值过 valid()（finite≥0），utilization clamp 0-1。
3. 投影：progress→consumption（used=used_percent/limit=100/utilization=used/100/unit="percent"/resetsAt iso/windowSeconds=period_ms/1000）；带 usage 读数→取读数；estimated 恒省略（v1 无推算值，字段预留）。
4. **描述符 opt-in 闸**：provider_catalog.rs 静态表 LimitsDescriptor{family_id,label,key,kind,unit}，(family,label) 双键（label 是行唯一身份，改 label 即 fail-closed 掉落）；action 行（Reset credit）/expiry/UI-only 行无描述符即不可进——结构性防泄漏。
5. **:id**：精确卡 ID（onenewapi@abc）或 family ID（onenewapi→全家卡）；家族匹配只在新路由（/v1/usage/:id 保持精确，Mac parity 测试钉死）。
6. 脱敏：resources 只取结构化数值+单位（零自由文本）；errors[]=status≠ok 的卡（code 取 Snapshot.status），message 过 sanitize_text（:144-165）；stale-but-ok 只置 provider.stale。
7. 测试 8 组（信封形状/无值省略/投影数学/UsageReading 源/门控 action+无描述符/:id 三态+Host rebound/脱敏黑名单 sk-·dashboard·apikey/Mac parity 字节不变）；docs/local-http-api.md 端点+示例+安全姿态更新。

**票 02 balance 原始值（~1-2 天）**：
1. Metric 增 `quota: Option<QuotaReading{unit:String,used:f64,limit:Option<f64>}>`（usage_units.rs，**独立于 UsageReading 防污染账本**——balance 是存量不是流量，usage_history.rs:305-312 只收 UsageReading 入累计）。
2. onenewapi/relaybalance/aihubmix/codex/devin 五家族 parse 点附着（**detail 字符串字节不变**，测试断言）；balance_metric 加伴生 balance_quota()。
3. 描述符表加 balance 条目（unit=None=动态币种随读数）；投影 kind="balance"：remaining=余额、used/limit/utilization 省略、available=remaining>0。
4. CHANGELOG 注 openusage.limits.v1 互认。

## 验收

票 01：parse-tests 8 组全绿；`cargo check` 绿；手动 curl /v1/limits、/v1/limits/onenewapi@<id>、/v1/limits/onenewapi 符合信封；/v1/usage 字节不变；docs 更新。票 02：五家族附着+UI 行文本不变；curl 见 balance 资源。

## 协调

mcp/pane-mcp 路径正则已兼容 /v1/limits（lib.mjs:174-203）无 limits 需求不立项；端口 6736 被占则 limits 同静默（docs 明示「后启动者静默禁用，limits 包含在内」）；QuotaBar 互认写发布说明。
