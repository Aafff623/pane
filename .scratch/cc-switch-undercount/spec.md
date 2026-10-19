# 识别真实少计：cc-switch 混合日量级比较（cc-switch-undercount）

**Status:** implemented（2026-10-19；owner 在二轮 Review 后拍板「要更强版」，阈值默认 10% + 2k tokens，Review 可调）
**来源**：GitHub #5 二轮评审的 a 方案；owner 2026-10-19 对话确认开工。
**分支**：`codex/github-issues`（与 #3/#4/#5/#6 同分支交付）

## Problem Statement

#5 的徽章一轮只标来源（cc_sourced_days），二轮评审用真实数据推翻「同日双覆盖 = 有直连实证」后，owner 要求能识别**真实少计**的天。少计的确切形态：某天 cc-switch 账本覆盖了该日（本地行被切），但它对本地会话日志的导入漏了记录（或它自己的 session dedup 丢了三），该日卡片数字就低于本地日志实际含量。

## 已查明的事实（设计约束）

- 两侧 token 口径可比：Claude 本地五桶（input/output/cache_read/w5m/w1h）与 cc-switch 归一化后同五桶（`ccswitch_fresh_input` 处理 input_token_semantics 三态）。按天求和后可直接比。
- `cc_claude_days = day_set(&cc.claude)` 取自账本实际有行的天，不存在「切了没人补」的空洞天。
- 比较必须单向：账本**高于**本地是正常且预期（relay 清零 CLI 日志时账本才是真值），只有本地**高于**账本才是少计嫌疑。
- 不能自动加回差额：无法区分哪些本地记录已被导入，直接加=双计（#5 验收标准明令禁止）。

## Solution

- `day_token_totals()` + `likely_undercount_days(local, cc)`：按天求和比较，超过阈值记一天。
- 阈值（可调判断值）：`UNDERCOUNT_RELATIVE_MARGIN = 0.10` +
  `UNDERCOUNT_ABSOLUTE_FLOOR = 2_000.0`——相对边际吸收取整/在写会话/归一化误差，
  绝对下限防止闲日翻标。
- 比较点在 `claude()` 切日**之前**（切了本地行就没了），结果落
  `ProviderSpend.likely_undercount_days`。
- UI：橙色「low? ×N」徽章优先于蓝色「cc-switch ×N」来源徽章；tooltip 明说
  「请把总额视作下限，不是精确值」。
- 回归测试 `likely_undercount_days_flags_only_clear_local_excess`：覆盖
  大超额/账本更高/噪声/账本未覆盖四种形态。

## User Stories

1. 作为同时用 cc-switch 代理和直连的用户，我要面板标出「哪些天很可能少计了」，这样我知道总额是下限而不是准确值。
2. 作为用户，我不想看到把完整数据说成不完整的误报——量级比较单向 + 双阈值保证只在本地明显多于账本时才标。

## 验收标准

- [x] 同日混合来源的回归测试（四种形态）
- [x] 同一请求不会重复计入（比较只读，不改任何数字；差额不加回）
- [x] 能识别并标记无法可靠合并的统计来源（low? 徽章 + 天数）
- [x] 不将不完整的汇总误表示为准确总额（tooltip 明确「视作下限」）
- [x] 保持现有历史数据兼容性（新字段 #[serde(default)]，纯增量）

## 已知边界

- 标记是怀疑不是证明：账本 dedup 丢记录但代理行已覆盖的情形也会被标（方向上仍属「本地多于账本」）。
- 阈值是判断值， owner Review 时可调；量级比较不做模型级（两侧模型名不保证一致，按天聚合规避）。
- 若后续要定位到具体哪条记录漏了，需要 cc-switch 的 session_usage_dedup 表语义，超出本票。
