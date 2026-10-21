# README assets

## Published files

All shots are theme-adaptive pairs (GitHub picks dark or light per the reader's
theme via `<picture>`), captured from the **real frontend demo bundle** with
fixed demo data — never a real account. The current set was refreshed
**2026-10-21 for v0.8.8**: a wide two-window hero, current screenshots, and the
71-provider count.

| File pair | Role | Window |
| --- | --- | --- |
| `attachments/hero-{dark,light}.jpg` | Wide header composition: floating panel + settings window, captured 2026-10-21 (v0.8.8) | 1200×680 CSS @2x |
| `attachments/overview-{dark,light}.png` | 额度和花费 subsection left cell; quota overview | 450×640 CSS @2x |
| `attachments/spend-tool-dollar-{dark,light}.png` | 额度和花费 subsection right cell; spend by tool / dollars | 450×640 CSS @2x |
| `attachments/how-{dark,light}.png` | 「它如何工作」架构图（HTML/CSS fixture 渲染，非 UI 截图） | 1160×430 @2x |
| `attachments/customize-{dark,light}.png` | 来源接入 subsection | 450×640 CSS @2x |
| `attachments/skins-{dark,light}.png` | 皮肤工坊 left cell（云端邮局 / Cloud Post Office） | 450×640 CSS @2x |
| `attachments/skins-ink-{dark,light}.png` | 皮肤工坊 right cell（山水笺 / Ink & Paper） | 450×640 CSS @2x |
| `attachments/settings-keys-{dark,light}.png` | 密钥 subsection（设置 · 密钥与额度页） | 975×677 CSS @2x |
| `attachments/expiring-{dark,light}.png` | 还有 subsection（临期清单） | 450×640 CSS @2x |
| `src/assets/pane-logo.png` | README 头部产品 logo（链接到官网） | 更换品牌时才动 |

Spare shots (settings-shortcuts, spend-detail 450, overview-month) stay out of
the tree; `settings-general` is used only as the hero's settings-window source.
Regenerate on demand.

## Structure (magpie skeleton, since 2026-10-14; hero refreshed 2026-10-21)

`README.md` / `README_EN.md` mirror yetone/magpie's section skeleton exactly,
with Pane content: centered header (linked logo → tagline → two-row badges →
bold nav links → language switch → **wide two-window hero**) → **Contents** →
**Is Pane for you?** (value bullets + five-console comparison table) →
**Quick start** (3 numbered steps + from-source `<details>`) → **Supported
sources** (auth-shape table + `<details>` full list) → **How it fits together**
(diagram picture + prose) → **What it does** (8 subsections: 额度和花费 / 来源接入 /
多账号池 / 花费 / 皮肤工坊 / 密钥 / Agent 自诊 / 还有; quota+spend and both skins
use 50/50 tables, secondary subsections fold into `<details>`; shortcuts table
inside `<details>`) → **Documentation** link table → **Privacy** dense paragraph
→ **Community** (contributing prose + theme-adaptive star-history chart) →
**License**.

## Capture pipeline

UI shots (regenerate after the version bump, so footers match the release):

```powershell
node site/scripts/build-demo.mjs          # demo bundle from the working tree
node temp/scripts/readme-shots.mjs temp/readme-0.8.8 --root site/public --port 8792
# crop the popover pairs to 450x640 CSS (900x1280 @2x); heatmap 780x1200
```

Diagram shots (edit `temp/readme-diagram/how-it-fits.html`, then):

```powershell
node temp/scripts/readme-diagram-shot.mjs
```

Rules baked into the harnesses:

- Floating-window views are captured at the **real window size** (450×640 CSS
  per `src-tauri/tauri.conf.json`; heatmap 780×1200; settings 975×677). The
  window scrolls in real life — a fold cut is authentic; the main board keeps
  every category's cards in the DOM (~5000px), so full-content capture is
  impossible anyway.
- Settings-window clicks MUST be scoped to `#panel-shell` — the popover's
  hidden settings overlay lives in the same document and swallows unscoped
  `[data-settings-tab]` selectors.
- `document.getAnimations().forEach(a => a.finish())` freezes entrance
  animations deterministically instead of sleeping longer.
- If the demo hangs on **"Loading Pane…"**, the frontend is calling a Tauri
  window method missing from `site/demo/bridge.ts`. Add a no-op stub; v0.8.8
  needed `setAlwaysOnTop`, `outerPosition`, `center`, `startDragging`,
  `startResizeDragging`, `innerSize`, `scaleFactor`, `setSize`, and the
  `on*` listeners. Also set `configNoticeVersion` to the demo version so the
  current-config notice does not cover the first screenshot.

## Bilingual maintenance

`README.md` (zh-CN) and `README_EN.md` (en) are a **mirrored pair**: same
section skeleton, same anchor ids (`intro, quickstart, providers, how,
features, docs, privacy, community, license`), same tables, same image set
and the same 71-provider list. When you change one, change the other in the
same commit. Verify with `node temp/scripts/check-readme.mjs` (image refs
exist, anchor mirror, badge == list count, tag balance).

## Hardcoded numbers that drift

| Where | Value | Refresh rule |
| --- | --- | --- |
| Providers badge + `<details>` list | `71` + full name list | Generated from `src/providerCatalog.ts` (`displayName` per entry) — regenerate on catalog changes. |
| Default-enabled families claim | "Claude、Codex、Cursor、Grok、Z.ai、MiniMax、Antigravity、Kimi 等" | Derived from `DEFAULT_DISABLED_PROVIDERS` in `src-tauri/src/lib.rs`; the exact count is deliberately not stated. |
| CI / Downloads badges | live shields.io endpoints | No hardcoded counts: CI reads `ci.yml`, downloads reads GitHub. |
| Footer version in screenshots | must equal the release version | Always re-run the capture pipeline after a version bump. |
