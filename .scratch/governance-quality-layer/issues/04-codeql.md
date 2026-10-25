# 04 — 代码扫描工作流

Type: task
Status: implemented (2026-10-25, commit 8ab876b) - first run pending push
Blocked by: None
Spec: ../spec.md
Issue: #52 — https://github.com/Aafff623/pane/issues/52

## Scope

- 新增 `.github/workflows/codeql.yml`：`javascript-typescript` 单语言矩阵，push 到 `main`、PR、每周一 `03:00` 调度。
- 权限只给 `contents: read` 与 `security-events: write`，不额外授予写仓库的能力。
- Rust 不进入矩阵：标准 CodeQL 分析不覆盖 Rust。是否引入 `cargo-audit` 或 `cargo-deny` 另开票，不塞进本工作流假装覆盖。
- 不新增自定义查询包，不把扫描结果自动转成 issue。

## Acceptance

- [ ] YAML 可解析，`gh workflow view codeql` 能列出工作流。
- [ ] 首次运行在默认分支或 PR 上完成，结果出现在 Security → Code scanning。
- [ ] 若扫描在引入当天就有历史告警，先记录基线，不为了变绿而排除路径。
- [ ] 失败时保留原始日志位置，不把「工作流跑过」写成「没问题」。

## Comments

- 2026-10-25：已规格化，尚未实施。底稿在本机 `tta-init/assets/templates/github/workflows/codeql.yml`；CodeQL 的 Rust 支持随版本变化，实施时按当前官方说明复核，不凭记忆断言。
- 2026-10-25：workflow 已落地（javascript-typescript、push main/PR/每周一 03:00、权限仅 contents:read + security-events:write，Rust 不入矩阵）。YAML 解析通过；gh workflow view 与 Security 首跑需文件进入远端默认分支后验证。
