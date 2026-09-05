# Pane — Project Context

Verified facts for AI agents working in this repo. Nothing here is guessed;
anything unverified lives under `待确认` at the bottom.

## Purpose and boundaries

- Windows tray app (Tauri v2) that tracks AI coding plans and subscriptions:
  per-vendor quotas with reset windows, local CLI spend, tray projections,
  optional toasts. Vanilla TypeScript + Vite frontend, Rust backend, no
  Electron, one process plus WebView2.
- Git layout: `upstream` = official `ItsJazii/pane`; `origin` = `Aafff623/pane`
  (this fork). A single branch `main` exists locally and on origin; feature
  work happens on `codex/<feature>` branches.
- Version 0.4.46 (`package.json` + `src-tauri/tauri.conf.json`), identifier
  `com.jazii.pane`, productName `Pane`.
- Privacy boundary (asserted by tests in the code): tokens go only to their
  own vendor's API; pasted keys live in `%APPDATA%\Pane`; the 6736 HTTP API is
  loopback-only, no CORS, Host-checked, and redacts One/New API origins and
  secrets; telemetry is anonymous, opt-out, and must never affect the app
  (every failure path is silent; disabling deletes the state file).

## Domain vocabulary

- **family** — one provider identity shared by all its cards: `claude`,
  `codex`, `cursor`, `kimi`, `onenewapi`, … 26 families in
  `src-tauri/src/provider_catalog.rs`, mirrored in `src/providerCatalog.ts`
  (keep both in sync).
- **card** — one UI card. Id is either the bare family id (`claude`) or an
  account card `family@<32hex>` — two-lane FNV-1a over the identity token;
  same scheme in `accounts.rs`, `antigravity_accounts.rs`, `cursor_accounts.rs`.
- **Snapshot** — one provider query result:
  `{id, name, plan, status: "ok"|"no_credentials"|"error", metrics, stale,
  warning, dashboard_url}`.
- **Metric** — `kind: "progress"` carries `used_percent` / `resets_at` /
  `period_ms`; `kind: "text"` rows are informational.
- **AccountEntry** — generic api-key account (`label`, `apiKey`, optional
  `baseUrl` for relaybalance), stored in `accounts/<family>.json`.
- **AgSlot** — Antigravity captured Google OAuth bundle (the IDE keeps one
  token in Windows Credential Manager; slots capture one account each).
  Identity = `refresh_token`. File: `antigravity-accounts.json`.
- **CursorAccount** — Cursor OAuth account. Identity = `access_token`.
  File: `cursor-accounts.json`.
- **Key card (One/New API)** — one site+key pair in `onenewapi.json`
  (version 1). Subscription numbers come from `/api/subscription/self`
  (access token + `New-Api-User` header) with silent fallback to the
  OpenAI-compatible billing endpoints.
- **Spend** — `ProviderSpend` (today / yesterday / last30 windows + 30-day
  trend) scanned from local CLI session logs (`spend.rs`), priced via
  LiteLLM / models.dev catalogs (`pricing.rs`, daily refresh, hourly while
  unpriced models exist). Tokens are facts even when no price is known
  (⚠ + `unpriced_models`).
- **Usage history** — `usage_history.json`: per card, daily max used-%,
  35-day retention; synthesizes the 30-day trend for cards without local
  logs (`usage_history.rs`).
- **Tray strip** — up to 4 starred metric entries rendered in the tray;
  main-tray projection in `tray_projection.rs`.

## Important relationships

- Frontend `refresh()` (`src/main.ts`) drives everything. Boot paints
  `cached_usage()` from `last_snapshots.json`, filtered by: disabled cards,
  removed One/New API keys, deleted accounts, and **account swaps**
  (`cache_identities.json` vs current claude/codex identity — a swapped
  family's old bare-id card is never repainted, not even briefly).
- Moonshot (Kimi API) balances fold into the Kimi Code card
  (`fold_moonshot_into_kimi`).
- OAuth: `oauth.rs` device-code flows — codex (OpenAI private flow), copilot
  (GitHub device flow), xai (OIDC discovery, issuer pinned to `auth.x.ai`).
  `cursor_oauth.rs` PKCE flow (`loginDeepControl` → `auth/poll`, 2 s ticks,
  300 s expiry). `cursor_oauth_poll` in `lib.rs` is the ONLY place a Cursor
  OAuth login becomes a stored account (dedup by token fingerprint).
- `alerts.rs` projects each progress metric linearly to period end →
  Ok / Close / RunOut verdicts → optional Windows toasts, once per period.
  A reset time moving >10 min means a new period and resets alert state.
- Kimi Code: a dead OAuth login (rotated refresh token) falls back to a
  pasted plan key (`rotated_fallback`); a Moonshot API key renders the
  wallet rows instead.
- Kimi-routed turns inside Codex logs (`kimi-oauth/k3` etc.) are split out of
  Codex spend and billed to the Kimi card (`split_kimi_routed`).

## Hard constraints

- This machine builds with the **GNU toolchain only** (no MSVC): prepend
  `D:\Tools\mingw64\bin` to PATH and use
  `cargo +stable-x86_64-pc-windows-gnu`. The full Tauri binary cannot link
  locally (167k-export cdylib DLL) — hence the local
  `crate-type = ["rlib"]` edit in `src-tauri/Cargo.toml`, which **must never
  be committed**, and the `src-tauri/target/parse-tests/` harness for unit
  tests.
- Dev runtime needs both processes: Vite `:1420` + `pane.exe` `:6736`. Never
  serve the frontend with Python `http.server` (permanent WebView2 cache
  locks). Launch `pane.exe` via `CreateProcess(lpDesktop="WinSta0\Default")` —
  anything else puts the window on a non-interactive station (invisible).
- UI conventions locked by the user:
  bar colors grade by used-% — 0–60 blue, 60–75 amber, 75–100 red
  (`src/main.ts` around line 1049);
  balance-style quota cards show balance rows, not bar charts;
  card-internal pace predictions are removed from the UI (the alerts.rs
  notification projection stays);
  merged card + account tabs is the approved multi-account pattern.
- `README.md` Features/Providers copy is upstream-inherited and can lag this
  fork — code is the source of truth.
- The user personally does UI acceptance; agents deliver build/test evidence
  plus an acceptance checklist.
- Do not suggest MiniMax to the user (removed from their environment; the
  provider source stays).

## Known failure modes

- Stale UI after a frontend change → WebView2 cache; delete
  `%LOCALAPPDATA%\com.jazii.pane\EBWebView` and restart.
- Window opens but is invisible → launched via `Start-Process`/`&` (wrong
  desktop station); relaunch with the CreateProcess snippet in
  `docs/dev-startup.md`.
- `:1420` refuses to bind → stale Vite/node process; kill the port owner.
- One/New API probe: a wrong access token still returns **HTTP 200 with
  `success:false`** — never treat 200 alone as success.
- MinGW-linked full app fails at process start → expected on this machine;
  use the parse-tests harness instead of fighting the linker.

## Durable decisions

- `docs/plans/pane-account-model-v2.md` — account model v2 (trend for every
  card, credential accounts, OAuth expansion). Phase 1 landed as
  `usage_history.rs` + frontend trend fallback.
- `docs/plans/` and `docs/superpowers/plans/` — quota architecture,
  api-key quota providers, Antigravity multi-account, UI overhaul phases.

## 待确认

- `pnpm` is the package manager in use but only `package-lock.json` is
  tracked (no `pnpm-lock.yaml`). Commit a pnpm lockfile, or standardize on
  npm?

## Resolved decisions

- 2026-09-05 — canonical dev-startup entry is `docs/dev-startup.md`;
  README's "Build from source" now points there instead of
  `scripts/dev-pane.cmd`. The script itself stays (tracked, user-maintained)
  but deviates from the canonical rules: it serves `dist/` via
  `npx serve`/`python http.server` instead of `pnpm dev` and launches
  `pane.exe` with plain `start` instead of `CreateProcess(WinSta0\Default)`
  — treat it as a static preview convenience, not the dev workflow.
- 2026-09-05 — governance assets (`AGENTS.md`, `CLAUDE.md`, `CONTEXT.md`,
  `docs/dev-startup.md`, `temp/` contract files) are tracked in Git; the
  one-off `run_test.cmd` launcher moved to `temp/scripts/` (local-only).
