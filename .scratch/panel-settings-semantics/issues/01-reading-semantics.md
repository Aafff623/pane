# 01 · 读数语义与文案一致

Status: done 2026-10-29 | Priority: P0 | Depends: —

## Scope

- 详情卡：条形宽度与 `--skin-meter`、主读数统一跟随 `config.showUsed`；颜色阈值继续按已用百分比。
- 概览条：百分比与条形改为同一约定（`config.showUsed ? 已用 : 剩余`），文字复用 `card.pctUsed` / `card.pctLeft`，不再输出裸 `%`。
- 卡片头 `.head-pct`：同样带量词，`title` 同步。
- 更新时间：去掉「数据更新于 N 分钟前」分支，统一「已更新 HH:MM」；删除三个语言里的 `footer.updatedAgo`。
- 快捷键列表：`settings.actionExpiring` 实际动作是「到期提醒」而不是「重置倒计时」，修正 en / zh / ru 文案。

## Done when

- 概览、卡片头、详情脚对同一个数字只说一种约定，且都带「已用/剩余」。
- `grep footer.updatedAgo src/` 为空。
- 构建通过。
