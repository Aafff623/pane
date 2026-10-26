# Spec: 0.11.1 close-out（21 个开放 issue 全量收口）

**Status:** in progress · **Branch:** `codex/0.11.1-closeout` · **Started:** 2026-10-26
**Baseline:** local checkpoint `f0c0756`（0.11.0 身份/遥测/UI/站点收尾）
**Remaining issues:** #27 #29 #31 #37 #38 #42 #43 #47 #48 #57 #58 #59 #63 #65 #79 #82 #83 #87 #88 #104 #105

## 目标

把 21 个开放 issue 按 implement-spec 的任务图做完：每张 ticket 独立可验收，
在独立 worktree 上 TDD，合并进本集成分支，最后统一 code-review。

**硬约束**

1. 不 push、不打 tag、不发布、不部署线上，除非 owner 在发布门单独授权
   （本次 Gate 0 已授权“最终验收后”执行，具体动作前仍要确认）。
2. UI 验收归 owner；自动化绿不等于视觉通过。
3. 与上游作者彻底脱钩：不引入 `ItsJazii` 身份、`com.jazii.pane`、
   `Pane.Pane`、对方 PostHog、对方更新端点或发布渠道。历史 CHANGELOG
   引用只作记录，不算活跃依赖。
4. 遥测保持为零；不新增 analytics/crash reporter。
5. 所有改动遵循 `AGENTS.md` 的验证与 Git 规则；`temp/` 不入库。

## Gate 0 已确认（2026-10-26）

- 0.11.0 先做本地 checkpoint，不发布；owner 验收后再发。
- #63 最小多账号接线；#65 八票全做。
- #104 十项全做（3/4/5/6/7/8/9/10/17/18）；19/20 远期不做。
- #59 先做反事实重放 research spike，再决定是否实现 01。
- #31 只做 WorkBuddy CN 用量面板；AI 档不做（无账号）。
- #83 采用“整理”档，不升级/重塑站点骨架；对比页作为决策记录。
- #38/#47：owner 登录后提供真实响应/控制台数字。
- #57：测试通过后授权部署皮肤索引。
- #88：owner 已买断 oil-ui Pro，许可按自有授权记录；不复制上游第三方素材。
- #105：最终验收后授权 push/tag/release 与 winget-pkgs PR 准备。

## Lane 与 ticket 图

| Ticket | 覆盖 issue | 依赖 | 并行 lane |
|---|---|---|---|
| 01 upstream-severance | #105 审计部分 | — | 已完成 |
| 02 correctness | #48-3、#37-03 | 01 | Rust/UI |
| 03 site-contracts | #79、#82 | 01 | site |
| 04 spend-architecture | #42、#43 | 02 | Rust |
| 05 misc-104 | #104 十项 | 02、04、07 | Rust/UI |
| 06 refresh-identity | #63 | 01 | Rust |
| 07 codexbar-arch | #65 | 06 | Rust |
| 08 workbuddy | #31、#29、#27 | 06 | Rust/UI |
| 09 skins-publish | #57、#58-05、#88 | 01 | site/owner |
| 10 site-narrative | #83、#87 | 03 | site |
| 11 counterfactual-spike | #59 | 01 | research |
| 12 provider-calibration | #38、#47 | owner 登录 | Rust |
| 13 release-winget | #105 | 全部 | release |
| 14 review-verify | 全部 | 全部 | quality |

**Frontier 规则**：01 已完成；02/03/06/09/11 可立即并行；
04 在 02 后；07 在 06 后；05 在 02/04/07 后；08 在 06 后；
10 在 03 后；12 等 owner 登录；13/14 最后。

## 验证门

- `pnpm build` + `cargo check`。
- `parse-tests` 串行去代理，0 失败。
- `node --test` + `site/site.test.mjs` + demo 重建。
- 每张 ticket 的专项测试；UI 票附 owner 可复现步骤。
- 最终 `code-review` skill；所有发现修复后再交付。
- 发布前 owner 三皮肤/明暗/380–1440 宽度验收。
