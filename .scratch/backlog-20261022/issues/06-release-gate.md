# 06 — 0.8.10 release gate

Status: blocked on owner authorization
Blocked by: skin pack deploy + release commit reconciliation + owner acceptance

## Required before shipping

- Deploy `site/public/skins/index.json` and both zips from a clean tree.
- `node site/scripts/check-skin-packs.mjs --live` must pass.
- Reconcile the v0.8.9 release commit with main: CHANGELOG, version files and
  site demo assets.
- Owner UI acceptance for download -> apply -> restart -> uninstall and old-user
  migration.
- Owner authorizes push/release; agents do not deploy or tag.

## Current evidence

- Local pack gate: OK (signal/ink size and sha256).
- Live index: STALE (old zip sizes 10.26 MB / 4.10 MB).
- v0.8.9 tag is not an ancestor of main; main still reports 0.8.8.
