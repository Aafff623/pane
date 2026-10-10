# Magic UI Orbiting Circles

Original author: Dillion Verma / Magic UI. Licensed under MIT; see LICENSE.

Source: user-provided Magic UI integration bundle, upstream revision
`ec1cce6c4192c0aaac279dd7e53537ccd5c99d44`.
Upstream: https://github.com/magicuidesign/magicui

The original component, demo, and animation CSS are archived here. The static
website adapts the animation in `public/site.css` without React or Tailwind:
two counter-rotating rings (20 and 10 seconds), equal angular spacing, and
counter-rotation that keeps each icon upright. Radii fit the Pane hub layout.
All 16 provider icons reuse existing `public/img/providers/` assets.
The independent ecosystem section has its own pause and offscreen controls,
and respects the system reduced-motion preference.
