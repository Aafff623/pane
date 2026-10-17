# MOTDES site effects

ClickSpark and LineSidebar were supplied by the owner on 2026-10-16.
`public/effects/click-spark.js` ports the Canvas burst engine to a native
lifecycle. The hero action group uses 7 line sparks, 9px length, 24px radius,
1.6px width, 440ms duration, low glow, no ring, and the existing accent palette.
Mouse presses are deduplicated against click; touch and keyboard clicks work.
Pause, reduced motion, hidden pages and PixelSwap clear or suppress bursts.
The four actions link to GitHub Issues, pull requests, the repository (Star),
and the Fork creation page. The previous local like/save and copy-link actions
were removed at the owner's request; the links do not mutate GitHub state.
The GitHub action icons are inline Phosphor SVGs: circle-duotone with a central
dot for Issues, git-pull-request-duotone, star-duotone and git-fork-duotone.
Their opacity layers stay intact instead of being flattened into CSS masks.
Each has a 22px glyph in a 44px target, with per-action hover tint, keyboard
focus treatment and a short accessible-name-independent visual tooltip.

`public/line-sidebar.js` retains LineSidebar's smooth falloff and exponential
100ms easing. It uses nine real page anchors, 25px markers, 9px maximum shift,
88px proximity radius, scroll tracking and aria-current. The directory defaults
to a collapsed disclosure on all screens and opens as a fixed overlay; no page
width is reserved. Selecting a section closes it. Reduced motion disables smooth anchor
scrolling; shared pause and PixelSwap suppress proximity animation.

TargetCursor was supplied by the owner on 2026-10-16. Its spring-based four
corners, rotation, target radius, parallax and click squeeze are ported to
`public/demo-target-cursor.js` with the supplied CSS. It is loaded only by the
website Demo, including the Demo build template; no desktop source is changed.
Settings: 24px roaming reticle, 8px corners, 1.5px stroke, 4px padding, 3px dot,
5s spin, 0.24s snap, difference blend and no label. Native input cursors remain.
Fine-pointer and reduced-motion checks, shared host pause messages (origin and
source checked), blur, pointer cancel and visibility all restore native cursors.

Source: the owner-supplied DitherVeil and GlowCursor JavaScript + CSS exports
from [MOTDES](https://motdes.com/components), supplied on 2026-10-15.
The exports describe the components as open source but carry no license header;
no additional license claim is made here.

The owner also supplied PixelSwap on 2026-10-15. `public/theme-swap.js` adapts
its bounded grid, deterministic random order, cubic easing and growing windows
to the browser's incoming root View Transition snapshot. It renders a single
snapshot through an animated compound clip path instead of cloning the entire
website for every pixel. Settings: requested 64px cells, hard cap 220 cells,
850ms total, 300ms pixel opening, initial scale 0.35, no spin or rounding. The
per-pixel fade is expressed through initial geometric growth; no root crossfade.

The prior direct theme-click handler is replaced. While PixelSwap runs it owns
theme motion and suppresses local colour easing and the cursor light. The footer
now renders directly from DOM/CSS and needs no asynchronous canvas paint before
the snapshot. Reduced motion, the shared pause switch and unsupported browsers use
an immediate theme change. The website theme stays isolated from the Demo theme.

`public/effects/dither-veil.js` (retained source, no longer loaded by the site)
and `public/effects/glow-cursor.js` preserve the supplied diffusion, reveal-mask
and fluid-shader implementations. React lifecycle
and refs were replaced with native `create` / `update` / `destroy` functions.
`public/effects.js` connects these to the site's shared pause state, theme,
pointer capability and reduced-motion preference. No React runtime is required.

The owner superseded DitherVeil with a TRAE-inspired solid brand footer on
2026-10-16: full-width Pane mint (#64d7c2), smooth black uppercase PANE in an
SVG text element using the site's existing Manrope font, followed by real source,
docs and feedback links. The pixel mask, reveal canvas, texture, entrance observer
and image-paint synchronisation are no longer part of the site runtime.

Tuned parameters:

| Effect | Settings |
| --- | --- |
| DitherVeil (previous, inactive) | Floyd diffusion, 2px cells, radius 170px; replaced by the solid brand footer |
| GlowCursor | intensity 0.45, trailWidth 1.2px, linger 0.45s, glow 0.35, hotspot 0.15, followSpeed 0.85, low quality, DPR cap 1, no click burst; overlay opacity 0.52 dark / 0.32 light |

OGL 1.0.11 (package-declared license: Unlicense) is bundled at
`public/vendor/ogl.js`, with only Renderer, Program, Mesh, Triangle, Texture and
RenderTarget exported. Upstream: <https://github.com/oframe/ogl>.
Build with esbuild (already present via Vite), ESM format, minified, using an
entry that re-exports those six names from the OGL package. It is served locally;
visiting the site makes no runtime requests to a package CDN.
