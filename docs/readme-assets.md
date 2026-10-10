# README assets

## Published files

| File | Role | Update rule |
| --- | --- | --- |
| `src/assets/pane-logo.png` | Canonical Pane product logo used in the README header | Keep as the identity mark; update only when the product branding changes. |
| `attachments/overview-dark.png` | 额度总览面板截图（功能特性 1） | 从当前官网演示前端截图，使用示例数据。 |
| `attachments/spend-dark.png` | 花费详情面板截图（功能特性 2） | 从当前官网演示前端截图，使用示例数据。 |
| `attachments/heatmap-dark.png` | 花费热力图截图（功能特性 2） | 从当前官网演示前端截图，使用示例数据。 |

## Static captures

The three `attachments/*.png` files were captured on 2026-10-10 from the current
frontend rebuilt with `node site/scripts/build-demo.mjs` and served by the local
Worker. They show the quota overview, model spend card and daily-use heatmap.
All values are demo data; these captures do not read real accounts or credentials.

Use static images for now. When GIF recordings are available, update the image
paths in both READMEs and this table together, preserving the centered image slots.

## Bilingual maintenance

`README.md` (zh-CN) and `README_EN.md` (en) are a **mirrored pair**: same 12 anchor ids
(`intro, demo, highlights, features, providers, download, i18n, build, structure, privacy,
contributing, license`), same section order, same tables and the same 66-provider list.

When you change one, change the other in the same commit. The numbers that must stay in
sync across both files: the test-harness link, the provider count (66), the default-enabled
families (11) and the refresh interval (5 minutes).

## Hardcoded numbers that drift

| Where | Value | Refresh rule |
| --- | --- | --- |
| Tests badge | `parse-tests` | Links to the harness without a hardcoded pass count. Report executed test counts with their verification date. |
| Providers badge + `<details>` list | `66` + full name list | The list is generated from `src/providerCatalog.ts`, not hand-maintained — regenerate on catalog changes. |

## Layout

The README uses a centered header, an emoji-anchored section list, and numbered
feature sections (`### N. <emoji> <title>`) that end with a centered screenshot
block (`<div align="center">` + `<img>`, `<br/><br/>` between multiple images).

Screenshots are copied into `attachments/` instead of being referenced from
`site/public/img/` so the README stays self-contained. The site keeps dark/light
pairs; the README uses dark captures for a consistent presentation. The app's default appearance follows the system.
Swap in the light variants — or a `<picture>` element — if a theme-adaptive
version is wanted.

Product behavior, commands, provider detail, privacy boundaries, and design
decisions stay in searchable Markdown and linked documents so the page does not
become stale when the dashboard changes.
