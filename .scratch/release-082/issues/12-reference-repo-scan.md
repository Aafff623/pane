# 参考仓库拉新 + 仓库级定时扫描

**What to build:** 把参考仓库（cc-switch、cockpit-tools 等）拉到最新，产出机制差异笔记；在仓库内加定时工作流定期扫描上游变化。

**Blocked by:** None

**Status:** in-review（实现已核实，验收边界见Comments）

- [ ] 参考仓库更新到最新，记录版本/提交
- [ ] 产出「值得借鉴的机制」差异笔记（按对 Pane 的价值排序）
- [ ] 仓库内新增定时扫描工作流（schedule），失败可见、结果落到文档或 issue
- [ ] 不得把参考仓库内容直接拷进仓库（许可证与体积）

## Comments

- 2026-10-14：由 spec `.scratch/release-082/spec.md` 拆分（owner 批次需求）。

- 2026-10-14（Codex独立接续）：核实既有docs/reference-repos.md与.github/workflows/upstream-scan.yml：4个参考仓库的commit/release基线已落文档，每周一03:00 UTC定时扫描、报告artifact、漂移失败可见；定时路径不写仓库。node --check .github/scripts/upstream-scan.mjs通过。本轮未额外fetch/改上游工作树，未触发远程Actions。
