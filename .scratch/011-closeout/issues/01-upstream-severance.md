Status: done (2026-10-26)
Type: task
Blocked by: —

# 01 · 与上游作者彻底脱钩审计

## 结果

- 活跃代码/配置无 `ItsJazii`、`com.jazii.pane`、`Pane.Pane`、`trypane.xyz`、
  `pane.jazii.dev` 端点；只剩 `lib.rs` 的历史注释/负向断言与 CHANGELOG 记录。
- 应用身份：`Pane-Aafff623` / `com.aafff623.pane` / `Aafff623.Pane`。
- 遥测：`telemetry.rs` 已删，无 PostHog token/端点。
- 更新链：只走 `pane.threetwoa.live` 与本仓库 GitHub Releases。
- oil-ui：owner 已买断 Pro；许可按自有授权记录，不复制上游第三方素材。

## 验收

全仓 `grep` 无活跃上游依赖；发布/安装/数据目录均使用 Aafff623 身份。
