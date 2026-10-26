Status: done (2026-10-26)
Type: task
Blocked by: —

# 02 · 泄漏值 >101 护栏（共享层）

## 做什么

1. 前置审计：扫全部 `Metric::progress` 调用点（约 102 处/30+ provider），区分「合法 >100」的超支 meter（在调用处 clamp，claude.rs:351 先例）与常规百分比。
2. providers/mod.rs `Metric::progress` 加守卫：`!finite || used_percent > 101.0 → Self::text(label, "Usage unavailable")`（101 留四舍五入余量；100.0/100.9 合法）。
3. 不动全局 clamp 本身；不碰 claude.rs 源头归一（归 claude-utilization 票）。

## 验收

- parse-tests：epoch 级值（1.7e12）/负值/NaN → text 行不误告警；合法 100.9 仍 progress。
- 审计结论（超支 meter 清单）写进票备注；`cargo check` 绿。
