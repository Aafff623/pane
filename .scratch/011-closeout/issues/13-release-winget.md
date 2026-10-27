Status: pending
Type: release
Blocked by: 全部 + owner 发布授权

# 13 · 发布与 WinGet 迁移（#105）

1. 最终验收后：版本号/CHANGELOG/发布说明；owner 授权 push、tag、release。
2. WinGet：
   - `Aafff623.Pane` 0.11.x 新包 bootstrap；
   - `Pane.Pane` 13 个历史版本按 winget-pkgs 流程移除/迁移；
   - 不触碰作者 0.4.x 历史与 publisher/support 元数据；
   - 提交前给 owner 看 PR 内容；外部提交单独确认。
3. 在 #105 更新作者：发布链接、winget PR 链接、身份/遥测脱钩结论。
4. 12-01 Argon 价格切换实机复核提醒。

## 验收

Release 正文符合 `docs/release-format.md`；winget PR 合入或明确挂起；
作者 issue 有最终状态。
