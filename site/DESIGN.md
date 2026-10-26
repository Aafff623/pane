# Pane Site — Design Contract

> Status: adopted 2026-10-26 (#83). This file is the acceptance baseline for
> every site UI ticket (#84–#89). A change that invents a new token, type
> scale or page skeleton is out of contract unless this file is updated first.

## 1. North star

**One instrument, two surfaces.** The app is a quiet local instrument; the
site is its blueprint. Data and states are the subject, the Pane mascot and
ink marks are the only expressive layer, and motion is feedback — never a
show. The site and the in-app usage topology (#60) share one palette, one
grid rhythm and one GSAP clock.

## 2. Tokens (single source: `public/site.css`)

| Group | Token | Dark | Light | Use |
|---|---|---|---|---|
| Surface | `--bg` | `#08090a` | `#f4f5f3` | page |
| Surface | `--surface` | `#0d0f11` | `#fff` | cards, inputs |
| Surface | `--panel` | `#111417` | `#f8f9f8` | raised panels |
| Text | `--ink` | `#eef1f2` | `#101415` | primary |
| Text | `--muted` | `#9ba3a8` | `#556065` | body copy |
| Text | `--faint` | `#7b8389` | `#666e73` | captions (AA on its own surface) |
| Line | `--line` / `--line-strong` | `#242a2e` / `#374046` | `#dce2df` / `#bec9c5` | hairlines / edges |
| Accent | `--accent` | `#64d7c2` | `#087d6d` | primary action, live state |
| Accent | `--blue` | `#78a9ff` | `#2866c7` | quota/limit semantics |
| Domain | `--domain-1..4` | mint / blue / violet / amber | darker equivalents | topology directions |

Typography: Manrope variable (200–800) + system CJK fallback
(`Noto Sans SC`, `Microsoft YaHei`, `PingFang SC`). Display sizes use
`clamp()`; body copy is 13px/1.8 or 14px/1.6. Data numerals use
`font-variant-numeric: tabular-nums`.

Spacing: 8px base step (4, 8, 12, 16, 24, 32, 48, 64). Section padding is
64–88px desktop / 40–48px mobile. Radii: 8px controls, 12px cards, 999px
chips. Borders are 1px; there are no shadows except the single card lift
token already in `site.css`.

Motion: one easing (`cubic-bezier(.2,.8,.2,1)`), 180–320ms for UI feedback,
600–1200ms only for narrative entrances. Every non-essential animation
stops under `prefers-reduced-motion: reduce` and when the tab is hidden.

## 3. Section and component rules

1. **Section index** (`01 / 概览` … `06 / 获取 PANE`) stays the page's only
   numbering system. Do not add a second set of counters.
2. **Cards** are one surface + one hairline + one purpose. No nested cards,
   no gradient card headers, no icon-in-a-box unless the icon is the subject.
3. **Chips** carry state (Available / Beta / Planned / Free), never decoration.
   A chip that does not change a decision should not exist.
4. **Primary action** is the Windows download; every other action is secondary
   or tertiary. A platform without a tested installer gets a status chip and
   no dead button.
5. **Ink marks** (highlight/underline/circle) are limited to the hero and one
   section title. They are not a general emphasis tool.
6. **Proof over adjectives**: numbers, file sizes, version, platform matrix,
   privacy facts. No "blazing", "revolutionary", "seamless".
7. **Privacy and license** are one click from the footer; the free/open-source
   expression lives beside the download action, not hidden in FAQ.

## 4. Forbidden patterns (anti-slop)

- New color tokens outside the table above.
- A second display font or more than three text sizes in one section.
- Glassmorphism, neon glow, floating 3D blobs, stock AI illustrations.
- Autoplaying carousels, parallax on every section, scroll hijacking.
- Rewriting the hero skeleton without a #83 exception recorded here.
- Cards made only of a generic icon + heading + lorem sentence.

## 5. Acceptance checklist (trimmed from oil-ui-pro visual-polish)

- [ ] One clear primary action above the fold; no competing CTA.
- [ ] Every section has one subject; headings are specific, not category labels.
- [ ] Text contrast: `--faint` readable on its surface; no pure-gray on gray.
- [ ] Spacing follows the 8px scale; no orphan 7/13/19px gaps.
- [ ] No overlapping/clipped text at 380, 768, 1280 and 1440 widths.
- [ ] All interactive controls have focus-visible, hover and disabled states.
- [ ] Images have alt text; decorative layers are `aria-hidden`.
- [ ] Reduced motion removes entrances and ambient loops.
- [ ] Dark and light themes both inspected; no hardcoded dark-only hex.
- [ ] Chinese and English routes show the same facts and version.
- [ ] `node --test site/*.test.mjs` green after any UI change.

## 6. Cross-surface anchors

- Palette and domain colors mirror the app's usage topology (#60).
- GSAP is the only animation runtime on the site and in the workshop; do not
  add a second animation library.
- The site's version and provider count come from `package.json`/the provider
  catalog, never from hand-maintained copy.
- The license and privacy facts mirror `LICENSE` and `docs/privacy.md`; the
  site pages are human summaries, those files are authoritative.
