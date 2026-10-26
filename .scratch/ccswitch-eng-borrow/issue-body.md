Status: ready
Priority: P3
Area: Engineering

# cc-switch 工程借鉴：自报成本 / i18n parity 测试 / 设置事务保存 / CI 发布门禁 / 错误与托盘小件

## 背景

cc-switch（MIT）与 Pane 同栈，工程实践成熟。用户确认五项借鉴（另见 #61 UI 借鉴批次）：自报成本优先、i18n 三语 parity 测试、设置事务性保存、CI 路径过滤+release 文档门禁、错误结构化+托盘指纹重建。证据 file:line 见 `.scratch/ccswitch-eng-borrow/spec.md` 与 `temp/research/20261025-ccswitch-deep-dive.md`。

## 范围

- **01 自报成本优先**：日志自带精确成本（如 Grok costUsdTicks）入账优先，本地价补缺+漂移告警；先 grep 真实样本确认哪些源带字段。
- **02 i18n parity 测试**：三段断言（key 集合 / 插值变量 / 品牌词保留），node --test，0.5 天。
- **03 设置事务性保存**：diff→逐字段副作用→失败回滚整次报失败；适用于有联动动作的设置项。
- **04 CI 门禁**：ci.yml 路径过滤骨架 + release 前 CHANGELOG/what's-new 存在性可失败检查。
- **05 错误结构化 + 托盘指纹**：小 error.rs（thiserror+JSON 错误串）；托盘只在问题态指纹变化时重建。

## 明确不做

TanStack Query（评估后不引入，只抄 queryKey 命名空间失效语义到既有事件监听群）；live/ 配置切换层；SQLite 迁移体系；React hook 结构；绘画/提示词。

## 验收

见 issues/ 各票；02/04 机械可验；01/03/05 owner 场景验收。
