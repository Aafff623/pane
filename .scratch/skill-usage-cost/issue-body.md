Status: ready
Issue: https://github.com/Aafff623/pane/issues/71
Priority: P2
Area: Spend

# skill 用量×成本：Claude transcripts Skill tool_use 旁路

## 问题

TokenTracker 独有角度：「哪个已装 skill 真被调用、各花多少钱」。Pane 的 agent_skill.rs 只是安装器，无 skill 清单读取。数据源实证：Skill 调用就在 Claude transcripts 的 assistant 行里（`{type:"tool_use",name:"Skill",input:{skill,args}}`），本机实测 10 次调用 9 个 skill。

## 方案（详见 spec.md，证据 temp/research/20261025-final-sweep-7repos.md §1-5）

**独立旁路**（piggyback 会搅乱 spend 游标/probe 重放且污染花费数字—— FileData/ClaudeFileState 均无格子）：新模块 skill_usage.rs+独立 cache 文件+独立命令，**spend.rs 零改动**。
1. 口径：每个 Skill tool_use→calls+1/tokens/cost=该请求行值（UI 明示「成本=发起调用的那次请求，后续轮次不计入」）；(message.id,requestId) 去重；**绝不存 input.args**。
2. 游标独立（offset+4KiB 边界哈希；全量重读前扣旧账再加新账保可重放）；与 fetch_spend 并行。
3. 设置页新卡片+重新扫描按钮；i18n 8 key 三语；隐私回归测试断言两 JSON 不含 args。

## 验收

parse-tests 7 组全绿+spend 零回归；本机产出非空卡片可与已知 9 个 skill 对账；owner UI 验收。不做：ZCode/Codex 扩展、estimated 标记、按 skill 拆模型成本。
