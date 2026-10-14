# 参考仓库拉新 + 仓库级定时扫描

**What to build:** 把参考仓库（cc-switch、cockpit-tools 等）拉到最新，产出机制差异笔记；在仓库内加定时工作流定期扫描上游变化。

**Blocked by:** None

**Status:** ready-for-agent

- [ ] 参考仓库更新到最新，记录版本/提交
- [ ] 产出「值得借鉴的机制」差异笔记（按对 Pane 的价值排序）
- [ ] 仓库内新增定时扫描工作流（schedule），失败可见、结果落到文档或 issue
- [ ] 不得把参考仓库内容直接拷进仓库（许可证与体积）

## Comments

- 2026-10-14：由 spec `.scratch/release-082/spec.md` 拆分（owner 批次需求）。
