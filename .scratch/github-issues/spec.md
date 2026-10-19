# GitHub Issues 批次：#3/#4/#5/#6（github-issues）

**Status:** implemented（4/4 票落地 codex/github-issues 分支，2026-10-18；parse-tests 877:0:13 + tsc/vite/i18n 门禁全绿；未 push、未合并，待 owner Review）
**来源**：GitHub open issues <https://github.com/Aafff623/pane/issues/>（#3、#4、#5、#6，owner 2026-10-18 提交，均已逐条对照本地 v0.8.6 源码核实属实）
**路由计划**：`temp/handoff/20261018-223813-github-issues-matt-plan.md`（本地文件）
**分支**：`codex/github-issues`（主工作树直接改——沿用 analysis-hygiene 批次的 owner 指定先例）

## Problem Statement

四条独立缺陷/债务，批次处理：

1. **#3 数据丢失风险**：`login_accounts.rs::save_to()` 顺序为「写新凭据 → GC 旧凭据 → 最后写账号 JSON」。第三步失败时旧 JSON 仍在但凭据已删，下次加载账号消失。
2. **#4 文档漂移**：`docs/project-facts.md` 停在 0.7.0；`SECURITY.md` 凭据存储描述自相矛盾（vault-first vs 登录账号明文 JSON）；`docs/privacy.md` 更新检查未含官网边缘入口与 GitHub 回退。
3. **#5 花费少计**：`spend.rs::claude()` 对 cc-switch 覆盖过的日期整体剔除本地 JSONL 记录（`retain_days(|day| !cc_days.contains(&day))`，注释自述 by design），混合日（部分请求走代理、部分直连）少算 token 与费用。
4. **#6 测试 flake**：`snapshot.rs::scheduled_backfill_does_not_block_key_cards` 用固定时间窗轮询（10ms 间隔 × 3s deadline ×2 + 服务端 5s gate + 500ms 不阻塞断言），CI 高负载下偶发超时；v0.8.6 发布前 CI 重跑才过。

## Solution

- #3：失败注入测试先行（合成凭据 + 临时目录，不碰真实凭据库），再调 `save_to()` 事务顺序，保证 JSON 写失败时旧账号与 Token 可恢复、无孤儿凭据残留。
- #4：以 v0.8.6 源码为唯一事实源修正三份文档，与 README/AGENTS.md 交叉核对，不引入新治理文件。
- #6：先建复现回路（负载下重复运行），定位先爆的时间窗，以确定性完成信号替代固定轮询，保留「后台任务不阻塞 Key 卡片读取」核心断言。
- #5：查 cc-switch ledger 与本地 JSONL 是否存在请求级共同标识；不存在则实现保守方案——维持按天切，但在卡片上明确标注该日统计来自 cc-switch 单源、可能不完整，不把不完整汇总显示为准确总额。

## User Stories

1. 作为多账号用户，我要账号文件写入失败时已有账号和 Token 不消失，这样一次磁盘/权限故障不导致我丢账号。
2. 作为读文档的用户或 agent，我要 SECURITY.md 对凭据存储只有一种说法且与代码一致，这样我能信任隐私声明。
3. 作为同时用 cc-switch 代理和直连的用户，我要面板不把混合日的少计显示为准确总额，我知道哪天开始数据不完整。
4. 作为 CI 维护者，我要 backfill 测试不依赖固定时长轮询，这样高负载机器不再偶发红。

## Blocking edges

- #4 软阻塞于 #3（SECURITY.md 描述修复后的最终保存顺序）。
- #3 / #5 / #6 互相独立。

## 不做

- #5 未验证请求级去重可行性前不改计费口径；不重构 spend.rs 其它路径。
- 不新增治理文件/文档体系。
- 不 push、不打 tag、不发版——本地 commit 后停下等 owner Review（AGENTS.md 发布授权门）。
