# Pane — Project Context

Verified facts for AI agents working in this repo. Nothing here is guessed;
anything unverified lives under `待确认` at the bottom.

## Purpose and boundaries

- Windows-primary tray app (Tauri v2) that tracks AI coding plans and
  subscriptions: per-vendor quotas with reset windows, local CLI spend,
  tray projections, optional toasts. Vanilla TypeScript + Vite frontend,
  Rust backend, no Electron, one process plus the host WebView. Tagged
  releases also publish macOS dmg and Linux AppImage/deb from the same
  repo (`platform/` seam). Those two are first published binaries — tray
  placement, autostart copy, and menubar/AppIndicator are still
  Windows-shaped.
- Git layout: single remote `origin` = `Aafff623/pane` — an independent
  project (detached from any upstream, 2026-09-06). A single branch `main`
  exists locally and on origin; feature work happens on `codex/<feature>`
  branches.
- Version 0.4.49 (`package.json` + `src-tauri/tauri.conf.json`), identifier
  `com.jazii.pane`, productName `Pane`.
- Privacy boundary (asserted by tests in the code): tokens go only to their
  own vendor's API; pasted keys live in `%APPDATA%\Pane`; the 6736 HTTP API is
  loopback-only, no CORS, Host-checked, and redacts One/New API origins and
  secrets; telemetry is anonymous, opt-out, and must never affect the app
  (every failure path is silent; disabling deletes the state file).

## Domain vocabulary

- **family** — one provider identity shared by all its cards: `claude`,
  `codex`, `cursor`, `kimi`, `onenewapi`, … 28 families in
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
- **Quota Overview (配额总览)** — pinned dashboard section aggregating every
  provider that has a rolling reset window (generalized from the initial 5-hour
  overview in commit `5d03a82`). Header badge = two independent symbols
  (`● N 可用` green, `● N 满额` red, shown only when non-zero), plus a
  `5h` / `Weekly` capsule. Default / 5h tab picks the most binding quota
  (5h session first, otherwise shortest-period percent such as
  daily/weekly/monthly). Weekly tab switches ordinary families to a week
  meter when one exists; Z.ai, One/New API, and Copilot keep the default
  binding.   Click a ring to scroll to that card; hover still shows the
  other window. One ring per family — Antigravity/Cursor extra
  accounts stay as separate cards below, but do not get a second
  overview tile. **maxed (满额)** = the shown window at 100%; providers
  without a time window render as `按量/长期` (non-quota) and don't count into
  the availability tally. Independent pools (Cursor Auto vs API, Antigravity
  Gemini vs Claude) are maxed only when every *present* pool is exhausted.
  Cursor's 5h-tab ring follows `Cursor Models` (Auto), not the API pool.
- **Relay site (One/New API)** — one site entry in `onenewapi.json`
  (version 1): `name`, `base_url`, optional dashboard **access token** +
  `New-Api-User` id, and N relay keys (`sk-…`). Sites are ACCOUNTS of the
  `onenewapi` family (`supports_extra_accounts = true` since 2026-09-05):
  the dashboard renders ONE merged card with a tab per site key; the site
  manager lives in the Customize drawer as the family row's account
  section (moved out of Settings). Card ids: `onenewapi@<key_id>`, or
  `onenewapi@<site_id>` for token-only sites (access token + user id but
  no relay key — display is the subscription endpoint only, billing
  fallback impossible without a key). Subscription numbers come from
  `/api/subscription/self` (access token + `New-Api-User` header) with
  silent fallback to the OpenAI-compatible billing endpoints. The bare
  `onenewapi` card on the dashboard is synthesized at render time
  (borrows the first healthy account's snapshot; never in
  `last_snapshots`), and the family layout entry is seeded once from the
  first configured key's layout.
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
  locally as a `cdylib` (167k-export DLL). The committed crate-type is
  `["rlib"]` (desktop only). Unit tests run from repo-root `parse-tests/`.
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
- `README.md` Features/Providers copy can lag the code — code is the source
  of truth.
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

- `docs/adr/0002-platform-seam.md` — OS capabilities go through
  `src-tauri/src/platform/`; Windows stays the supported tray, Linux/macOS
  are first published binaries.
- `docs/plans/pane-account-model-v2.md` — account model v2 (trend for every
  card, credential accounts, OAuth expansion). Phase 1 landed as
  `usage_history.rs` + frontend trend fallback.
- `docs/plans/` and `docs/superpowers/plans/` — quota architecture,
  api-key quota providers, Antigravity multi-account, UI overhaul phases.

## README assets

- `docs/readme-pane.png` is the published 1200 × 380 README hero. It is a
  flattened PNG: typography and telemetry layout are deterministic, while the
  isolated character/card material is composited into the final image. Keep
  README copy and commands in Markdown; do not replace the hero with an SVG
  that depends on an external raster layer.
- `docs/promo.png` remains the interface proof directly below the hero. It is
  a product screenshot, not a replacement for the project promise in the
  first screen.

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
- 2026-09-05 — One/New API sites are accounts (merged card + tabs, site
  manager in Customize). Agent launch rule learned the hard way: pane.exe
  must be started with stdout/stderr redirected to a FILE — a transient
  agent shell pipe breaks when the shell exits and the next `println!`
  panics, killing the refresh task (symptom: footer stuck "Refreshing…",
  every card ⚠数据过时, `/v1/usage` returns `[]`).
- 2026-09-06 — Overview availability badge style is FINAL: two independent
  symbols (green dot + available count, red dot + maxed count), never a
  combined "7/8"-style solid capsule. User-mandated after rejecting the
  first rendering that shipped in the initial 0.4.48 build.
- 2026-09-07 — Overview generalized from 5-hour session windows to Quota Overview
  with per-window rings: prioritizes 5h session windows, falls back to shortest
  period percent (daily/weekly/monthly) so One/New API daily plans and Cursor/Copilot
  monthly plans get rings too.
- 2026-09-08 — Cursor Team `Total usage` must not meter spend against the
  API dollar floor (`planUsage.limit` ≈ $20). When bucket rows exist, Total
  is text. Commit `fef014a`.
- 2026-09-09 — OS capabilities go through `src-tauri/src/platform/`
  (ADR 0002). Windows 11 may still draw a light focus stroke on the
  frameless popover; left as-is after a DWM `COLOR_NONE` attempt did not
  remove it. Hover on a 5h overview ring shows the weekly sibling, not
  the same 5h line; Copilot/Z.ai keep the ring window. Quota Overview
  header has a 5h / Weekly capsule: ordinary rings follow the tab;
  Z.ai / One/New API / Copilot keep their original binding.
- 2026-09-10 — Refresh semantics: the background loop fetches FIRST then
  sleeps (a live pass runs at launch), and on a total outage (no live-ok
  snapshot) it clears ordinary-error benches and retries every 15 s, at
  most 5 times. Only explicit user clicks (Refresh button, Ctrl+R, the
  overview ⟳) clear ordinary benches via `fetch_usage(clearBenches)`;
  timer/refocus passes stay bench-respecting. 429/rate-limit cooldowns
  survive every path (FailState.rate_limited). The per-card ⟳ command
  (`refresh_provider`) bypasses benches entirely by design.
- 2026-09-12 — Provider additions and spend-panel facts (branch
  `codex/qoder-trae-providers`): Command Code GOAT queries the CLI's own
  undocumented `/alpha/billing/{credits,subscriptions}` (Bearer key;
  `credits.monthlyCredits` is the amount LEFT, an idle 5h window reports
  `resetAt: 0`, planId caps the monthly percent — unmapped plans degrade
  to a dollar line). Kimi For Coding's monthly cap is not in the usages
  endpoint: every refresh rides a parallel `max_tokens=1` probe and only
  a rejection naming "monthly" pins a maxed Monthly row (reset parsed
  from the error text). Doubao is a web-session provider: its Cookies
  SQLite `v10` blobs are AES-256-GCM with the Local State os_crypt key
  and the GCM plaintext carries a 32-byte random header before the value
  (DPAPI direct unwrap fails); Doubao locks the DB while running, so the
  header is cached in `%APPDATA%\Pane\doubao_cookies.json` and re-extracted
  whenever Doubao is quit. The spend panel additionally scans ZCode's
  `~/.zcode/cli/rollout/model-io-*.jsonl` (usage nested at
  `response.usage`, camelCase). Subscription providers expose only
  window percentages server-side — no per-model token data exists there;
  the donut's token views list every provider (the Others fold is
  dollar-only), rendered as three side-by-side period columns. Commit
  `e7c3ec7` / `43a4707` / `25ebb85` / `5757818` / `1a83dec` / `e74af16`.
- 2026-09-13 — ClawsGO Science provider (commit `d113e48`): RPC over HTTP,
  `POST https://api.clawsgo.ai/api/<method>` with body `{"data":{...}}` and
  the web app's Bearer `clawsgo_token` (browser localStorage, pasted into
  Settings; rotates via `set-auth-token`). Credits are milli-units
  (3,000/$1); cycle credits lapse at planEndAt, `balanceMilli` is the
  whole remaining pool. getTeams discovers teamId; getSubscription +
  getUsageStats feed the card. 0.4.51 shipped earlier today; the reset-card
  expiry (`e987f8e`) and ClawsGO ride 0.4.52.
