# 01 — GitHub issue 表单

Type: task
Status: implemented (2026-10-25, commit 8ab876b + uncommitted deltas) - GitHub-side visibility unverified until pushed
Blocked by: None
Spec: ../spec.md
Issue: #52 — https://github.com/Aafff623/pane/issues/52

## Scope

- 新增 `.github/ISSUE_TEMPLATE/bug_report.md`：版本/Commit、操作系统与运行时、实际表现与预期、复现步骤、日志或截图。
- 新增 `.github/ISSUE_TEMPLATE/feature_request.md`：要解决的问题、建议方案、替代方案、影响范围。
- 新增 `.github/ISSUE_TEMPLATE/config.yml`：关闭空白 issue，保留一个指向 `docs/project-facts.md` 与 `CONTRIBUTING.md` 的引导链接。
- 模板正文用中文，与仓库现有 issue 语言一致；标题前缀保持 `[Bug]: ` / `[Feature]: `。

## Acceptance

- [ ] 三个文件位于 `.github/ISSUE_TEMPLATE/`，YAML frontmatter 可被 GitHub 解析。
- [ ] 新建 issue 时能看到两个模板，且不能创建空白 issue。
- [ ] 模板只问仓库确实需要的字段，不引入安全策略声明或 CLA 流程。

## Comments

- 2026-10-25：已规格化，尚未实施。模板底稿在本机 `tta-init/assets/templates/github/` 的 mvp 目录，取用时按 Pane 的实际事实改写，不照抄示例版本号。
- 2026-10-25：三个模板已落地（commit 8ab876b，早于本任务指令到达）。版本/Commit 字段按 Pane 实际改写为「Pane 版本」（发布走 tag，用户不掌握 commit 号）；feature 模板补「影响范围」节、config.yml 补 CONTRIBUTING/project-facts 引导链接（本批未提交改动）。模板在 GitHub 新建页的可见性需推送后验证。
