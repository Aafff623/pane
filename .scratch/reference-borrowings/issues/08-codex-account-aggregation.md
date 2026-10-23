# 08 — ChatGPT/Codex 账号跨渠道聚合

Status: done (code and automated regression) — owner visual acceptance pending
GitHub: none (reported by owner 2026-10-23)
Priority: P1
Blocked by: owner UI acceptance

## Bug and acceptance target

The initial backup-import fix only added accounts. Once a live default login appeared, the stale imported backup stayed beside it and the two web logins: four tabs, badge 3/4. The bare default tab also lacked an identifiable desktop email/source.

Required identities: laiyif68@gmail.com (desktop), junfennie162@gmail.com (web 3581686f), mahmoud01096235919@gmail.com (web 5548c721). Retire 90ce5aee; do not change vendor auth files or switch accounts. UI target: three tabs and 3/3 while all three providers are healthy.

## Implementation

- Add backward-compatible `imported_from_backup` metadata. Backup import is fallback-only while no live default identity exists.
- Migrate legacy backup rows only when their account id matches a scanned backup and their label matches the generated `email · 本机`. Preserve distinct managed workspaces even if they share an email.
- When a live default appears, remove imported backup rows and persist suppression ids, including backups not yet imported. A later missing live auth file must not resurrect them.
- Managed re-login promotes a marked fallback to a normal web login and removes its generated local label. Same account id never replaces managed web credentials with backup credentials.
- Account-list default metadata uses `default_account_info()` and `email · 桌面`; omit a managed row matching that default id. User notes retain priority.
- Suppression persistence failure now prevents the account deletion save from silently proceeding without a tombstone.

## Same-pattern audit

- Codex/Claude local discovery already seeds its seen set from the default identity and deduplicates extra homes by stable vendor identity. Added injectable roots so both real discovery paths can run against synthetic homes in one parameterized regression, without reading the developer's homes.
- Copilot/Grok/Kiro managed store wins over the legacy OAuth slot for the same account id. Added a parameterized import test that repeats the merge and checks one row, managed label, and fresh credentials.
- Neither Claude nor these three login stores imports `.pane-bak` sources; no new retirement subsystem was added to them. Existing runtime native/slot identity guards remain in place.

## Comments

2026-10-23: Reopened the prior "done" result. The earlier 2/3 backup-error result is historical and is superseded by this correction; re-login to the dead backup is no longer an acceptance step.

- Red first: `cargo test backup_import_live_default -- --test-threads=1` reproduced both unwanted import and failure to retire the old backup (2 failed).
- Directed regression: `cargo test backup_import -- --test-threads=1` passes 5/5, including persisted retirement/restart and fallback promotion.
- First full run: 1023 passed / 1 failed / 14 ignored. The new parameterized test read a previous test's provider-scoped mock OAuth slot; fixed fixture isolation by clearing that mock slot before constructing each case, preserving the assertions.
- A separate test link attempt overlapped the running full test exe and hit Windows Permission denied. Final regression is serialized to avoid the executable lock.
- Frontend final: `corepack pnpm build` passed; `node --test tests/*.test.mjs` passed 104/104.
- GNU debug build passed. Runtime restarted through `temp/scripts/restart-pane-dev.ps1`: `/v1/providers` lists only `codex`, `codex@3581686f`, `codex@5548c721`, all `ok`; store contains the two web entries, and 90ce is suppressed. Repeated restart retains the three-account result.
- Metadata backup (local only): `temp/secrets/account-retirement-20261023-192005/`. No manual vault unsealing, vendor-file edits, model calls, commit, push, tag or release.

## Owner acceptance

- Open ChatGPT: exactly three tabs, with the full desktop email and desktop source; no old 90ce or second junfennie tab; badge 3/3.
- Switch each tab and verify it shows the intended account. Confirm custom notes still take priority.
- Restart and refresh again: the old backup remains absent.

UI appearance remains unverified: the standard launcher leaves the floating window hidden and Computer Use currently discovers no visible Pane target. API health and unit tests do not substitute for owner UI acceptance.

### Final verification (2026-10-23)

- Final full parse-tests: **1025 passed / 0 failed / 14 ignored**, 393.08 s; doc-tests 0 failed. Command: `cargo +stable-x86_64-pc-windows-gnu test --manifest-path parse-tests/Cargo.toml -- --test-threads=1`, MinGW PATH prepended and proxy environment cleared.
- Multi-source regressions pass 2/2 (parameterized across five families). The Kiro fixture uses the harness's mock secret slot directly, since Kiro does not expose the generic OAuth logout API. Earlier failed fixture runs remain recorded; assertions were retained.
- Final Node suite: 104/104. Frontend build and GNU debug build pass; existing build warnings remain. Edited-source whitespace check passes.
- Review and simplification: checked retirement/suppression ordering, managed re-login, default-id dedupe, source marker compatibility and panel config synchronization; unified all scale snapping through the existing helper and avoided a new general account abstraction.
- Running debug app and Vite remain available. Actual GUI interaction is still unverified and remains owner acceptance. No commit/push/tag/release.

## 2026-10-23 close-out review (second expert pass)

- Two real defects found and fixed:
  1. `merged_single_login_from` wrote the store with `let _ = save_accounts_to(...)` and still retired the legacy OAuth slot. A failed write destroyed the only copy of the credential — the pre-batch code popped the row and returned instead. Slot retirement is now gated on a durable account write.
  2. `save_accounts_to` tombstoned every id that left the list, so an incidental shrink — `read_accounts_from` drops a row whose vault lookup misses, a failure mode already recorded in `docs/project-facts.md` — permanently blocked the `.pane-bak` fallback. Tombstones now come only from deliberate delete/archive in `account_lifecycle`; live-default retirement keeps persisting its own suppression set.
- Real-machine evidence for (2): the owner's suppression file held `3581686f…`, which is still an active managed login, beside the genuinely retired `90ce5aee…`. The stale entry is removed after backing both files up to `temp/secrets/`.
- New regressions: `failed_store_write_keeps_the_legacy_single_slot`, `incidental_list_shrink_never_tombstones_a_surviving_account`, `codex_delete_suppresses_the_local_backup_fallback_from_returning`. `removing_an_account_suppresses_backup_reimport` now drives the explicit suppression API. The slot-retention test was reasoned red (the old path called logout unconditionally), not replayed against pre-fix code.
- Codex directed suite 17/17, lifecycle 5/5; runtime keeps three Codex rows, all `ok`.
