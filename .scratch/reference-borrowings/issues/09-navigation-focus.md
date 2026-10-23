# 09 — Scale-safe centered navigation and target highlight

Status: implemented; owner UI acceptance pending

2026-10-23 owner follow-up: invisible frame reproduced as neutral theme `--accent` (#27272a in dark mode) rather than semantic highlight color. Explicit overview blue #3b82f6 and settings/auth yellow #eab308 now drive outline and inset ring; provider/auth headings also get a frame for oversized sections. Regression added before fix; final Node suite 125 passed and corepack pnpm build passed. Real owner appearance acceptance remains pending.
Priority: P1
Blocked by: None

## Owner report

Customize → auth/API key/provider settings jumps land at the top without a useful highlight. Overview → provider-card jumps misplace the target after window/font zoom changes. All credential jumps must center the target and show a clear highlight after rendering/scrolling. Preserve existing navigation and authentication ownership.

## Regression first

Exercise existing overview scroll code with fractional CSS zoom, then unify settings/auth entry focus after async rendering. Check reveal of folded groups, reduced-motion behavior, target at scroll limits, and highlight surviving async DOM replacement.

## Implementation and evidence (2026-10-23)

- Existing overview calculation reproduced wrong layout offsets: 5/6 zoom cases failed before the fix. Convert visual rect deltas/heights back to layout coordinates, account for the scroller border, clamp scroll limits, and keep the leading portion of oversized cards visible.
- Overview, auth-center family, provider credentials and MCP/API-key intent routes now share centered navigation. Show a 3-second outline after arrival; restore it across DOM replacement. Honor reduced motion and cancel previous navigation on a new jump/view switch.
- Coalesce concurrent auth-center loads: entering Auth and focusing a family share one pending read instead of rendering twice and erasing the cue.
- Settings scroll viewport gets leading/trailing pseudo-element space while focused, so first/last entries can actually center; providers get trailing space for the final card. Remove settings space on a settings/auth view switch. This does not enlarge the native window or shift the menu horizontally.
- Node suite: `node --test tests/*.test.mjs`, **123 passed**, including 13 navigation regressions (6 zooms, scroll bounds/tall card, auth in-flight sharing, highlight restoration/expiry, reduced motion, cancellation when leaving during a pending auth load).
- Ego browser fixture imports actual navigation functions and full CSS: final **54 combinations passed**, max settings center error **0.282 px**, max final-provider center error **0.319 px**. Six zooms × three target types × first/middle/last positions; DOM replacement preserves the outline. Evidence scripts: `temp/scripts/build-navigation-fixture.mjs`, `temp/scripts/navigation-browser-05.mjs`. The fixture's first provider lacks the real overview preceding it, so that fixture case checks visibility/highlight rather than exact centering.
- `corepack pnpm build` passed; GNU debug build passed; edited-source whitespace check passed. Ordinary `pnpm` points to a missing global module; corepack works without changing global configuration. Existing bundle-size/linker warnings remain.
- Final full parsing/provider harness: `cargo +stable-x86_64-pc-windows-gnu test --manifest-path parse-tests/Cargo.toml -- --test-threads=1`, **1033 passed / 0 failed / 15 ignored**, 434.94 seconds; doc-tests passed. Log: `temp/logs/navigation-rust-all.log`. This is later than issue 07/08's earlier suite evidence.
- Review/simplification checked coordinate units, borders, scroll bounds, async load ordering, target lifetime and cleanup. Removed superseded highlight styles/animation; retained folded-group reveal flow. No new dependency.
- Browser evidence is a layout fixture, not the running WebView2 interaction. Native app restarted with `temp/scripts/restart-pane-dev.ps1`; owner visual acceptance remains pending. No commit/push/tag/release.

## Owner acceptance

1. At 80%, 117%, 175% and small/large fonts, click MiniMax (and a long provider card) in overview: intended heading remains visible and its outline lasts about 3 seconds after arrival.
2. From Customize, jump to ChatGPT add-account, a web-session sign-in and an API-key entry: target appears near the middle with an outline, including the first/last settings family.
3. Repeat while usage refreshes; the outline survives a data tick. Switch settings pages during loading: confirm there is no later unexpected jump.
4. Also accept issue 07's continuous slider/font menu/centering and issue 08's three ChatGPT tabs; previous automated evidence does not replace owner acceptance.

## 2026-10-23 close-out review

- `focusNavigationTarget` added the 50vh centering spacer to the scroller and never removed it: after one overview jump the floating window's `#providers` list kept a blank half-viewport until reload, because only `setPanelView` cleared it for the panel. The cleanup now drops the spacer together with the highlight, and the explicit `setPanelView` removal is gone.
- New Node regression asserts the spacer is present while centering and absent after expiry; the existing fixture's scroller stub gained the missing `classList.remove`. Node suite 127 passed.
