# 11 — Startup critical path and lightweight loading feedback

Status: implemented; native cold-start timing and owner visual acceptance pending

Owner: speed up the loading window and add a tiny creative animation without new dependencies or media. Parallelize independent config/locale IPC reads; preserve locale/skin gate and failed-resource recovery. Verify concurrency with a failing regression before implementation. Report structural improvement separately from native cold-start timings, which remain unmeasured.

Implementation: config/locale IPC now starts concurrently; locale-command failure falls back without breaking config load. Loading meter is inline SVG plus CSS arc/needle motion, no new dependency/media/request, supports prefers-reduced-motion. Keeps existing 8-second reload recovery and locale/skin first-paint gate. No fake progress percentage.

Validation: regression red before implementation; final Node suite 126 passed; corepack pnpm build passed; edited-source git diff --check passed. Browser on running Vite verifies visible 64px meter, active animation, zero SVG image references and animation:none under reduced motion. Script temp/scripts/loading-browser.mjs. No actual native cold-start benchmark or independent visual review; large main JS remains ~444 KB gzip. Build index+CSS gzip grew about 0.38 KB from previous build. No speed multiplier claim, commit/push/tag/release.
