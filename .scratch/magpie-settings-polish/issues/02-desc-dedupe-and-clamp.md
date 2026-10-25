# 02 — 说明文字去重 + 副标题两行封顶

Status: implemented (2026-10-25) - automated green; owner visual acceptance pending
Type: task
Blocked by: —（与 01 同改 src/main.ts 相邻区域，串行实施更稳）

## Scope

- 删双写：`panelSwitchRow` 去掉 `toggle.title = t(tipKey)` 与 `dataset.i18nTitle`（main.ts:17616-17624），保留行内 desc；`adoptControl` 对 `#ui-font`/`#ui-font-cjk` 显式 `removeAttribute("title")` + `delete dataset.i18nTitle`（main.ts:17888-17905，controls.ts:46/50）；controls.ts 源同步删掉这两个属性（popover 形态同样受益）。
- 保留 title 的正当情形不动：图标按钮（快捷键重置/清除，main.ts:6860/6862）。
- 行高：`.set-row__desc`（styles.css:10263-10269）`overflow-wrap:anywhere` 改 `break-word` + `-webkit-line-clamp:2` 两行封顶；label 侧（10261）同理。

## Acceptance

- [ ] 同一说明不再同时出现在 tooltip 与行内；图标按钮 tooltip 保留。
- [ ] 副标题两行封顶后单行高度显著回落（向 Magpie 的 40px 行高靠拢），无关键限定条件被截断。
- [ ] 4 皮肤 × 亮暗 desc 颜色仍走 `--muted-foreground`，无包覆写冲突（已核实 signal:109、comic-pixel:199 的强制列表不含 `.set-row__desc`）。
- [ ] `pnpm build` 与相关 Node 测试通过。
- [ ] 不提交、不推送。
- 2026-10-25：已实现。panelSwitchRow 的 toggle.title/dataset.i18nTitle 双写删除（描述行承担提示）；#ui-font/#ui-font-cjk 的 title 在 controls.ts 源与面板装配处双清；.set-row__label/.set-row__desc 改 break-word + 两行 clamp。快捷键图标按钮的正当 title 保留。
