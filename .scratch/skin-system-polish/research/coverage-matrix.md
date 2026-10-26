# Skin coverage matrix

Evidence date: 2026-10-25. Static selector presence is only a coverage signal, not visual acceptance.

| Surface | Signal now | Ink now | Comic Pixel now | Target |
|---|---|---|---|---|
| Outer sidebar buttons | Generic `.sidebar` surface; no `.sidebar-buttons` icon system | Generic `.sidebar`; no `.sidebar-buttons` icon system | Button shells themed; icons remain native Phosphor | Every rail button has theme-specific frame, icon, active/hover/disabled state |
| Theme / refresh / workshop / settings icons | Native line icons | Native line icons | Native line icons | Comic: 16px pixel masks; Signal: postage/ticket frames; Ink: paper-slip/brush frames |
| Root active pills | Explicit foreground/background pairs | Brush underline and wash | Roles exist, but yellow active states use dark-theme white as foreground | One semantic pair per role; never borrow a frame token as text colour |
| Spend card | Rounded paper display slab | Paper landscape art, ink graph | Hero frame + ray burst + hard shadow | Keep unique hero treatment |
| Quota card | Rounded card with soft shadow | Paper sheet with landscape wash | Same heavy frame family as hero | Make data card quieter than hero; reserve strongest frame for hero |
| Provider cards | Theme-specific rounded cards | Transparent paper rows | Same 3px frame + hard shadow pattern repeated | Distinct ticket/card grammar; readable names; status stamp |
| Settings shell | `.set-shell__nav`, `.set-section__card`, `.set-row` present | Same selectors present | Present in source; installed pack predates them | Same shell grammar across themes, different materials |
| Settings rail icons | Native icons in theme surface | Native icons in theme surface | Native icons on pixel buttons | Theme-specific icon card treatment for General / Keys / Shortcuts / Account |
| Settings rows / controls | Covered | Covered | Covered in source | Verify nested selectors, keyboard focus, disabled and error states |
| Skin workshop chrome | Partial `.skin-market` rule | No skin-market-specific rule found | Market detail/actions partly covered | Complete gallery, preview, state badge, action and error styling in all themes |
| Dialogs | Covered via grouped selectors | Covered | Covered | Add long-content, destructive and narrow-window states to acceptance |
| Empty / loading / error | Partial state rules | Partial paper treatment | Bespoke placeholder rules | Theme-specific but legible; character art only in state surfaces |
| Focus / reduced motion | Present | Present | Present | Keep reduced-motion path for new GSAP transitions |
| Share PNG | Existing contract | Existing brush/mask checks | Needs new icon/font/transition audit | Verify after assets are added |

## Current selector-presence examples

These are mechanical counts from the pack CSS, useful only to find blind spots:

| Selector fragment | Signal | Ink | Comic Pixel |
|---|---:|---:|---:|
| `.sidebar-buttons` | 0 | 0 | 1 |
| `.panel-nav-item` | 4 | 6 | 6 |
| `.set-shell__nav` | 1 | 1 | 1 |
| `.set-row` | 1 | 1 | 1 |
| `.skin-market` | 1 | 0 | 1 |
| `.skin-mode-btn` | 0 | 0 | 2 |
| `.provider-name` | 1 | 3 | 3 |
| `.overview-item-name` | 0 | 1 | 1 |
| `.ovbar-pct` | 4 | 1 | 5 |
| `.toggle` | 0 | 2 | 7 |
| `.card-account-tab` | 0 | 0 | 7 |

## Non-goals

- Do not paint every card with the same 3px border and hard shadow.
- Do not use decorative fonts for provider names, quota values, reset times, form labels or errors.
- Do not replace real charts, logos, numbers or data geometry with decorative art.
- Do not re-use AIGC News image assets; create original masks and CSS materials.
