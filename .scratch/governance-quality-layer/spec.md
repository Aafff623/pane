# 质量层补齐：issue 表单、编辑器约定、依赖自动化与代码扫描

Status: implemented (2026-10-25) - owner confirmations open (Dependabot enablement, acceptance)
Type: governance
Date: 2026-10-25
Issue: #52 — https://github.com/Aafff623/pane/issues/52
Owner decisions: 不引入 Repo Wiki；只补四项通用质量层；不开启 automerge；Rust 不使用 CodeQL。

## 1. 需求

Pane 的 TTA + Matt 骨架已经完整——`CLAUDE.md` 有 `## Agent skills` 三节、`docs/agents/` 三件齐全、`.scratch/` 承载全部任务状态——因此本次不重新 bootstrap，也不重跑 `/setup-matt-pocock-skills`。缺的是骨架之外的质量层，四项都会产生真实后果：

1. 20 个 open issue 全是自由文本，外部报障没有固定的版本、系统与复现字段。
2. 仓库没有任何 lint/format 约定：`devDependencies` 只有 `@tauri-apps/cli`、`typescript`、`vite`，`scripts` 只有 `dev`/`test`/`build`/`preview`/`tauri`；TS 与 Rust 混排靠人工对齐。
3. `pnpm-lock.yaml` 与 `src-tauri/Cargo.lock` 两套锁文件没有依赖更新自动化，安全补丁靠手动发现。
4. Pane 处理系统凭据库、明文 `onenewapi.json` 与多家厂商 token，没有任何代码扫描。

Repo Wiki 的结论是**不引入**：代码定位由 codegraph 覆盖（Codex、ZCode、Qoder 三端共用同一份索引），已验证事实与硬约束由 `docs/project-facts.md` 覆盖，模块地图由 `AGENTS.md` 提供；自动生成的快照会成为第四份会过期的真相。

## 2. 现场与证据（2026-10-25 核对）

- 分支 `codex/rtk-orcaterm-handoff` 与 `main` 同停在 `v0.9.1`，工作树有主对话的大量未提交改动；本批只新增文件，不改 `src/`、`src-tauri/`、`site/`。
- `.github/` 只有 `ci.yml`、`release.yml`、`site-deploy.yml`、`upstream-scan.yml`、`winget.yml` 五条 workflow 和 `scripts/upstream-scan.mjs`；没有 `ISSUE_TEMPLATE/`、`pull_request_template.md`、`dependabot.yml`、CodeQL。
- GitHub 标签仍是默认集（`bug`、`enhancement`、`documentation` 等）。跟踪器是本地 Markdown，因此不需要分诊标签，也不补 `needs-triage` 一类。
- `package.json` 没有 `packageManager` 与 `engines` 字段，仓库没有 `.nvmrc`；本机用 corepack 管 pnpm。
- `docs/project-facts.md` 正文仍写 `Version 0.8.9`，实际版本是 `0.9.1`（`package.json` 与 `src-tauri/tauri.conf.json` 一致）。

实施材料已并入全局 `tta-init`（本机 `C:\Users\Lenovo\.agents\skills\tta-init\`）：`references/quality-gates.md` 是生态与门禁的对照表和采用顺序，`assets/templates/` 下有 `.editorconfig`、`dependabot.yml`、`codeql.yml`、issue 与 PR 模板。该路径在本机之外不存在，复制进仓库时以本规格为准。

## 3. 目标文件

| 文件 | 作用 |
| --- | --- |
| `.github/ISSUE_TEMPLATE/bug_report.md`、`feature_request.md`、`config.yml` | 固定版本、系统、复现步骤与证据字段 |
| `.editorconfig` | UTF-8、LF、TS 2 空格、Rust 4 空格、Markdown 保留行尾空格 |
| `.github/dependabot.yml` | npm + cargo + github-actions；月更；minor/patch 分组；PR 上限 3 |
| `.github/workflows/codeql.yml` | `javascript-typescript`；push、PR、每周一 |

## 4. 边界

- 不新增 npm 依赖，不动构建脚本；`.editorconfig` 只声明约定，不引入格式化器。
- 不开启 automerge；Dependabot 是否真的启用由 owner 决定，开启后会开始接收 PR。
- Rust 不进入 CodeQL：标准分析不覆盖；依赖侧留给 `cargo-audit`，是否引入另开票。
- 不在未获授权前提交、推送、打 tag 或发布；不覆盖主对话的未提交改动。

## 5. 验收

- [ ] 三个 issue 模板在 GitHub 新建 issue 时可见，字段能覆盖版本/系统/复现。
- [ ] `.editorconfig` 与现有文件不冲突，`pnpm build` 与 `cargo check` 结果不变。
- [ ] `gh workflow view codeql` 能列出工作流，YAML 可解析。
- [ ] `docs/project-facts.md` 版本口径修正为 `0.9.1`。
- [ ] owner 明确答复 Dependabot 是否启用；未答复前只提交配置，不宣称已生效。

## 6. 票与依赖

1. `issues/01-issue-templates.md` — issue 表单与 `config.yml`。
2. `issues/02-editorconfig.md` — 编辑器与缩进约定。
3. `issues/03-dependabot.md` — 依赖更新自动化与启用决策。
4. `issues/04-codeql.md` — 代码扫描工作流。
5. `issues/05-verification.md` — 集成校验、文档漂移与 owner 确认。

依赖：01、02、03、04 → 05。01–04 之间无先后耦合。
