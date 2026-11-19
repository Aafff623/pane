# Spec: 上游格式漂移的三处修复（0.12.3）

**Status:** implemented 2026-11-18 — 已发布范围，待 CI 构建 · **Owner:** Aafff623

## 现象

三件事在同一轮里暴露，共同的形状是「上游变了，我们静默地少算了数据」：

1. **Qoder CN 用量统计空白**：客户端 0.5.14 把转录 `usage` 里的 `credits` /
   `original_credits` / `billable` 整段删除，解析器遇到没有 credits 的记录直接
   `continue`，于是 11-14 之后每一次请求都被丢弃。
2. **cc-switch 账本只读 8 个 app**：cc-switch v4.0.21–v4.0.35 新增了 Kimi / DSH /
   WorkBuddy / Antigravity 四个导入器，本机账本里这四类共 610M tokens，Pane 的
   `CcSwitchData::bucket()` 对它们返回 `None`，全部丢弃。
3. **两类失败无声**：文件被外部截断/改写时静默整份重读（可能几百 MB），
   以及某源扫到文件但一条用量都没解析出来——两者都不会出现在任何地方。

## 决定

### Qoder CN

- 解析器接受「没有 credits 但有真实 token 计数」的记录。
- 实测 1228 条：`input_tokens ÷ context_usage_ratio` 恒等于 1,000,000，说明新计数
  与旧「占比×窗口」同口径，可直接替换。
- 有实测计数优先，否则回退估算；credits 确实缺失时卡片显示「—」而不是 0。
- 新增 `credits_missing_trips` / `exact_token_trips` 两个来源标记。

### cc-switch 账本

- `CcSwitchData` 增加 kimi / dsh / workbuddy / agy 四个桶。
- 接入沿用既有 `merge_missing_days`：Pane 自己扫过的天优先，cc-switch 只补缺口，
  同一笔请求不双算。
- `sources/mod.rs` 的 `ccswitch-sqlite.feeds` 同步补 4 个 id。

### 扫描诊断

- 逐文件记录「打开 / 是否解析出用量」，汇总成**源级**结论：一个索引文件为空是正常的，
  整个源都为空才是格式变更的信号。
- 记录增量续读被放弃的原因（截断 / 改写 / 价格表变更 / 无断点 / 边界失配）。
- `SourceSpec` 新增 `scan_family`：SQL 与网络类源不参与文件哨兵，避免借用别的
  family 的计数器（cc-switch 的 `feeds[0]` 恰好是 `claude`，曾造成误报）。

## 边界

- 不新建 SQLite 明细库——Pane 继续只读消费 cc-switch 的账本，避免重复它的剪枝体系。
- 不做全站字号/颜色令牌重构：字号有 11 个档位、硬编码颜色数十处，属于系统性改造，
  风险高于收益，留给独立版本。

## 验证

- `parse-tests` 1223 passed / 0 failed / 19 ignored（serial, proxy-free）。
- 前端 258 passed；site 7 passed；i18n 三语配对通过。
- 实机锚点：Qoder 今日 1411 请求 / 156.4M tokens 正常显示；cc-switch 四类
  610,800,942 tokens 从前丢弃、现在读回；扫描诊断在 codex 36 文件 / antigravity
  66 文件上无假阳性。
