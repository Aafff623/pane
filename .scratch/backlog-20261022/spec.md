# Backlog completion — 2026-10-22

Status: active
Branch: `codex/backlog-20261022`
Source: owner instruction 2026-10-22; GitHub #25 #27 #29 #31 #32 #33 #36.

## Objective

Finish the remaining GitHub issue pool in one verified batch:

- close tracker noise (#35 already closed, #33 decided, #27 rescoped);
- finish #29 multi-account center, #25 encrypted backup, #31 official-usage
  probe + pure module, #36 platform plan;
- leave owner-only real-machine acceptance and the 0.8.10 deployment explicitly
  gated, never silently marked done.

## Decisions

- #33 keeps the 2026-10-21 UI decision: `est.` badges stay on period rows,
  column totals and share images; the headline total keeps the explanation in
  its tooltip only. No new header suffix.
- #27 is read-only QA. The write/switch scope retired on 2026-10-21 does not
  return.
- #31 is probe-first. No UI or ledger write before the probe in
  `.scratch/workbuddy-family-v2/issues/09-official-request-usage.md` passes.
- #36 is planning only; no platform code in this batch.
- #25 is local-only encrypted export/import. No WebDAV, cloud or auto-upload.
- Do not touch the uncommitted `site/` work, push, deploy, tag or release.

## Reference scan (2026-10-22)

| Repo | Revision | License | Use |
|---|---|---|---|
| cc-switch | e4873bc | MIT | import validation, current-account as derived fact |
| cockpit-tools | 915e7a2 (pull timed out) | CC BY-NC-SA 4.0 | design only |
| magpie | 8c68b68 | MIT | backup Parts/Result contract, applied-state drift |
| openusage | 26132ff | MIT | identity key vs card id vs login source |
| workbuddy-switch snapshot | 0.1.62 snapshot | no LICENSE | design only |

Top borrow candidates:

1. `official_usage.rs` sweep: capped `total`, `Stalled` decision, boundary
   dedupe on `(requestId, requestTs)`, 100-round cap.
2. magpie backup: selection as a value type, per-part restore report, key
   policy separate from selection.
3. openusage account identity: stable card id vs identity key vs source.
4. cc-switch import gate: validate parsed statements, apply under one lock.
5. magpie `applied.json` / `bypassed`: derived drift instead of an `owned`
   boolean.

## Workstreams

### A. Tracker hygiene

- #33: comment current implementation state, close as completed with narrowed
  acceptance.
- #27: rewrite body to read-only scope, link
  `.scratch/workbuddy-family-v2/acceptance-checklist.md`, keep open for owner
  real-machine sign-off.
- #32: add a regression test for `source_status` (`detected` vs
  `not_detected`), comment code-level evidence; owner spot-check remains.

### B. #36 platform plan

Deliver `docs/platform-support.md` and split child tickets:

- platform difference matrix (Windows-only APIs, tray, windows, paths, single
  instance, signing);
- Linux: WSL2/Docker build, Ubuntu CI, WebKitGTK/GTK/AppIndicator deps, one
  real desktop smoke;
- macOS: Actions runner, arm64 dmg, screenshot-based acceptance, signing /
  notarization separate;
- Android: Tauri 2 minimal proof only; read-only/display surface first; desktop
  tray/popover explicitly out;
- prerequisites: WSL Ubuntu (absent), Android Studio (unverified), Actions
  quota, macOS signing.

### C. #29 multi-account center

Slices:

1. identity semantics: add a derived `current` fact for local-login rows; show
   `Pane 托管` / `本机登录` / `当前登录` labels.
2. last used: persist a non-secret `lastUsedAt` per stable card id, update on
   account selection; keep `lastOkAt` as "last checked".
3. batch remove: backend resolves stable card ids once, takes the mutation lock
   once, returns per-item results; frontend selection mode with confirm.
4. cross-account pack ordering: expose nearest pack expiry from snapshots
   (add structured expiry to WorkBuddy/CodeBuddy pack rows where missing) and
   add a stable "expiry" sort.
5. batch relogin: sequential reuse of the existing login flow, never parallel
   in one family.

Constraints: never write vendor stores; vault-first secrets; en/zh/ru i18n;
GNU parse-tests harness.

### D. #25 encrypted backup

- New `src-tauri/src/backup.rs`: `pane-backup-v1`, Argon2id + HKDF + AES-256-GCM,
  strict schema, size cap.
- Selection: accounts / keys / settings; key policy separate from selection.
- Export: include secrets inside the ciphertext; header exposes only version,
  KDF params, timestamp and counts.
- Preview: decrypt in Rust, report added/conflict/unchanged counts and masked
  labels; never return secrets.
- Apply: preflight OS-vault availability; abort instead of writing plaintext;
  atomic writes; per-section result and rollback snapshots.
- UI: settings section with three checkboxes, password, export/import, preview
  and explicit confirm.
- Tests: roundtrip, wrong password, tamper, duplicate ids, oversized, unknown
  version, no plaintext secret, conflict classification, rollback under
  injected write failures.

### E. #31 official usage

- S0: probe script (redacted output) for CN/AI, enterprise header matrix,
  pageSize 1/3000, one continuation round. Blocked without a verifiable account.
- S1: pure Rust module for sanitize / parse / sweep / merge + fixtures. No UI,
  no ledger write, no network in tests.
- S2+: absolute-host POST, cache, command, UI — only after S0 passes.

### F. Acceptance and release gate

- #32/#27 owner real-machine checks remain owner-only.
- 0.8.10 gate: deploy new `site/public/skins/index.json` + zips, run
  `node site/scripts/check-skin-packs.mjs --live`, reconcile the v0.8.9 release
  commit with main, then owner authorization for push/release.
- Do not merge to main until tests, review and owner acceptance are recorded.

## Verification

- `corepack pnpm build`
- `cargo +stable-x86_64-pc-windows-gnu test --manifest-path parse-tests/Cargo.toml`
- `node --test tests/*.test.mjs`
- `node --test site/*.test.mjs` only if site files change (they should not).
- code-review pass + security review for #25.

## Out of scope

- Writing official client auth files, process control, session DB writes, IDE
  state writes, hooks, cross-account session copy.
- WebDAV/cloud backup.
- Platform implementation (Linux/macOS/Android) and mobile UI.
- Publishing, deployment, version bumps and release notes before owner sign-off.
