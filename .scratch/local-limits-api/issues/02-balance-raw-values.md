Status: done (2026-10-26)
Type: task
Blocked by: 01

# 02 · balance 原始值：Metric.quota + 五家族附着

## 做什么

1. usage_units.rs 增 `QuotaReading{unit:String,used:f64,limit:Option<f64>}`（valid() 拒 NaN/负）；Metric 增 `quota: Option<QuotaReading>`（serde default+skip none，四构造函数补 None）。
2. onenewapi（billing.rs:14-49）、relaybalance（:221-242）、aihubmix（:84-115）、codex（:523）、devin（:243）parse 点附着；balance_metric 加伴生 balance_quota()；**detail 字符串字节不变**（测试断言）。
3. 描述符表加 balance 条目（unit=None 动态币种随读数）；投影 kind="balance"：remaining=余额、used/limit/utilization 省略、available=remaining>0。
4. CHANGELOG 注 openusage.limits.v1 互认。

## 验收

parse-tests：余额投影（kind/unit/remaining/available）+ 五家族附着+UI 行文本不变；手动 curl 见 balance 资源；CHANGELOG 更新。
