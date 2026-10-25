# 05 — native 截图补齐、集成验证与 owner 验收

Status: ready-for-agent
Type: task
Blocked by: 01, 02, 03, 04

## Scope

- 补 4 张 native 默认主题新截图：常规页/隐私页 × 亮/暗（当前只有 10-24 的 Signal 暗实拍，native 下 A/B/C 是盲飞——尤其 thumb 用错 token、`--row-line` 实际深浅这类问题只有 native 截图能证伪）。
- 集成验证：`pnpm build`；相关 Node 测试（`tests/settings-primitives.test.mjs`、`settings-search.test.mjs` 等）；跨皮肤抽查（native + 至少一个包）亮暗各一遍设置面板。
- 代码审查 + 冗余/simplifier 扫描（AGENTS.md 验收第 5 条）。
- 交付验收清单，交 owner 亲自 UI 验收。

## Acceptance

- [ ] 4 张 native 截图齐；pill 观感、两档线、行高、hover 行为在 native 亮/暗下符合 spec 第 5 节定义的值。
- [ ] 6 个设置行为零回归清单逐项核对（含 jump-animation 延迟保存、spend-tier 联动、语言切换）。
- [ ] 构建与测试实际运行结果如实记录（未运行写未运行）。
- [ ] owner 完成 UI 验收前，不提交、不推送、不打 tag、不上传或发布产物；`main` 合并与发版授权按 AGENTS.md 门禁。
