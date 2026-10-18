# 0.8.6 release outcome

Date: 2026-10-18
Tag: `v0.8.6` (annotated, points at `a09638c`)
Branch: `codex/architecture-deepening` fast-forwarded into `main` (`396cbed`),
plus one post-bump site fix (`a09638c`).

## Implementation commits

1. `53880db` refactor: deepen architecture seams across providers, snapshots and vault
2. `378c2e9` fix(overview): show a known balance instead of the unknown-ratio question mark
3. `40d8968` fix(settings): keep the reveal target alive after unlocking the vault
4. `c5d08ed` fix(settings): continue the gated action after unlock and show vault errors
5. `396cbed` chore: release v0.8.6 (version bump, changelog, owner acceptance record)
6. `a09638c` chore(site): refresh demo bundle and hero version for 0.8.6

## Local verification

- Frontend: `node --test tests/*.test.mjs` 51 passed; `tsc` and `vite build` clean.
- Rust: parse-tests `cargo test --offline` exit 0 (874 passed / 13 ignored in the
  earlier full run of the same sources); GNU `check` and `build` clean.
- Site: `node --test site/*.test.mjs tests/i18n-pairing.test.mjs` 38 passed.

## CI

- Run `39130036888` on `main@a09638c`: `site` and `frontend` failed first
  (stale demo bundle, hero version), fixed by `a09638c`; the Rust jobs then
  failed the known intermittent
  `providers::onenewapi::snapshot::tests::scheduled_backfill_does_not_block_key_cards`
  (3-second persistence deadline, already recorded in issue 16). The re-run of
  the failed jobs passed: parse-tests 874 passed / 13 ignored, src-tauri-tests
  890 passed / 13 ignored.

## Release

- Run `39131174243` from tag `v0.8.6`: all three platform builds and the
  `release` job succeeded.
- GitHub Release `Pane 0.8.6` — formal (not draft, not prerelease), 8 assets:
  `Pane_0.8.6_x64-setup.exe`, `latest.json`, `Pane_0.8.6_aarch64.dmg`,
  `Pane_0.8.6_amd64.deb`, `Pane_0.8.6_amd64.AppImage` plus the `.sig` files.
- `gitee-mirror` job succeeded (the "Publish to Gitee" step was slow but
  completed); the whole release run finished green.
