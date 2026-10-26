# 02 — Comic Pixel 对比度与数据字体

Type: task  
Status: done (2026-10-26) — ink tokens, three yellow spots fixed (17:1), data font boundary verified at MV Boli; pack 1.0.2.  
Blocked by: none  
Spec: ../spec.md  
Issue: [#57](https://github.com/Aafff623/pane/issues/57)

## Scope

- 在 `skins/comic-pixel/skin.css` 定义固定 `yellow-ink`、`cyan-ink`、`purple-ink`。
- 修复所有黄色/青色背景上的前景；禁止再用 `--cp-frame`、`--foreground` 当亮底文字色。
- 修复总花费按模型、今日金额、额度总览 Coding Agent 三处。
- 为 provider 名、额度、reset、meta、表单、按钮、告警加固定可读字体。
- Hero 与章节标题继续使用主题 display 字体。

## Acceptance

- [ ] 黄底前景 ≥ 4.5:1，实测建议 17:1；青底 ≥ 4.5:1。
- [ ] 三处截图问题不复现。
- [ ] `uiFont=MV Boli` 时 provider 名仍可读。
- [ ] 用户全局字体配置未改。
- [ ] 原生、Signal、Ink 的工坊样张不被 Comic 规则污染。

## Comments

- 2026-10-25：由 #57 拆分。当前安装包 1.0.0 早于源码修复，必须由 06 重打包。
