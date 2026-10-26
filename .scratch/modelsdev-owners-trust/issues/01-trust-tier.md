Status: done (2026-10-26)
Type: task
Blocked by: —（#56 已落地，仅协调）

# 01 · modelsdev_trust schema + 仲裁 + 热更即生效

## 做什么

1. supplement 解析 `modelsdev_trust{owners,resellers}`（key 小写 fold，条目数上限仿 MAX_ALIAS_RULES）；Store 增 trust 字段；parse_modelsdev 比较元组升级 (trust,score)。
2. 热更填丁：trust 集变化→从磁盘重读 modelsdev.json 重跑 parse_modelsdev→清 memo→bump GENERATION+LAST_INGEST_MS（复用 :568-569 语义使 collect_daily 的 ingested 检测点火）。
3. parse-tests 6 新+1 守成（spec 验收节）。

## 验收

- 6 新测全绿；现 `modelsdev_prefers_the_most_complete_reseller_entry` 零改断言通过。
- 本机：注入 owners 后 kimi-k3/glm-5.3/deepseek-v4-pro 切官方条目价；移除 key 后逐字节一致。
- 时间线最小版断言：trust 切换后旧 pin 复放不变、仅追加新 pin。

## 证据

temp/research/20261025-final-sweep-7repos.md §1-2（含 live 232 家无 owner 字段实测与 owners 候选清单）。
