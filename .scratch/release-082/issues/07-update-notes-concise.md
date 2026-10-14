# 更新完成后的说明弹窗精简

**What to build:** 复用既有 showChangelogDialog，把「本次更新内容」压成要点级（标题 + 3~5 条短句），不再堆长段落。

**Blocked by:** None

**Status:** in-review（实现已核实，验收边界见Comments）

- [ ] 弹窗只显示要点（每条一行，尽量 ≤ 30 字）
- [ ] 标题体现本版主题；三语一致
- [ ] 内容仍来自 CHANGELOG 对应版本节，不新造文案源

## Comments

- 2026-10-14：由 spec `.scratch/release-082/spec.md` 拆分（owner 批次需求）。

- 2026-10-14（Codex独立接续）：接续发现原实现按每个小节限4条，且设置里查看日志也被裁成单版本。已改为自动更新弹窗仅最新版本、全局最多4条要点；设置里的更新日志恢复完整版本与全文，均直接读取CHANGELOG原文。没有另造文案源。三语标题沿用既有翻译。原生视觉终验待owner。
