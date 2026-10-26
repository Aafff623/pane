Status: done
Type: task
Blocked by: 04
Priority: P3

# 06 ·（可选）统计口径补齐：turn_usage/tool_usage

## 做什么

1. spend.rs 增读 ZCode turn_usage（会话数/轮数/轮均时长，schema.baseline.sql:283）与 tool_usage（调用数/错误数，:253）——与现有 model_usage 扫描（spend.rs:2636-2972）同连接同回扫窗。
2. KPI 补齐「最长会话时长」与新增「会话数/工具调用」格；热力 tooltip 升级三行式（tokens/会话/成本）。
3. 数据经 spend_history 或独立小聚合暴露给前端。

## 验收

- 本机实测：KPI 数字与 ZCode 官方用量页对账一致（同库同口径）。
- parse-tests 聚合断言；`cargo check` 绿。

## 注意

ZCode usage 三表仅滚动保留 30 天——Pane 侧台账持久化后反而更全；时区口径照抄 dayIndex 本地时区语义（builder :22-48）。
