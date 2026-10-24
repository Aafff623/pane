# 07 — VS Code 灰主题与三皮肤设置适配

Type: task
Status: done (2026-10-24); owner UI acceptance pending
Blocked by: 03

## What was built

- Native light/dark tokens moved from zinc black/white to VS Code Dark+ /
  Light+ neutrals: `#1e1e1e` / `#252526` / `#3c3c3c` / `#d4d4d4` / `#007acc`
  dark, `#f3f3f3` / `#ffffff` / `#d4d4d4` / `#333333` / `#0066b8` light.
- Hardware-coded intermediate colours (toggle thumb, hover, intro background,
  glass surfaces) moved to tokens or VS Code equivalents.
- signal / ink / comic-pixel: `.set-section`, `.set-section__head`,
  `.set-section__card`, `.set-row`, `.set-shell__nav`, `.set-shell__head`
  compatibility rules; active-nav specificity lowered so skins own the accent.
- The `.set-section` wrapper is forced borderless under skins; the head + card
  already paint the frame (comic/ink were showing a double outer frame).
- Comic Pixel pack installed locally from
  `site/public/skins/pane-skin-comic-pixel-1.0.0.zip`; no source/assets were
  lost, the worktree handoff file referenced by the owner was absent.

## Verification

- Live screenshots: native dark/light, signal dark, comic dark/light at the
  default 868 CSS viewport; A–Z rail gutter widened to 30px.
- `pnpm build`, 173 node tests, 15 site tests, full `parse-tests` green.
- Remaining owner checks: ink light/dark settings pass, popover at light mode,
  and the fine typography comparison against VS Code.
