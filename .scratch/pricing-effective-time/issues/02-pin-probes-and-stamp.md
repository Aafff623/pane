Status: done (2026-10-25)
Type: task
Blocked by: 01

# 02 · 探针改造与 catalog_stamp 止血

## 做什么

1. `PriceProbe` 增加 `Pin { scope, key, ts_ms, pin: Option<PricePin> }` variant，替换 Lookup/LookupAt/FastMult（spend.rs:320-419）；探针复放 = 用同一 ts 重查时间线。`PERSIST_VERSION` 5→6（旧缓存整份弃用，一次性全重解析，发布说明明示）。
2. `catalog_stamp()` 降级为快路径优化（完全相同则跳过复放），不再承担正确性。
3. `CORRECTIONS_REV` 变化不再整份弃缓存（改 spend.rs:509 路径）：启动检测到 rev 变化 → 对所有在用 (scope, model) 追一条 basis:"correction" 观察，仅向前生效。
4. override 保存（pricing.rs:582-593）= 向所有在用 scope 追一条 basis:"override" 观察；`lookup_at` 移除 override 优先分支（仅 `lookup()` 保留供新 pin 解析）。
5. unpriced 也记 `Pin{pin:None}`（修 spend.rs:353-355 的反向缺口：后补价只触发一次「填充」）。

## 验收

- parse-tests：探针复放在 catalog 变化后恒真（文件不重解析）；CORRECTIONS_REV bump 缓存不弃、旧日不变；override 保存仅向前；unpriced→priced 一次填充后冻结；stamp 空转零重解析。
- 重解析无害性：overflow 文件（>64 模型）在 catalog 变化后重解析，美元逐字节相等。
