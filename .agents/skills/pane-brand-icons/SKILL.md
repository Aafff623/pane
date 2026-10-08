---
name: pane-brand-icons
description: Regenerate every Pane product-icon asset (exe/ico/icns/android/ios, README mark, site favicon, splash logo) from the single vector master src/assets/brand/pane-mark.svg. Use whenever the Pane brand mark changes or any icon asset looks out of sync.
---

# Pane brand icon pipeline

One vector master feeds every raster asset. Never hand-edit a derived PNG/ICO/ICNS —
edit the master and re-run this pipeline.

## Source of truth

- `src/assets/brand/pane-mark.svg` — the only editable brand mark (512 viewBox,
  transparent rounded-corner tile so taskbar/tray cutouts stay clean).

## Regenerate

1. Rasterize with Edge headless (no ImageMagick/rsvg on this box). The wrapper
   `temp/scripts/icon-render.html` scales the master to the viewport; render 1024 and 512:

   ```bash
   "/c/Program Files (x86)/Microsoft/Edge/Application/msedge.exe" --headless=new --disable-gpu \
     --hide-scrollbars --default-background-color=00000000 --virtual-time-budget=4000 \
     --window-size=1024,1024 --screenshot="D:/code/pane/temp/scripts/icon-1024.png" \
     "file:///D:/code/pane/temp/scripts/icon-render.html"
   ```

   Known trap: `--window-size=128,128` screenshots come out blank (paint race);
   do NOT render small sizes this way — `tauri icon` derives them.
   Always eyeball the 1024 PNG before continuing (Read it).

2. Full set (ico/icns/png ladder/Square*/android/ios) into the bundle dir:

   ```bash
   pnpm tauri icon -o src-tauri/icons temp/scripts/icon-1024.png
   ```

3. Derived copies (same bytes, different consumers):

   ```bash
   cp temp/scripts/icon-512.png app-icon.png            # README / repo identity
   cp temp/scripts/icon-512.png site/public/favicon.png # official site favicon + brand img
   cp src-tauri/icons/128x128.png src/assets/pane-icon.png # splash + header logo (?inline)
   ```

4. Gates: `pnpm build`, `node --test site/site.test.mjs` (locks site asset paths),
   then `cargo build` + `temp/scripts/restart-pane-dev.ps1` — the tray/taskbar icon is
   baked into the exe, so a Rust rebuild is required to see it.

## Splash motion (GSAP)

The startup splash inlines the master mark directly in `index.html#splash`
(svg ids `#sa-arc`, `#sa-needle`, `#sa-hub`; ripples `.sa-rip`; wordmark letters
`.sa-word span`) so strokes can animate. The timeline lives in `main.ts`
`playSplash()` (gsap is a runtime dependency): tile settle → arc draw
(strokeDashoffset 360→145) → needle sweep → hub pop → ripples → letter stagger
→ exit. Reduced motion parks the timeline at 2.45s (composed static frame, then
normal dismiss) — never skip the element. If the mark's geometry changes, keep
`pathLength=360` and the 145 final offset in sync between the svg and the
timeline.

## Do not

- Point README or site at a different mark: `app-icon.png` / `favicon.png` /
  `pane-icon.png` must stay byte-identical siblings of one render.
- Add a second brand concept "for the site"; the 2026-10 drift (light donut in the exe
  vs dark gauge on the site) is exactly what this pipeline exists to prevent.
