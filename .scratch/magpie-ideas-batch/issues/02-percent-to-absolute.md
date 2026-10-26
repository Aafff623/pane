Status: done (2026-10-26) — exact vendor allowance live; guarded >=24h estimate; 5h suppressed; owner wording accepted as current copy.
Type: task
Blocked by: —

# 02 · 百分比反推绝对用量（精确/估算双口径）

## 做什么

1. 精确口径（token 套餐：zai/kimi/minimax 类）：剩余 = plan_limit − Σ 各模型消耗；卡片窗口行加「N tokens」副行。
2. 估算口径（黑盒窗口：Claude 5h / Codex 周）：从 resets.rs 窗口起点累计本地消耗 X、厂商已用 p% → 剩余 ≈ X×(1−p)/p；标注「≈ 按你目前的用法」；hover 解释口径。
3. 护栏：日志不覆盖窗口起点不显示；p≥99% 或 ≤1% 不推算；pct>101 视为泄漏丢弃（CCUM 教训）。
4. parse-tests：两口径断言 + 护栏边界。

## 验收

- 本机对智谱/Kimi（精确）与 Claude（估算）实测出数。
- UI 文案经 owner 确认（两种口径的措辞）。

## Comments

- 2026-10-26: 实施与验证见 `temp/handoff/20261026-magpie-borrow-batch.md`；未 commit/push，UI 由 owner 验收。
