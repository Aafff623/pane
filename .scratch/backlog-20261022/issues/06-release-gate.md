# 06 — 0.8.10 release gate

Status: release reconciliation prepared; deploy/push blocked on owner
Blocked by: skin pack deploy + owner authorization + owner acceptance

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
- Release reconciliation is prepared on the isolated branch
  `codex/release-reconcile` in `D:/code/pane-release-reconcile`:
  `c09d666` cherry-picks the v0.8.9 release commit, `be6e001` rebuilds the site
  demo from the reconciled source. Version files are 0.8.9, CHANGELOG has a
  clean Unreleased section plus 0.8.9, and `node --test site/site.test.mjs`
  passes 15/15. No merge to main, no push, no deploy.
- v0.8.9 tag is not an ancestor of main; main still reports 0.8.8.
