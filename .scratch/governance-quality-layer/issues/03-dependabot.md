# 03 — 依赖更新自动化

Type: task
Status: implemented (2026-10-25, commit 8ab876b) - one deviation for owner decision
Blocked by: None
Spec: ../spec.md
Issue: #52 — https://github.com/Aafff623/pane/issues/52

## Scope

- 新增 `.github/dependabot.yml`：`npm`（根目录，`pnpm-lock.yaml`）、`cargo`（`src-tauri/`）、`github-actions`（`/`）。
- 三条规则统一 `interval: monthly`，`open-pull-requests-limit: 3`，minor 与 patch 合并成组，major 单独开。
- 不配置 `automerge`，不配置 `assignees` 之外的策略。
- 提交配置不等于启用：`dependabot.yml` 进入默认分支后才开始工作。

## Acceptance

- [ ] YAML 可解析，`version: 2`，三个 ecosystem 路径与实际锁文件位置一致。
- [ ] `gh api repos/Aafff623/pane/contents/.github/dependabot.yml` 能读到文件。
- [ ] owner 明确答复是否让它在默认分支生效；未确认前只落文件，不宣称已开始收 PR。
- [ ] 首轮 PR 到来后观察一次真实噪声，再决定是否调 `interval` 或分组。

## Comments

- 2026-10-25：已规格化，尚未实施。底稿在本机 `tta-init/assets/templates/github/dependabot.yml`，取用时删掉不存在的 ecosystem 并填真实路径。
- 2026-10-25：已落地（月更、minor/patch 分组、上限 3、无 automerge）。偏差：除 spec 的三条外多了一条 cargo /parse-tests —— parse-tests/ 有自己独立的 Cargo.lock（规格现场证据只数了两份锁文件），owner 可决定保留或裁剪。是否在默认分支启用（开始收 PR）待 owner 答复。
