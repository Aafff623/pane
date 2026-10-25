Status: ready
Type: task
Blocked by: 01, 02

# 03 · 扫描器迁移：scope 字段 + 全量走 resolve_pin_at + record_observations

## 做什么

1. `FileEntry/PersistEntry` 加 `scope` 字段（scanner 注册时写入，与 provider card id 严格对齐；注册表单测校验 scope 集合）。
2. 所有 `probe_lookup` 调用点改走 `resolve_pin_at`：请求级源 seg_end = 事件 ts；日级源（cc-switch/workbuddy/codebuddy 的 add_day_event）seg_end = 当地日 23:59:59.999。现存三处时戳定价（codex:2249/copilot:205/gemini:3561）为迁移样板。
3. `ensure_fresh` fetch 循环结束后若发生 ingest → `record_observations()`：枚举在用 (scope, model) → 新 catalog 逐 scope 解析 pin → 与时间线末条 diff → 仅变化者追加，observed_at = 本次 ingest 时刻 → 原子写。
4. 扫描期首见：resolve_pin_at 未命中 → 实时解析并记观察，observed_at = 当前 catalog epoch（最近一次成功 ingest 时刻）；解析失败不写条目。
5. Mutex + 原子写保护时间线并发（ensure_fresh 的 diff 与扫描线程同在阻塞线程，实际串行，仍加固）。

## 验收

- parse-tests：日内调价分段（10:00/15:00 事件 + 12:00 观察 → 旧价×早段+新价×晚段）；日级源日末规则；alias/倍率变化 → 新观察、旧文件美元不变；checkpoint 续扫旧行美元保留。
- 集成验证：mock catalog ingest → schedules.json 仅追加变化项；拉取失败 → 无新观察 → 无追溯。
