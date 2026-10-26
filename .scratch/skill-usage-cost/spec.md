# Spec: skill 用量×成本（Claude transcripts 旁路）

**Status:** ready · **Priority:** P2 · **Area:** Spend
**Issue:** https://github.com/Aafff623/pane/issues/71
**Evidence:** temp/research/20261025-final-sweep-7repos.md §1-5（旁路论证/游标/隐私/测试全设）

## 问题

TokenTracker 独有角度：回答「哪个已装 skill 真被调用、各花多少钱」。Pane 的 agent_skill.rs 只是安装器（写 ~/.agents/skills/pane），**Pane 无任何 skill 清单读取**（票内需更正此前提）。数据源实证：Skill 调用就在 claude_line 已解析的同一 assistant 行（message.content[] 中 `{type:"tool_use",name:"Skill",input:{skill,args}}`）；本机实测 10 次调用 9 个 skill（含 `superpowers:writing-plans` 命名空间形态）。

## 范围（1 票，~1-1.5 天）

**独立旁路，不做 piggyback**（FileData 全是卡片日聚合语义无格子；ClaudeFileState 被 serde 进 checkpoint，混入累计量搅乱 parser_rev/probe 重放；skill bug 会污染花费数字）。照抄 rtk_savings.rs 先例：新模块+独立存储+独立命令，**spend.rs 零改动**。

1. 新模块 skill_usage.rs：数据模型 SkillRow{skill,calls,tokens,cost,last_used_ms}；口径（最简且诚实）：每个 Skill tool_use→calls+1、tokens+=该行 usage 五桶、cost=该行 costUSD（缺失按 claude_cost 同时点定价）——UI 明示「成本=发起该 skill 调用的那次请求的成本；skill 展开的后续轮次不计入」；去重照抄 claude_line:1709-1723 的 (message.id,requestId)；sidechain 计入但重放不双算；skill 名=input.skill 原样为键（plugin: 前缀展示时拆分组）；**绝不存 input.args**。
2. 游标：独立 %APPDATA%\Pane\skill_usage_cache.json（files:{abs:{offset,boundary_hash,size,skills:{name:[calls,tokens,cost]}}}+totals）；续扫条件同 spend（size>offset+4KiB 边界哈希）；半行尾留下次；shrink/不匹配→全量重读前用 per-file skills 快照从 totals 扣旧账再加新账（可重放）；rev+1 全量重建。与 fetch_spend 并行（spawn_blocking 抄 lib.rs:3857-3874）。
3. UI：设置页新卡片（skillUsage 小节+重新扫描按钮）；不放 auth center/popover/stats 页（理由见 research §1-5）；i18n 8 key 三语。
4. 隐私：只落盘 skill 名+数值；args/prompt 一律不持久化不上屏；测试断言两 JSON 不含 args 子串。
5. 范围：31 天窗口复用（spend.rs:1128），卡片 30 天；Codex 无 Skill 工具调用；ZCode model-io 同构含 skill 字段但主源 SQLite 不带工具事件——留 follow-up，本票只做 Claude。

## 验收

- parse-tests 7 组：单 tool_use/同行多 tool_use 只计 Skill/sidechain 计入+重放不双算/坏行静默跳过/命名空间键原样/游标增量+篡改重读一致/隐私回归。
- `pnpm build` 绿；spend 既有测试零回归（旁路不改 spend.rs，diff 为证）。
- 真机扫描 ~/.claude 产出非空卡片（已知应有 handoff-to-agent×3 等 9 个 skill 可对账）。
- owner UI 验收。
