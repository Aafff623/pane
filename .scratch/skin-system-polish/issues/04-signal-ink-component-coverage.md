# 04 — Signal / Ink 同层级组件覆盖

Type: task  
Status: done (2026-10-26) — rail frames, settings nav frames, workshop chrome, states, readable data fonts.  
Blocked by: 02  
Spec: ../spec.md  
Issue: [#57](https://github.com/Aafff623/pane/issues/57)

## Scope

- Signal：补侧栏图标框、设置 nav、工坊、状态与二级页动作。
- Ink：补侧栏按钮、工坊、空/载/错与工坊动作。
- 保留既有设计方向：Signal 用邮戳/票据，Ink 用纸签/笔触。
- 不直接套 Comic Pixel 的点阵图标和硬阴影。
- 检查 `.set-shell__nav`、`.set-row`、`.panel-nav-item`、`.skin-market*`、`.skin-mode-btn` 等当前盲区。

## Acceptance

- [ ] 两套皮肤的侧栏按钮都有主题框体和图标语言。
- [ ] 设置、密钥额度、快捷键的 nav 与内容区一致。
- [ ] 工坊列表、预览、状态、按钮、空索引都有专属样式。
- [ ] 亮暗均通过可读性检查。
- [ ] 不出现 Comic Pixel 样式漏到其他主题。

## Comments

- 2026-10-26: 实施主对话回报见 `docs/project-facts.md` 的 Skin system 节；截图在本地 `temp/skin-shots/`（untracked）。未 commit/push。
