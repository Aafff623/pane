Status: ready
Type: task
Blocked by: 04
Priority: P3

# 08 · 多窗口 Auto 前缀 + 策略链离线兜底

## 做什么

1. 多窗口数值前缀：claude/codex 等双窗口同卡时，Auto/pace 数值加 `5h`/`周` 短前缀——仅当语义窗口集合去重 >1 且选中窗口唯一归因；多窗口同值不加（防单配额看着像多个）（MenuBarLayoutRenderer.swift:675-725 语义）。
2. 离线兜底：策略链全失败时返回旧快照 + diagnostic 降级解释（「缓存于 HH:mm，原因 X」），不显示空卡（AntigravityProviderDescriptor.swift:220-241 语义）。

## 验收

- 单测：前缀判定（多窗口唯一归因/同值不加/单窗口不加）；兜底快照带时间戳与原因。
- owner 场景验收：断网时卡片显示旧值+解释而非空白。
