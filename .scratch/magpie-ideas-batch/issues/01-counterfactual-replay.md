Status: blocked (2026-10-26) — data audit: no grounded compression-size model in current logs; did not ship a fabricated savings estimate.
Type: task
Blocked by: —

# 01 · 反事实重放 MVP（压缩时机推演）

## 做什么

1. 数据可行性盘点（先做）：zcode turn_usage / Claude Code JSONL（含 compact 侧链标记）/ Codex sessions 的会话内轮次级 token 粒度确认，产出对照表。
2. MVP：单场景「若在 T 压缩，本会话省多少 token/$」——context 增长曲线 + 压缩点推演；只读展示（洞察卡），不改用户配置。
3. UI：花费页/会话列表「洞察」徽章 + 推演卡，三语 i18n，数字带「≈」。

## 验收

- 本机真实数据出数且手算可复核；粒度不足的源显示「暂不支持」。
- `pnpm build` 绿；owner 验收文案与口径标注。

## 证据

temp/research/20261025-magpie-deep-dive.md §一（internal/tune/tune.go）。

## Comments

- 2026-10-26: 实施与验证见 `temp/handoff/20261026-magpie-borrow-batch.md`；未 commit/push，UI 由 owner 验收。
