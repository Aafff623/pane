# README assets

## Published files

| File | Role | Update rule |
| --- | --- | --- |
| `docs/readme/hero.jpg` | Identity banner (3:1, 2172×724, ~230 KB): wordmark + tagline + real Pane dashboard screenshots (spend donut, quota overview) | GPT-composed from real UI, adopted as-is 2026-09-20 with the user's sign-off that real spend figures and the `v0.4.62 · build 0918.1229` stamp stay visible. Re-encode from the source PNG at `temp/readme-visuals/webai_images/img-r1/`; regenerate on a major dashboard redesign or when the baked version string feels too stale. |
| `docs/readme/how-it-works.jpg` | Five-step flow strip (3.5:1, 2355×668, ~155 KB) under *How it works* | GPT-generated 2026-09-20 keeping the five verbatim labels (Find your accounts / Ask each vendor / Project the reset / Count the money / Stay local). Source PNG at `temp/readme-visuals/webai_images/img-r2/`. |
| `docs/readme-pane.png` | 1200 × 380 first-screen product board (real UI) | Replace only with a flattened PNG/WebP that works without external image references. Keep the README alt text aligned with the visual's actual claims. |
| `docs/promo.png` | Product-interface proof after the hero | Re-capture after a material dashboard or visual-language change. It must remain a real Pane interface, not a mockup. |

The banner carries identity; the screenshots immediately supply product
proof. Commands, provider detail, privacy boundaries, and compatibility
stay in Markdown so they remain searchable and accessible when images
fail to load.

Generated art is published as quality-93 JPEG (PNG sources archived under
`temp/readme-visuals/webai_images/`); keep final files under ~300 KB.

## Palette (shared by all generated art)

| Role | Hex |
| --- | --- |
| Background | `#0B1220` (gradient to `#080D18`) |
| Panel | `#172033` (stroke `#2A3A55`) |
| Primary accent | `#2563EB` (light text variant `#93C5FD`) |
| Warning | `#F59E0B` amber / `#DC2626` red |
| Text | `#F4F4F5` / muted `#71717A` |

Motif: the reset-countdown ring with tick marks — reuse it, do not
wallpaper it.
