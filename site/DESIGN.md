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
| Comic | `--comic-shadow` / `--comic-border` / `--comic-highlight` / `--comic-dot` | derived from `--accent` + `--ink` | same derivation, darker | #89 only, under `html[data-site-skin=comic-pixel]` |

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

## 6. Signal cards (#88)

`#signals` is the one section adapted from the oil-ui-pro card prototypes. The
section skeleton is approved because each card shows a real, verifiable
mechanism instead of a generic icon + heading:

**License.** The owner holds a purchased, perpetual oil-ui Pro licence, so these
cards are used under that licence and do not depend on any upstream third-party
template grant. The boundary record is
`.scratch/site-oil-ui-cards/license.md` (2026-10-26): no unlicensed third-party
assets are copied, no oil-ui private component code is vendored into the site —
the shipped implementation is this repository's own CSS/JS. If the cards are ever
replaced or redistributed, that record and the Pro terms govern.

- **Refresh path** (beam-graph): timer / manual / launch → single-flight gate
  → snapshot → panel.
- **30-day metric** (metric): median-day projection with the sample-data label
  visible; no projection in the first seven days.
- **Local API** (terminal): the real `openusage.limits.v1` envelope served on
  `127.0.0.1:6736/v1/limits`.
- **Shortcut** (keycaps): Alt + 2 summon, Esc dismiss, Shift period.

Card rules: only the §2 tokens (no prototype palette); one finite entrance per
card, started when the section enters the viewport; no ambient loop; no nested
cards; the visual stage is `aria-hidden` and every claim also appears in the
caption. Prototype `--card-*` variables are not imported.

## 7. Comic site skin (#89)

The site previews the in-app comic-pixel skin behind an opt-in
`html[data-site-skin=comic-pixel]` attribute (header palette button, stored in
`pane-site-skin`, or `?skin=comic-pixel`). Native visitors parse none of it.

Seven-piece subset, all CSS and scoped to the attribute: uneven hand-drawn
borders (`2px 2.5px 3px 2px`, uneven radii), 4px hard shadows, Ben-Day dots on
cards (150×96 patch, masked, opacity ≤ .55), a highlighter swipe on section and
card titles (`width:fit-content`, lower half of the text), a burst badge on the
sample-data label, and sticker provider badges (border + 2px shadow +
alternating tilt). The outline-text item is deliberately omitted — at 10px
labels it hurts readability more than it adds character.

This is the only place where the §3 “one hairline / no shadows” card rule is
relaxed, and it never leaks outside the attribute. Comic typography does not
add a font; the display face stays Manrope.

## 8. Cross-surface anchors

- Palette and domain colors mirror the app's usage topology (#60).
- GSAP is the only animation runtime on the site and in the workshop; do not
  add a second animation library.
- The site's version and provider count come from `package.json`/the provider
  catalog, never from hand-maintained copy.
- The license and privacy facts mirror `LICENSE` and `docs/privacy.md`; the
  site pages are human summaries, those files are authoritative.

## 10. Legal pages (#82)

Single source of truth: **`docs/privacy.md`**. The repository-root
`PRIVACY.md` is a pointer plus a short version — it never carries a fact that
`docs/privacy.md` does not, and a disagreement is resolved in favour of
`docs/privacy.md`.

The website keeps one canonical markup per language:

| Route | File | Language |
|---|---|---|
| `/privacy.html` | `site/public/privacy.html` | Chinese (canonical) |
| `/terms.html` | `site/public/terms.html` | Chinese (canonical) |
| `/en/privacy` | `site/public/en/privacy.html` | English summary |
| `/en/terms` | `site/public/en/terms.html` | English summary |

Rules: the English pages state the same facts and add no new promise; every
legal page links `LICENSE`; the outbound-request list on both language
versions must match the table in `docs/privacy.md` (a new endpoint is added to
the doc first). The worker serves `/en/*` legal routes with
`content-language: en` and falls through to the 404 page when the file is
missing.

## 11. Design tier decision (#83)

Three paths were costed in [`public/design/tiers.html`](public/design/tiers.html):
**整理 (organize)** / **升级 (upgrade)** / **重塑 (rebuild)**.

**Chosen: 整理 (organize)** — 2026-10-26, by the repository owner.

Rationale: the site's fact layer (version, provider count, platform matrix,
privacy and legal copy) had just been locked by tests, and a new visual
language would invalidate that acceptance baseline. #87's scroll narrative
fits inside the organize tier as motion consolidation; it does not require a
new information architecture.

Consequence for future work: any change that would belong to the upgrade or
rebuild tier (new type scale, new card system, reordered sections, rewritten
hero) must update this contract and the tier page **first**, then the code.
The demo stage geometry and its annotation anchors are explicitly part of the
frozen skeleton — they are what the organize tier keeps.

## 9. Motion and type tiers (#86)

**Entrance order.** Hero copy rises in one CSS animation; the two mono chip
labels type in after it (`.35s`, second label `.55s`). Section reveals start
when their element is 8% inside the viewport (`.14` threshold), with a
`--reveal-delay` capped at 180ms. Count-up figures start at 45% visibility and
run 1.4s once. Signal cards start only when their section is 18% visible; the
beam then runs its finite CSS timeline. There is no ambient loop on the page
except the provider belt and the topology flow, both behind the shared pause
switch.

**Weight tiers.** Display/headline: Manrope 650–750 with negative tracking.
Body: 400 at 13–14px. Labels and chips: 500, uppercase only for mono
kickers. Numerals: `tabular-nums`; never synthetic bold. This is three tiers,
not a per-section type scale.

**Footer.** The footer is a solid brand field (`--footer-field`) with the
scalable wordmark; the download CTA carries the only ambient wash. No second
glow was added to the footer because it would compete with the wordmark and
break the one-primary-surface rule.

## 12. Scroll narrative (#87)

**One loop.** `public/ticker.js` is the page's only `requestAnimationFrame`
chain. Subscribers ask for a frame rate and get throttled callbacks; the loop
stops when the last subscriber leaves or the tab is hidden, and restarts on
`visibilitychange`. The demo annotation follower, the collector beam and the
count-up figures all moved onto it — before this they were three private loops
that could be alive at the same time.

**One reveal gate.** `public/reveal.js` registers every seen-once entrance.
Observers are shared per `(threshold, rootMargin)` pair, so `.reveal-text`
(`is-revealed`, `.25`) and the section reveals (`is-visible`, `.14` / `-8%`)
now come from two shared observers instead of one per heading plus a second
private one. GSAP entrances register through the same gate with
`className: null` and an `onReveal` callback, because their threshold and
margin are identical to the section reveals. A node is unobserved the moment it
intersects: scrolling back up never replays an entrance.

**Scrub.** `#why-pane`, `#demo` and `#ecosystem` carry `data-scrub`; their copy
blocks carry `data-scrub-fade`. `public/scroll-scrub.js` writes a unitless
`--scrub` **on each fade block** — 1 while its section holds the screen, dipping
toward 0 as it crosses either boundary over half a viewport — and CSS turns that
into `opacity: calc(.55 + .45 * var(--scrub, 1))`.

Three rules are load-bearing:

- **No per-frame layout reads.** Document offsets are cached and re-measured
  only when something can move them: resize, `load`, late web fonts, a
  `<details>` toggle, and a `ResizeObserver` on each section plus `body`. The
  observer is not optional — the demo iframe loads lazily and roughly doubles
  `#demo`'s height after init, which silently invalidated the first cached
  measurement. A scroll frame reads `scrollY` and writes custom properties.
- **Write on the element that uses the variable.** An inherited custom property
  dirties the whole subtree, and `#demo` contains the embedded panel; publishing
  on the section would recalculate style for all of it on every frame of a
  boundary crossing.
- **Opacity only, never transform.** `#why-pane-title` and `#ecosystem-title`
  already carry `.reveal-text`, which owns `transform`; a second transform on
  the same element would fight the reveal. Scrub therefore stays on the
  unoccupied copy blocks.

There is **no IntersectionObserver gate** on the scrub. An earlier draft only
painted sections the observer had marked "near"; measured on a loaded machine,
the observer lagged far enough behind the scroll position that a section already
on screen still carried its CSS fallback (fully opaque) and then dimmed in one
jump. Painting every section is cheaper than that bug, because a section away
from a boundary is saturated at 0 or 1 and the epsilon check writes nothing.

The scrub obeys the shared pause switch: pausing settles every block at
`--scrub: 1` and stops the loop rather than freezing it mid-dip. Reduced motion
never starts it, and CSS pins the fade to `1` so the copy is legible with script
off too (the `var(--scrub, 1)` fallback does the same for no-JS).

**Known cost, not ours.** Measured with a receiver-attributed
`getBoundingClientRect` counter over a 17-step walk of the narrative: the scrub
made **zero** layout reads, and `line-sidebar.js` made 75 on the same three
sections. That is the reading-progress rail, which predates this ticket and
coalesces to one read per scroll event; it is recorded here so the next person
profiling this page starts in the right file.
