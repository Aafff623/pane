# 03 — Comic Pixel 图标包与材质层级

Type: task  
Status: done (2026-10-26) — 26 pixel masks in the pack, four material tiers, workshop chrome + card tools.  
Blocked by: 02  
Spec: ../spec.md  
Issue: [#57](https://github.com/Aafff623/pane/issues/57)

## Inputs

26 个原创 SVG：

```text
C:\Users\Lenovo\.codex\visualizations\2026\10\25\01a172dc-24f5-7301-a38f-8b44737c1ba0\skin-pixel-20261025\prototype\assets\icons\pixel
```

视觉原型：

```text
C:\Users\Lenovo\.codex\visualizations\2026\10\25\01a172dc-24f5-7301-a38f-8b44737c1ba0\skin-pixel-20261025\prototype\index.html
```

## Scope

- 复制资产到 `skins/comic-pixel/assets/icons/pixel/`。
- 用 pack-local CSS mask 覆盖侧栏、设置分类、工坊、状态和常用工具按钮。
- 不删除原生 SVG；Signal / Ink 仍需要。
- 把材质分成 Hero / 数据卡 / 控件 / 状态四类，减少重复 3px 粗框。
- Provider 卡使用轻边线、状态侧条或印章，不再复制 Hero 硬阴影。

## Acceptance

- [ ] 侧栏四个入口有像素图标，不只是按钮换壳。
- [ ] 设置常规/密钥额度/快捷键/账号均有图标卡。
- [ ] 工坊、折叠、分享、关闭、删除、空态、错误态无原生图标混搭。
- [ ] 380/450px 下图标清晰，按钮命中区不小于 30px。
- [ ] 分享 PNG 中 mask 图标可见；若 foreignObject 不稳，切 inline SVG。
- [ ] 四皮肤样张互不污染。

## Comments

- 2026-10-26: 实施主对话回报见 `docs/project-facts.md` 的 Skin system 节；截图在本地 `temp/skin-shots/`（untracked）。未 commit/push。
