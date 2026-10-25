# 02 — 编辑器与缩进约定

Type: task
Status: implemented (2026-10-25, commit 8ab876b) - automated green
Blocked by: None
Spec: ../spec.md
Issue: #52 — https://github.com/Aafff623/pane/issues/52

## Scope

- 新增根目录 `.editorconfig`：`root = true`、UTF-8、LF、末尾空行、去行尾空格、2 空格缩进。
- `[*.rs]` 缩进 4 空格；`[*.{md,markdown}]` 保留行尾空格（Markdown 硬换行）。
- 只声明约定，不引入 Prettier、ESLint 或 rustfmt 配置，不修改任何 npm 依赖与构建脚本。

## Acceptance

- [ ] 文件为根级 `.editorconfig`，不覆盖已有的 `.gitattributes` 规则。
- [ ] 现有 TS、Rust、CSS、Markdown 文件不需要批量重排；如个别文件已不符合，只记录不顺手改。
- [ ] `pnpm build` 与 `cargo +stable-x86_64-pc-windows-gnu check` 结果与改动前一致。

## Comments

- 2026-10-25：已规格化，尚未实施。底稿在本机 `tta-init/assets/templates/project/.editorconfig`。
- 2026-10-25：.editorconfig 已落地并随 8ab876b 提交；与 .gitattributes（仅 skills/pane/** 钉 LF）无冲突；落地后 pnpm build 与真机 cargo build 均绿（比 check 更强的门禁），未发生重排。
