# 05 — 集成校验、文档漂移与 owner 确认

Type: task
Status: implemented (2026-10-25) - local checks green; remote-side items and owner confirmations pending
Blocked by: 01, 02, 03, 04
Spec: ../spec.md
Issue: #52 — https://github.com/Aafff623/pane/issues/52

## Scope

- 四项文件落地后统一复核：YAML 解析、`gh workflow view`、模板可见性、`.editorconfig` 不引发重排。
- 修正 `docs/project-facts.md` 的版本漂移：正文 `Version 0.8.9` → `0.9.1`；同时检查同一段的说明是否仍与实际一致。
- 在 `AGENTS.md` 的验证章节补一行「质量层文件在哪、怎么复核」，避免下个会话重新推断。
- 汇总结论：哪些已生效、哪些只落文件未生效（Dependabot、CodeQL 首跑），逐条区分。

## Acceptance

- [ ] `pnpm build` 与 `cargo +stable-x86_64-pc-windows-gnu check` 结果与实施前一致。
- [ ] `.github/` 四个新增文件与 `.editorconfig` 都能在远端读到（未推送时标注为本地未推送）。
- [ ] `docs/project-facts.md` 不再出现 `0.8.9` 的当前版本表述。
- [ ] 交付说明区分「已修改 / 已测试 / 已验证 / 未验证」，不把静态检查当作运行证明。
- [ ] owner 完成确认与验收；未确认前不提交、不推送、不发布。

## Comments

- 2026-10-25：已规格化，依赖 01–04 完成。工作树当前有主对话的大量未提交改动，执行前重新核对边界，只加文件不改他人改动。
- 2026-10-25：版本漂移 0.8.9→0.9.1 已在 8ab876b 修正（同段其余说明核对仍准确）；AGENTS.md 验证章节已补质量层条目（第 4 条，原 4-7 顺延为 5-8）；YAML 全部可解析；pnpm build 与真机 cargo build 与改动前一致。远端项（gh workflow view、模板可见性、Dependabot 生效、CodeQL 首跑）与两项 owner 确认（Dependabot 启用、验收）未完成。
