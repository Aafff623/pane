Status: done (2026-10-26)
Type: task
Blocked by: —

# 02 · 关于页更新态（更新到 vX / 新版本行 / release 直达 / Star 横幅）

## 做什么

1. 主按钮四态：检查更新 / 检查中 / **更新到 vX（solid 高亮）** / 更新中——按现有 updateVersion 切换（main.ts:18416-18460），复用 update-available 事件，无新后端。
2. 有新版时追加「检测到新版本 vX」行 + release notes 摘要（line-clamp 3 行）。
3. 更新日志按钮直达 GitHub release tag URL（按 updateVersion 拼）。
4. Star 邀请横幅：一行卡片 + 可关闭（localStorage `pane:about:starDismissed`）；关闭后 GitHub 链接补回链接行。

## 验收

- 模拟 update-available 事件 → 绿点/按钮态/新版本行三处同步；横幅关闭后刷新仍隐藏。
- owner 视觉验收（对照 cc-switch 关于页截图）。
