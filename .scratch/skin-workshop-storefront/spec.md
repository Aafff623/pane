# Spec · Skin Workshop Storefront (2026-10-27)

**Status**: implemented; **owner accepted 2026-10-27**; shipped in `v0.11.1`. **Issue**: https://github.com/Aafff623/pane/issues/107

## Implementation result (2026-10-27)

- New Settings page **通用 → 皮肤工坊** (`#st-skins`); the old drawer market and its entry were removed. The popover palette button now opens the panel on this page through `panelIntent.page = "skins"`.
- Gallery: native + installed + index entries; masonry (2 wide columns) and grid, remembered in `localStorage` (`pane.skins.view`); search, count, empty state, index-loading and offline notes; real component previews at natural width inside phone-shaped frames.
- Detail: crumbs/back, hero, factual metadata (version, status, source, package size only when the index provides one), horizontal real-component screenshot rail, zoom with Esc and focus return, day/night/both, download → use → restore-native and remove states driven by the existing commands.
- No fabricated ratings, downloads, authors or metrics; remote skins show a factual “download to preview” placeholder instead of concept art.
- A saved skin that is missing locally stays Selected/Not installed; native is no longer reported as applied, and the gallery explains the state. Pack-registry read failures keep the last good registry and surface a message.
- Files: `src/main.ts` (skin shop + settings page + intent), `src/skins.css` (storefront styles), `src/skins.ts` (index settled flag + cross-window pack nudge), `src/i18n.ts` (three locales).

## Problem
Existing market is housed in small drawer; an enlarged nested UI sample and multiple switchers produce a cramped, multi-scroll settings-within-settings experience. User wants a recognizable theme **store** hosted by the larger settings window, not skin rewrites.

## Scope
1. Move primary navigation into Settings → General → Appearance / 皮肤工坊. Legacy entry deep-links to it.
2. Build product gallery with realistic portrait panel previews, visual identity, human-readable names, version and installed/current state. User switches between masonry and uniform grid; preserve choice.
3. Detail page uses hero identity, sparse metadata, horizontal portrait screenshot gallery with zoom, description, version and canonical action.
4. Wire existing download→use, use→native, failure and progress states. Clearly distinguish hovered/previewed from applied, with real backend persistence feedback.
5. Use Pane existing settings shell and design tokens, not a separate app theme. Previews must come from real skin components with fixed harmless demo data; no authentic personal account data.

## Object & data contracts
`SkinProduct`: id; name; version; installedVersion?; visualPreview[]; author?; tags?; minAppVersion?; availability; active?; description?. Use actual API and manifest facts, not placeholder authors or unverified ratings. `native` is a built-in first-class option without download. Only official entries indexed and verified in current Pane distribution appear.

## Layout
Settings baseline from #49: 980×700, min 860×620, sidebar 200px. Gallery: masonry with staggered tile heights and fixed-width columns or uniform grid, portrait real-panel image centered in each tile; each tile shows title, short subtitle/optional metadata, status. Detail: editorial Hero → compact metadata rail → horizontal screenshot strip → description. One vertical scroll surface in settings; horizontal scroll only gallery. On small widths, card count and text reflow without clipping.

## Interaction states
- Browse / search / no results; gallery/grid view remembered.
- Detail and back must retain list scroll/search/filter and keyboard focus.
- Not downloaded → downloading (progress, cancel only if real command supports) → downloaded → use → active; failure shows retry; native restore always possible.
- No fabricated network success; no fake downloads or false metrics. Demo-only state in standalone HTML is intentionally simulated and marked in documentation.
- Day/night screenshot toggle only when both variants actually exist; otherwise omit or disable; zoom closes via Esc and focus returns to trigger.

## Exclusions
No changes to `skins/*/skin.css`, no replacement of all settings shell, no duplicated #57 fixes, no new skin pack format, no payment, ratings/reviews, accounts or uploads, no API proxy.

## Acceptance
- Settings → General → Appearance opens the shop without nested drawers and legacy entry routes correctly.
- Both gallery modes visibly differ and persist; item identity/status clear; missing assets have accessible fallback.
- Detail hero, factual metadata, horizontally swipeable portrait screenshots, zoom, main action; no nested vertical scrollbar.
- Download/apply/restore actions represent real backend outcomes; offline, failures, stale index, and retry covered.
- Preview parity with true skin rendering, including fonts and actual fixed-data UI; no secrets.
- Settings 980×700 / 860×620, floating view 380/450 references; desktop keyboard; reduced motion and contrast sanity; owner visual approval.

## Dependency plan
01 Settings route → 02 Gallery → 03 Detail & action → 04 Integration/QA. 02 and 03 can parallelize after route contracts settled; do not edit shared stylesheet simultaneously.
