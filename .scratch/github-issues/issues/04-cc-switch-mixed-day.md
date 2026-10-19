# 04: cc-switch 混合日统计（GitHub #5）

**What to build:** 混合来源同日（部分请求走 cc-switch 代理、部分直连）不再被整体少计后当准确总额显示；同一请求不双计；历史数据兼容；无法可靠合并的来源被明确标记。

**Blocked by:** None

**Status:** implemented（2026-10-18，4c54cc5；保守方案，Review 可推翻）

**来源**：<https://github.com/Aafff623/pane/issues/5>

## 验收标准（issue 原文照抄）

- [x] 增加同日混合来源的回归测试
- [x] 保证同一请求不会重复计入
- [x] 能识别或明确标记无法可靠合并的统计来源
- [x] 不将不完整的汇总误表示为准确总额
- [x] 保持现有历史数据兼容性

## 已知事实（已对照 v0.8.6 源码核实）

`src-tauri/src/spend.rs:1154` `retain_days(&mut all, |day| !cc_days.contains(&day))`；1132-1139 注释自述 day-granular by design。

## 决策记录

- 调查结论（2026-10-18 实查本机 cc-switch.db）：两侧无行级共享身份——`proxy_request_logs` 有 cc-switch 自己的 `request_id`/`session_id`（session 级拆不掉会话内直连/代理混合），>30 天 `usage_daily_rollups` 无任何 id。
- 一轮实现（4c54cc5）：维持按天切 + 「partial ×N」徽章，当时判定依据写的是「本地与 cc-switch 同日双覆盖 = 有直连请求的实证」。
- 二轮评审（2026-10-19）推翻该前提：`proxy_request_logs` 有 `data_source` 列，claude 行含 1903 行 session_log / 11 天——cc-switch 本身就把同一批本地 JSONL 导入账本，双覆盖是它的正常状态，徽章会在大多数活跃日误报「可能偏低」。
- 终态设计（owner 授权「按建议走」下选评审建议的 b 方案，Review 可推翻）：徽章只陈述可证事实——`cc_sourced_days` 数「该卡最终数据中来自 cc-switch 单源的天数」，tooltip 明说这是来源说明、不代表数字不完整。若要「真实少计」的更强信号（评审 a 方案：按 (day, model) 量级比较），留作后续工单。
- **后续工单已落地**（2026-10-19，owner 拍板「要更强版」）：`.scratch/cc-switch-undercount/` 单票实现并合入本分支——`likely_undercount_days` 按天比较两侧 token 总量（同五桶归一化），本地明显多于账本的天记入 `ProviderSpend.likely_undercount_days`，UI 橙色「low? ×N」徽章优先于来源徽章；单向比较（账本高于本地是 relay 清零日志的正常态），双阈值（10% + 2k tokens）为可调判断值；只标记不自动加回（加回=双计）。
