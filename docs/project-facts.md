# Pane — Verified project facts

Engineering facts for agents working in this repo: scope, key code paths,
constraints, integration boundaries, known failure modes, and dated decision
logs. Nothing here is guessed; anything unverified is marked `待确认`.

Single authorities for the neighbouring concerns:

- **Domain vocabulary (the glossary)** — [`CONTEXT.md`](../CONTEXT.md): terms only.
- **Hard-to-reverse decisions** — [`docs/adr/`](adr/).
- **Work state** (tickets, specs, blockers, Wayfinder maps) — the configured
  tracker, see [`docs/agents/issue-tracker.md`](agents/issue-tracker.md).
- **Startup / build / run / acceptance commands** — [`docs/dev-startup.md`](dev-startup.md).

Last reconciled: 2026-10-28 (first-run source detection #109; 0.12.0 released).

## Scope and boundaries

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
- Version 0.11.0 (`package.json` + `src-tauri/tauri.conf.json`), identifier
  `com.aafff623.pane`, productName `Pane-Aafff623`. The pre-0.11 identity
  (`com.jazii.pane` / `Pane`) was the upstream author's; #105 split it so the
  two apps no longer share install folders or data directories.
- Privacy boundary (asserted by tests in the code): tokens go only to their
  own vendor's API; the secrets Pane itself persists — pasted API keys,
  multi-account keys, Pane-managed login tokens — live in the OS credential
  vault, with plaintext JSON only as the documented fallback while a vault
  write fails or a legacy entry awaits migration. Three stores are
  deliberately outside the OS vault: One/New API sites/keys (owner-only
  plaintext `onenewapi.json`), the master-password key vault
  (AES-256-GCM sealed in `keyvault.json`), and search/MCP keys (plaintext
  `search-keys.json`, the 2026-10-11 search-keys cut). The 6736 HTTP API is
  loopback-only, no CORS, Host-checked, and redacts One/New API origins and
  secrets. The app sends no telemetry, analytics or crash reports.
- Vendor HTTP rides a transport ladder (`providers::send_with_direct_fallback`,
  shared by Codex and — since 2026-10-25 — the CN-facing key/login providers):
  proxied first, and a connect/timeout failure jumps straight to a direct
  attempt, so a dead machine proxy cannot take domestic endpoints down with
  it (measured 2026-10-25: an env proxy breaking reqwest while curl worked).
  Every call still starts on the proxy path, so a recovered proxy is used
  again on the next request; a rate-limited log line reports the rescue.

## Key code paths and relationships

- Frontend `refresh()` (`src/main.ts`) drives everything. Boot paints
  `cached_usage()` from `last_snapshots.json`, filtered by: disabled cards,
  removed One/New API keys, deleted accounts, and **account swaps**
  (`cache_identities.json` vs current claude/codex identity — a swapped
  family's old bare-id card is never repainted, not even briefly).
- Local encrypted backup (`src-tauri/src/backup.rs`): Argon2id + AES-256-GCM;
  account store files, their OS-vault secrets, the already-encrypted
  `keyvault.json` and selected settings. Restore merges account arrays by
  stable id instead of replacing the file, validates paths and secret-key
  prefixes, and snapshots every file before writing.
- Auth center (`auth_center.rs`) rows carry `lastOkAt` (last successful
  query), `lastUsedAt` (latest day with positive usage; cumulative meters are
  differenced so flat polling days do not count; percent quota samples are
  excluded because occupancy is not activity), and `current` (only when
  exactly one official-client local sign-in is visible).
- Quota trends: a card with no absolute usage counter records its
  longest-window progress bar as a per-day percent high-water sample in
  `usage_history`; `meter_trends` serves it to the card trend only after
  every real quantity unit. Percent samples never enter credit totals or
  `lastUsedAt`, and a renamed bar replaces the card's old percent series.
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
  `["rlib"]` (desktop only). Unit tests run from repo-root `parse-tests/`
  (1200 passing / 0 failed / 18 ignored in the latest serial proxy-free run,
  2026-10-28; run with
  `cargo +stable-x86_64-pc-windows-gnu test` and no proxy).
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
  `%LOCALAPPDATA%\com.aafff623.pane\EBWebView` and restart.
- Window opens but is invisible → launched via `Start-Process`/`&` (wrong
  desktop station); relaunch with the CreateProcess snippet in
  `docs/dev-startup.md`.
- `:1420` refuses to bind → stale Vite/node process; kill the port owner.
- One/New API probe: a wrong access token still returns **HTTP 200 with
  `success:false`** — never treat 200 alone as success.
- Window "can't be summoned" (Alt+2 / tray click / relaunch all no-op) →
  a **minimized window still reports `is_visible() == true`** (Win+D
  iconifies without clearing WS_VISIBLE), so every toggle routed to the
  hide branch and the popover was unreachable. Since 2026-09-18
  `toggle_popover*` treats iconic as hidden and calls `unminimize()`
  before `show()`; the blur auto-hide also ignores a focus loss <500 ms
  after gain (system focus yank, not a user click-away). Diagnostic
  fingerprint: `IsIconic=True` + `GetWindowRect ≈ (-21333,-21333)`.
- pane's `6736` HTTP API dead right after a restart → the new instance
  **silently fails to bind** while the old socket sits in TIME_WAIT (it
  still prints the URL). Wait for the port to be free before relaunching;
  `temp/scripts/restart-pane-dev.ps1` does kill → port-wait → launch →
  probe → summon in one shot.
- MinGW-linked full app fails at process start → expected on this machine;
  use the parse-tests harness instead of fighting the linker. The harness
  does NOT compile the command layer in `lib.rs` (verified 2026-10-25: a
  type error in `run_usage_fetch_inner_mode` passed parse-tests green and
  failed the real `cargo build`) — changes there need a real
  `cargo +stable-x86_64-pc-windows-gnu build` as their gate.

## Provider credential semantics (2026-10-03, three handoff providers; tokenrhythm added 2026-10-19)

- **ClinePass** (`clinepass`): sk_ key, no expiry, no session. `GET
  api.cline.bot/api/v1/users/me/plan/usage-limits` → three rolling windows
  (five_hour/weekly/monthly). `percentUsed` is a 0-100 percent read
  directly (CodexBar's official plugin clamps 0..100 without scaling —
  verified against its source); `resetsAt` is RFC3339-with-nanos and is
  ABSENT while a window has zero usage, and is dropped when already in
  the past (rolled anchor — no fake countdowns). ClinePass supports pooled
  API-key accounts in `accounts/clinepass.json`; each saved key receives a
  stable account card and the provider header shows the pooled-account count.
  A rejected key is rendered as an explicit error state rather than a gray
  unknown dot. The current local pool contains the pre-existing key plus two
  retained labeled accounts; a revoked third key was removed on 2026-10-04.
- **SenseNova** (`sensenova`): quota is console-session only (sk- keys get
  401 auth_type_disabled). `GET platform.sensenova.cn/lite/console/v1/
  tokenplan/pool-usage`; string decimals, unix-second resets, pools keyed
  by `pool_type` (default|dedicated), never by localized name. Renewal is
  the documented ladder in providers/sensenova.rs: L0 local JWT exp
  precheck → L1 refresh_token (public client `nova`, single-flight +
  atomic temp-file persist + 30s/5m/30m backoff, invalid_grant drops the
  token) → L2 paste-tokens bootstrap (webview silent SSO re-auth deferred
  until its interception is runtime-verified — rationale: three unverified
  chain points, shipping blind violated the delivery bar) → L3 stale
  snapshot with warning. 429 (TPM/RPM) never enters the ladder.
- **APIGOTO** (`apigoto`): sk key, official read-only `GET api.apigoto.com/
  v1/usage`. `is_active` covers key+user, NOT subscription liveness; a
  lapsed subscription still returns 200 with `subscription` gone and
  mode=payg — the parser must show an expired state, never a fresh 0%
  card. No window reset instant exists by design (anchor-rolling); the
  surfaced reset is `subscription.expires_at` (the metric's own death).
  Error mapping trusts numeric `code` (documented stability contract),
  never message text. `/v1/usage` reports the ACCOUNT, not the key: keys
  of one account return identical payloads, and `fold_same_account_keys`
  (run twice per refresh — before the history/cache section and after the
  stale-restore pass) collapses same-account cards into the default
  account's card with an `API keys: N · one account` row. Error cards are
  never folded (a rejected key needs its own visibility), and the row
  write is idempotent across the two passes. Ids a cycle's folds dropped
  are evicted from the last-good cache so a boot paint cannot flash a
  duplicate; a disabled family drops nothing and keeps its cache.
- **基元律动 / tokenrhythm** (`tokenrhythm`): a WEB-SESSION provider —
  the `sk_tr_…` key authenticates ONLY the `/v1` model namespace; every
  console endpoint that reports money (`/api/wallet/summary`,
  `/api/api-keys/usage`, `/api/call-logs`) answers **401 to the key**
  (verified 2026-10-19 against the live site with a real key). The console
  authenticates by login session only — production is **cookie-only**
  (the Bearer branch in its bundle is mock-only; the CSRF value is read
  from the `tr_csrf` cookie and sent as `X-CSRF-Token`). So Pane hosts the
  login itself: a Pane-owned webview window (`tr-login`) opens
  `https://tokenrhythm.studio/login`, the human clears the SMS captcha, and
  each add/relogin uses an **incognito WebView2 session**, so the next
  account does not silently reuse the previous browser login. The
  cookie manager (`platform::read_webview_cookies`) reads httpOnly cookies;
  `/api/auth/me` must confirm `code:0` plus a non-empty `data.id` before
  persistence. Multi-account sessions use `login-accounts/tokenrhythm.json`
  and per-account `token:login/tokenrhythm@…` vault slots. Cookie/CSRF ride
  the existing access/refresh secret fields; public account rows expose
  labels and masked values only. The standard plaintext file fallback
  applies when the OS vault is unavailable. The old `web/tokenrhythm`
  single-session slot is migrated save-first and then retired.
  - Account identity comes from the server user id, never the cookie or
    label: re-login updates the original account and keeps its label.
    The first account publishes as `tokenrhythm`; later accounts publish
    stable `tokenrhythm@…` cards and share the merged account tabs.
  - An expired wallet session keeps the account row with `no_credentials`
    and a re-login hint; it never clears a sibling account. Deletion removes
    the selected account and matching archive; archiving retains credentials
    for restore in Settings → Keys & quota.
  - Login and legacy migration are single-flight. Global and single-card
    refreshes check login/default identities before accepting results, so
    deleting or promoting an account cannot publish its older in-flight read.
  - Response shapes of `/api/wallet/summary` were captured from the live
    site on 2026-10-19 (owner's first login): the envelope is
    `{code, message, data}` and the balance rides as the decimal string
    `data.availableBalanceCny`. The parser still fails honestly with
    `待确认` on an unrecognized shape instead of inventing numbers.
  - Login-completion detection is Rust-side polling of `/api/auth/me` — the
    login window is a REMOTE origin, and Tauri refuses app commands from a
    remote webview by default ("not allowed"); granting the capability
    would need an `AppManifest` in build.rs, which turns on ACL gating for
    every command in the app (an unrelated command gets refused at once).
  - The `tokenrhythm` family is a **web-session family**: its Customize row
    carries a jump button (`data-session-jump`) that opens the settings
    panel on that family's key row, because the sign-in control lives on
    the key row rather than in the drawer.

## Popover window chrome (2026-10-19)

- The popover's own chrome lives in the total-spend card's head, left of
  the daily-detail button (that row is always the first thing in view);
  the panel-form window has a real frame and gets none.
- The panel-form settings chrome (`minimize` / `close`) must call the backend
  `minimize_panel_window` / `close_panel_window` commands, not
  `getCurrentWebviewWindow().minimize()/close()`. The WebView-window calls can
  fail under the panel capability set, and the old empty `.catch()` made the
  red close button appear completely inert. A static test now locks the
  backend-command path and checks every rendered `close/cancel/dismiss`
  control has a delegated handler. Debug CDP verification: close removed the
  `panel` page target, minimize made `is_minimized(label="panel")` true, and
  reopening the panel made it false. The installed `0.9.1` binary predates
  this fix.
- Three config keys, all defaulting in `config_with_defaults` (Rust) and
  mirrored in `FRONTEND_CONFIG_KEYS` (frontend): `popoverPin` (default
  `true` = always-on-top), `popoverLock` (default `true` = re-center on
  every summon, no dragging), `popoverPos` (`{x, y}` or null = the
  position the user dragged to). `lastView` (the surface to restore on the
  next summon) sits alongside them.
- `popoverPin` is a WINDOW property the OS forgets on restart, so it is
  re-applied on every summon from config; the drag region must likewise be
  re-armed on the fresh DOM — and the cards are rebuilt from scratch on
  every render, so the card-reorder grips' drag opt-outs are re-armed in
  `renderDashboard`.
- The three window permissions the chrome needs
  (`allow-set-always-on-top`, `allow-center`, `allow-start-dragging`) are
  declared in `src-tauri/capabilities/default.json` for the `main` and
  `panel` windows only.

## Search/MCP quota system (2026-10-03, searchquota + keyvault unification)

- **Key resolution** (superseded 2026-10-11 by the search-keys cut, commit
  961c50e): bocha/tavily/firecrawl/brave keys come ONLY from the
  Pane-managed plaintext `search-keys.json` (`search_keys::for_service`) —
  never from environment variables, `tavily-keys.json`, or a third-party
  MCP client config, and never gated behind a vault unlock
  (`keyvault::keys_for_service` routes search services away from the vault
  entirely). What the user adds in Settings is exactly what Pane queries.
  `keyvault_add/remove` still calls `searchquota::invalidate_service` so
  the next tick refetches with the new key set (no 45-min cache stall).
- **Multi-key = multi-pool**: every provider sums pools for the headline
  metric (the overview ring) and lists one row per key with the vault label
  (or a masked tail for legacy keys). Firecrawl's headline reset is the
  EARLIEST billing_period_end across keys.
- **Brave has no quota endpoint** — the only signal is rate-limit headers
  on real search responses (`X-RateLimit-Remaining: "0, 1994"` = monthly
  slot last). One real query is spent per probe, cached 12h per key
  (~2/day ≈ 3% of the free 2000/month). Reset = now + X-RateLimit-Reset.
- **Keenable: no quota surface at all** (probed /v1/usage|quota|account|me
  → 404; real search responses carry no quota headers/fields; only signal
  is 429 retryAfter) — console-only, not implementable.
- **Exa**: official `GET api.exa.ai` team-management usage endpoint exists
  (total_cost_usd etc.); no key on this machine — wire when one lands in
  the vault.
- **Tavily `/usage` carries NO period dates** — never fabricate a reset;
  plan (Researcher = 1000 credits/month) is surfaced as the plan label.

## Token spend coverage system (2026-10-04)

- Token totals are the primary fact; provider-reported cost wins when present,
  otherwise the shared pricing catalog derives dollars. Unknown prices do not
  erase measured tokens: they remain in totals/trends and are marked
  `unpriced`.
- Current local forms and routing rules are catalogued in
  `docs/token-spend-coverage.md`. Sources include JSONL session logs, SQLite
  ledgers, Antigravity protobuf-in-SQLite, OpenCode Desktop's
  `~/.local/share/opencode/opencode.db`, and Cursor's authenticated CSV.
- `spend::source_statuses()` plus the `fetch_spend_sources` Tauri command expose
  whether each known source is detected locally, absent, or runtime-only. This
  is coverage evidence, not a claim that every tool turn is persisted.
- **cc-switch ledger source (2026-10-05)** — `ccswitch_db_data()` in `spend.rs`
  read-only scans `~/.cc-switch/cc-switch.db`: `proxy_request_logs` detail
  (last 35 days, `created_at` in SECONDS) UNION `usage_daily_rollups`
  (permanent, `date` = local-day TEXT). `input_token_semantics` CASE normalizes
  fresh input (0=legacy incl. cache-read, 1=total incl. both caches, 2=fresh;
  cache-inclusive apps: codex/gemini/grokbuild). Merging is day-level set
  difference, never double-counted: claude's card switched to cc-switch as
  primary (its proxy records real tokens where relays zero the CLI logs —
  self-scan undercounted 7~24x; self-scan keeps only days cc-switch lacks);
  codex/opencode/grok keep Pane's scan as primary with cc-switch filling
  missing days; pi folds onto the claude card with both pi destinations
  yielding cc-covered days; mcode (MaxCode), gemini and claude-desktop are
  cc-switch-only cards (mcode is spend-only: frontend `providerCatalog.ts`
  entry, no Rust quota provider). Costs always come from Pane's own catalog
  (`probe_lookup`), never cc-switch's `total_cost_usd`; rollup rows price at
  base rates (no per-request long-context claim). Model splits
  (MiniMax/qwen→AihubMix/kimi-routed) also apply to cc-switch rows.
  Mid-iteration read errors abort to the last good parse — a truncated day
  set would poison the day-diffing. **cc-sourced-days flag (2026-10-19,
  GitHub #5 second review)** — the two sources share no row-level request
  identity (`proxy_request_logs` carries cc-switch's own
  `request_id`/`session_id`; a session-granularity id cannot split the
  direct/proxied mix inside one session, and the >30-day
  `usage_daily_rollups` rows carry no id at all), so a day the ledger
  covers is taken from the ledger alone — the local same-day rows are cut
  to avoid double-counting. `cc_sourced_days()` counts those days on the
  card's final data and `ProviderSpend.cc_sourced_days` carries the count
  to the UI ("cc-switch ×N"). The count is a PROVENANCE fact, not a
  completeness claim: the ledger also imports the same local session logs
  (`data_source = session_log` — measured 2026-10-19 on the author's db:
  1903 session_log rows / 11 days vs 3545 proxy rows / 12 days for
  claude), so a covered day is cc-switch's normal state. An earlier
  revision flagged "days both sources cover" as proof of missed direct
  traffic — disproved by that `data_source` column and removed. No numbers
  change; the flag is additive.
- **Likely-undercount marker (2026-10-19, ticket
  `.scratch/cc-switch-undercount/`)** — `likely_undercount_days()` sums
  per-day tokens on both sides (each normalized to the same five buckets:
  fresh input + output + cache read + cache writes) and counts days where
  the LOCAL scan holds more than the ledger shows; the cut keeps the
  ledger's lower number, so those days likely miss records cc-switch's
  session import didn't capture. One-way by design: the ledger
  legitimately EXCEEDS the local scan when a relay zeroed the CLI logs
  (the reason it is primary), so only the excess direction flags.
  Thresholds (10% relative margin + 2 000-token absolute floor) are a
  tunable judgment call. It marks suspicion and never auto-corrects —
  adding the difference back would double-count. The UI badges
  "low? ×N" ahead of the provenance badge.
- **spend_history VERSION = 3** — v1→v2 cleared zcode cells (cache
  double-count fix), v2→v3 clears only bare `claude` cells (cc-switch source
  switch). Extra `claude@<fnv1a>` account cards keep their history: cc-switch
  covers only the default `~/.claude/projects`, so their self-scan accounting
  is unchanged and clearing would erase unreplacable data.
- **Spend heatmap (2026-10-05)** — the total-spend bar's detail icon opens a
  26-week Sunday-aligned heatmap from `spend_history` (`fetch_spend_daily`
  → `daily_spend`), styled after ZCode's usage board: square shrink-to-fit
  cells (`repeat(26, minmax(0,1fr))` + `aspect-ratio: 1`, no horizontal
  scroll), month labels under the grid (a column containing the 1st belongs
  to the new month), Less/More legend on top; hovering shows $/tokens/tool
  count, clicking a day opens per-tool/per-model cards; Esc/backdrop closes.
  The panel is `overflow-x: hidden` — nothing inside may force a min width.

## RTK command-output savings (2026-10-24)

- RTK savings is the third Total Spend metric beside dollars and tokens; it is
  deliberately outside `lastSpend`, the donut, spend history, quotas and daily
  notes. `saved_tokens` is RTK's bytes ÷ 4 estimate — never billable tokens,
  never subtracted from spend, never added to quota. There is intentionally no
  `--quota` metric. Because it lives inside Total Spend, `showTotalSpend`
  hides it too.
- Global source: `rtk gain --all --format json` (`summary` + `daily`), 10 s
  timeout via `platform::hidden_command`, 60 s in-memory cache in
  `rtk_savings.rs`. A manual Pane refresh sends `force: true` and bypasses that
  cache. Missing binary, non-zero exit, invalid JSON and timeouts return
  `status` (`missing` / `unreadable`) instead of panicking.
- Project attribution: `%LOCALAPPDATA%\rtk\history.db` (+ `-wal`/`-shm`) is
  copied to a temp folder and opened read-only; rows bucket by local RFC3339
  date and `project_path`. Paths lose the `\\?\` prefix; temp, AppData,
  `.qoder-cn`, `resources/app`, user-root, date-shaped and GUID-shaped
  directories are filtered. Only basenames reach the UI — never raw commands
  or full paths.
- Windows: today/7d/30d sum merged daily rows; `all` uses `summary`. `daily`
  merges RTK's rows with `%APPDATA%\PaneAafff623\rtk_savings_history.json` (same-day
  replace, 400-day retain, atomic tmp + rename; an unreadable file is kept as
  `.json.bad`). `otherSaved` holds identifiable projects below the Top-5 cut;
  `unattributedSaved` holds temp/noise rows and old rows without a project,
  so Top + other + unattributed reconstructs the window total. The history DB
  is read before the CLI summary; if a command still lands mid-read and the DB
  briefly leads by a few bytes, rows are scaled into the CLI budget so the
  breakdown stays visible instead of being replaced by an error.
- Every history row is also grouped by RTK command channel (`rtk_cmd` second
  token: grep, git, read, cargo…) and project source (`project_path`). The
  channel section shows Top 5 + other channels + unclassified channels, while
  the source section shows Top 5 projects + other + unattributed, so both
  breakdowns close to the CLI window total. RTK's schema does not record the
  invoking harness, so harness provenance cannot be inferred honestly.
- UI states: installed RTK gets an accent-coloured terminal icon. `missing`
  mutes the metric tab and opens setup guidance with the official Winget
  package `rtk-ai.rtk` plus a website link. The installer is Windows-only;
  harness setup is available for Codex, Claude Code, Cursor, Trae, Windsurf,
  Cline, Kilo Code, Antigravity, Kimi Code, GitHub Copilot, Gemini CLI,
  OpenCode, Factory Droid, Mistral Vibe, Hermes, Pi and Oh My Pi.
- Harness setup uses a fixed Rust allowlist; the webview cannot pass arbitrary
  flags. Global targets run `rtk init -g ... --auto-patch`, except Codex, whose
  `--codex` mode rejects `--auto-patch` in RTK 0.50.0. Cline, Kilo Code,
  Antigravity and Kimi Code are project-scoped: Pane opens a native folder
  picker and runs RTK with that folder as its working directory. Setup buttons
  stay disabled until RTK is installed. Detection checks known user/project
  config paths in two passes: an exact `rtk hook <tool>` marker first, then a bounded
  fuzzy scan for RTK plugin/rules filenames or content inside only those known
  paths. Detected rows are green (`exact` or `fuzzy`), missing rows are red.
  The configuration dialog follows CC Switch's card hierarchy, neutral bordered
  controls, brand-icon tiles and generous whitespace, with larger type.
  The primary savings figure keeps two decimal places in `M` so small new
  savings do not disappear behind one-decimal rounding.
  Codex currently registers only a `Bash` matcher; PowerShell commands do not
  pass through RTK and therefore do not increase its counters. The dialog says
  this explicitly rather than implying that a green Codex row means every shell
  is covered.
  RTK's `unreadable`, invoke-failure and `scanning` states are hints, never a
  fake zero. Project-only failure degrades the list while the CLI windows stay
  live.
- Tests: unit tests in `parse-tests`; the live probe is `#[ignore]`
  (`real_machine_rtk_savings_is_readable`).

## Tencent Cloud OrcaTerm quota card (2026-10-24)

- Reads only `%APPDATA%\com.orcaterm-desktop.app\data.bin`, the official
  desktop client's JSON session. Pane never writes it. The local token is
  `oauth_access_token`; `oauth_expires_at` is milliseconds.
- Calls `https://api.orcaterm.com/cgi/api` with the `orcaterm` product header:
  `DescribeUserBillingStatus` for quota rows and `OAuthRefreshToken` only
  when the disk token expires within 60 seconds. A refreshed token stays in
  memory and is bound to its source disk token, so switching accounts in the
  client discards the old cached token immediately.
- The billing card shows `roleType`, the monthly `hunyuan_ai` request window,
  `space` in GB, `secret` count and `session_share`; `-1` renders as
  Unlimited. Missing or non-numeric usage is an error, not zero usage, and no
  plan limit is hardcoded.
- 401/403 become `no_credentials`; transport and parse failures become
  `error`. A concurrent snapshot uses one fetch mutex so two expired-token
  reads cannot race the refresh.
- 2026-10-24 local observation: Basic plan, AI requests 0/1,000 with a
  monthly window, storage 0/10 GB, credentials 0/5 and shared sessions
  Unlimited. These values are calibration only, not product constants.
- Pending owner test: refreshing a still-valid token may revoke the desktop
  client's own token. The 60-second margin limits exposure but does not prove
  the vendor behavior.

## Key vault and credential storage (2026-10-05 → 2026-10-08)

- **Key vault encryption** — `keyvault.json` v2: per-entry metadata
  (service/label/note/masked preview) stays plaintext so the locked vault
  still lists its contents, while each key VALUE is sealed with AES-256-GCM
  under a key derived from the master password via Argon2id (19 MiB/2/1,
  OWASP floor; `argon2` + `rand_core` crates). A sealed canary distinguishes
  wrong passwords from corruption. The derived key lives only in process
  memory — restarting the app locks the vault. While locked,
  `keys_for_service` returns empty and searchquota providers fall back to
  their legacy sources (provider key files / env). Legacy v1 plaintext files
  load unchanged until a password is set; setting one rewrites the file
  encrypted, permanently.
- **Provider/account keys in the OS vault (2026-10-08; extended 2026-10-15)** —
  the 22 providers' API keys, every multi-account key, AND all Pane-managed
  login tokens now live in the OS credential store via `keyring`
  (`secretstore.rs`, service "Pane"; entry names `provider:<family>` /
  `account:<card-id>` / `archived:<card-id>` on the stable
  `<family>@<fnv1a>` ids, plus `token:<scope>` slots — `token:oauth/<provider>`
  for the device-flow single slots, `token:login/<card-id>`,
  `token:codex/<card-id>`, `token:cursor/<card-id>`,
  `token:searchkey/<entry-id>`). *(Amended 2026-10-19: the
  `token:searchkey/<entry-id>` slot is stale — the 2026-10-11 search-keys
  cut (commit 961c50e) moved search/MCP keys back to the plaintext
  `search-keys.json`, routed away from the vault entirely; see the
  search/MCP section below.)* JSON files keep only non-secret fields
  (labels, ids, expiry; cursor rows carry a `cardId` snapshot because the
  card id hashes the access token). Reads lazily migrate old plaintext —
  the vault write must succeed before the field leaves the file, and a
  failed vault write keeps the plaintext file as the fallback. The 2026-10-11
  "three deliberate plaintext exceptions" entry is superseded (owner 拍板
  A 案, analysis-hygiene batch, commit f73b4d1): search-keys.json,
  login-accounts/*.json, the oauth single slots and the cursor/codex
  account stores all migrate on first read. *(Amended 2026-10-19: the
  "remaining exception `workbuddy_web.json`" claim was itself stale —
  since 53880db the WorkBuddy web-session cookie is vault-first through
  `secretstore::save_session("web/workbuddy")` (slot
  `token:web/workbuddy`), the JSON file only surviving a failed vault
  write; SECURITY.md/privacy.md had copied the stale sentence and were
  corrected in the same pass.)*
  `reveal_provider_key` / `reveal_account_key` demand
  the master password (same canary + session as the vault; "查看" in the
  panel) — the password gates human viewing, background refresh never needs
  it. `change_master_password` verifies the old password first. The
  parse-tests harness swaps in an in-memory store via the `harness` feature.

## Update mechanism

- Signed with this project's own minisign key (private key in the
  `TAURI_SIGNING_PRIVATE_KEY` repo secret; local backup in
  `temp/secrets/` — losing it kills update signing). The detached upstream's
  trypane.xyz/ItsJazii endpoints were dropped. New versions raise an in-app
  popup (`maybePromptUpdate`, once per version per run) beside the footer
  update button. Since 2026-10-10 the updater follows the app-wide proxy
  chain (config `proxy.url` → Windows system proxy, via
  `providers::outbound_proxy_url`; system mode reads static
  `ProxyEnable=1`, and in PAC mode (`ProxyEnable=0` + `AutoConfigURL`) falls
  back to the local mixed listener published in `ProxyServer`, accepted only
  when its loopback port answers. Pane does not execute PAC scripts, so a
  PAC-only client still needs the manual URL.) and caps the manifest check at
  20 s —
  the plugin's reqwest is built without the system-proxy feature, so
  system-proxy users previously updated over a direct github.com link.
  Transport failures render the localized `update.networkHint`. Channel
  details: `docs/update-mechanism.md`.
- Endpoints are tried in order and the updater only moves on when a request
  fails or answers non-2xx (tauri-plugin-updater 2.10 source), so the fast
  source must be listed first. Since 2026-10-14 the ladder
  (`updater_endpoint_strings()` in `lib.rs`, overriding the single URL
  pinned in `tauri.conf.json`) is: **edge route with `?source=gitee`** when
  a proxy is configured (a CN user behind a global proxy exits abroad, so
  the geo branch would miss exactly the users who suffer most — a proxied
  client therefore asks for the mirror explicitly), then the **plain edge
  route** (geo by request country), then **GitHub latest.json** as the
  reachable-everywhere fallback. The edge route serves the same GitHub
  manifest but rewrites the Windows installer URL to the Gitee mirror for
  CN clients, after probing that the mirror asset exists — a missing
  mirror falls back to the GitHub URL, so it can cost speed but never
  availability. `PANE_UPDATE_DEBUG=1` prints the resolved order to the
  log. Measured 2026-10-14 on the author's machine: GitHub release asset
  ~37–39 KB/s (direct and proxied), own CF domain ~43 KB/s, Gitee release
  attachment ~315 KB/s.
- The Gitee mirror is uploaded by the `gitee-mirror` job in `release.yml`
  (action `nICEnnnnnnnLee/action-gitee-release`, pinned) using the
  `GITEE_TOKEN` secret; the job fails loudly when the secret is missing. It
  needs a Gitee repo (`fan-tengda/pane`, branch `master`) that exists with at
  least one commit. Bytes are identical to the GitHub asset and the minisign
  signature is verified client-side, so the mirror is never trusted.

## UI facts (2026-10-04; context-menu anchoring added in 0.4.68)

- Overview rails and the provider column explicitly clip horizontal overflow;
  the category/period switch row may wrap within the card so Shift + wheel
  cannot expose a page-sized blank side region.
- Health dots distinguish `green` available, `yellow` peak, `red` maxed, and
  `error` fetch/auth failure. An account-card removal targets only its
  `family@fingerprint`; the family card remains enabled.
- The overview month tab falls back to a weekly metric when a provider has no
  monthly quota and labels that fallback. Skin market has a native-reset action;
  the selected mascot is anchored at the outer card's bottom-right edge.
- Global shortcut registration restores the previous working binding after a
  failed replacement, while Settings shows an explicit availability/conflict
  state. ClinePass uses the official Cline bot mark (Magpie icon set, MIT) in
  `src/assets/providers/clinepass.svg`.
- Spend details render the selected period as a single active column; local
  shortcut settings cover six focused-window actions and reject duplicate
  bindings. Pooled-account count badges use blue informational styling.

## Durable decisions

- `docs/adr/0002-platform-seam.md` — OS capabilities go through
  `src-tauri/src/platform/`; Windows stays the supported tray, Linux/macOS
  are first published binaries.
- `docs/plans/` and `docs/superpowers/plans/` (removed 2026-10-11 as completed
  and absorbed) — used to hold quota architecture, api-key quota providers,
  Antigravity multi-account and UI-overhaul phase plans; the durable outcomes
  live in this file and the ADRs.

## README assets

Maintenance authority: [`docs/readme-assets.md`](readme-assets.md) — the
published-file table, the bilingual (zh-CN / en) mirroring rules, and the
"hardcoded numbers that drift" list. Keep it in sync when the README pair
changes.

- The README header uses `src/assets/pane-logo.png` as the product identity
  mark (both `README.md` and `README_EN.md`).
- Feature sections embed nine theme-adaptive screenshot pairs from
  `attachments/*.png` (hero overview + weekly view, architecture diagram,
  customize drawer, spend heatmap, skin workshop gallery, settings keys
  page, expiring list), captured 2026-10-14 from the 0.8.1 demo frontend
  via `temp/scripts/readme-shots.mjs` (real window sizes, device scale 2,
  demo data) plus `temp/scripts/readme-diagram-shot.mjs` for the diagram.
  GitHub picks dark or light per the reader's theme through `<picture>`
  elements.
- The READMEs replicate yetone/magpie's section skeleton with Pane content
  (2026-10-14): centered header with an embedded hero screenshot, "Is Pane
  for you?", quick start, supported sources, "How it fits together" diagram
  picture, eight What-it-does subsections, docs table, privacy paragraph,
  community + star history, license. Banners and screenshot strips from the
  pre-2026-10-04 layout remain gone.

## CodeBuddy family v2 — WorkBuddy AI, channel provenance, local tokens (2026-10-19)

- **Four family members, one table** (`codebuddy_login::FAMILIES`):
  `codebuddy` (codebuddy.ai, platform `ide`), `codebuddycn`
  (codebuddy.cn, `ide`), `workbuddy` (copilot.tencent.com, `workbuddy`, CN
  build), `workbuddyai` (**www.workbuddy.ai**, platform **`workbuddy-ai`**,
  WorkBuddy's INTERNATIONAL build — NOT CodeBuddy International).
  `tests in codebuddy_login` assert every row is in `LOGIN_FAMILIES` and the
  provider catalog, so a half-added member fails the harness.
- **Variant differences** (upstream `workbuddy-switch` v0.1.62
  `variant.rs`, verified against its source): AI billing lives under
  `/billing/meter/...` first and only falls back to `/v2/billing/meter/...`
  on HTTP 404; AI refresh needs `X-Auth-Refresh-Source: plugin` (without it
  the vendor answers `invalid_grant` 12153); desktop auth files are
  `workbuddy-desktop.info` (CN) vs `workbuddy-desktop-ai.info` (AI) in the
  same directory; data roots are `~/.workbuddy` vs `~/.workbuddy-ai`.
- **Isolation is enforced, not assumed.** A scan whose token `domain` does
  not belong to the family (`workbuddy.ai` suffix = AI, everything else =
  CN) is refused before it is stored. The web-cookie fallback is CN-only;
  the AI card never borrows it.
- **Scan state is per family** (`PENDING: HashMap<family_id, Pending>`), so
  a CN scan can no longer kill an AI scan. Token refresh is single-flight
  per `family:account_id`; a queued caller re-reads the store and reuses the
  rotated pair instead of refreshing twice.
- **Quota provenance.** Only non-default sources append a `Source` row
  (`desktop app (read-only)` / `web session (explicit import)`); a Pane
  login card stays clean. Package detail is deliberately compact: a pack
  expiring within 7 days gets its own row (at most three), and the rest
  collapse into one summary row. Measured on the owner's real account
  (2026-10-19): 13 live packs would have produced 13 rows; the compact form
  shows one `Packages` row beside the `Credits` meter. An unparseable
  deadline prints `expiry unknown` rather than inventing one.
- **Desktop sign-in is read-only for usage.** The old
  `workbuddy_import_desktop` command and auth-center button were removed on
  2026-10-23; Pane no longer copies a desktop session into its login store.
  The provider may still read the plaintext desktop store as an automatic
  tier-1 usage fallback; `$wbEncrypted` is reported, never decrypted. Web
  login is the only account-creation path.
- **Local token sources** (`spend.rs`): WorkBuddy CN/AI and CodeBuddy CLI
  use one `workbuddy_line` parser (three usage locations, input includes
  cache read, reasoning already inside completion; verified on real files
  2026-10-19); CodeBuddy IDE reads `CodeBuddyExtension/Data/**/history/**/
  index.json` (`requests[].usage`, model from the workspace index). All
  four stay separate cards and never blend with the credits ledger. The
  request fingerprint is shared across files (oldest first), so a copied/
  forked session counts once; this bypasses the per-file day-total cache
  and re-reads the 31-day window on every scan.
- **Not implemented on purpose**: official `get-user-request-usage`
  ledgers (needs a verifiable account — probe plan in
  `.scratch/workbuddy-family-v2/issues/09-official-request-usage.md`),
  account switching, session copies, check-ins, hooks, Agent Companion.

## API-key migration and folded-account identity (2026-10-22)

- Legacy provider keys live in both `<provider>.json` metadata and the
  `provider:<family>` secret slot. Removing only the file leaves a key
  resolvable from the vault; the next refresh can import it again.
- API-key account migration holds the same mutation lock as CRUD. It saves
  active accounts before retiring the provider vault slot and metadata file.
  Vault removal happens first so relay `baseUrl` survives a vault failure.
  Final snapshot identity acceptance only reads the current account list;
  it does not enter the migration lock a second time.
- An archived identity wins over an active duplicate created by the old
  migration bug. Durable active-list repair precedes collecting that
  duplicate's active vault slot. Permanent deletion and archive retire a
  matching legacy source without touching another identity's source.
- Folded account tabs and overview counts share one filter: account index
  zero publishes once under the bare family id, and only other active
  identities publish under `family@...`. One account has no tabs/ratio;
  valid native env/CLI bare snapshots still work with an empty saved list.
- API-key card-menu Add account opens that provider in Customize, where the
  account form is added. Pinning is display preference, never a credential.
- Re-adding an archived identity is rejected by `account_add`/`account_update`;
  `account_update` and the legacy index-based `account_remove` retire the
  matching legacy source first. Relay cards recover `baseUrl` from an active
  row when `<provider>.json` was already deleted, and retirement matches by
  API key so a missing baseUrl cannot strand the old provider slot. Migration
  failures are logged instead of being swallowed.
- Account lists are backend state, not config: frontend mutations emit
  `accounts-updated` so the Settings panel and popover drop their separate
  `accountsCache` entries. Display notes live in
  `config.layout.providers[<card id>].note`; `accountNote` reads the stable
  id plus the legacy `@__default__` alias and writes through `setAccountNote`
  to one canonical key. The bare family id IS the provider card's layout key:
  its note is the card title, never an account name, so a bare default
  account reads/writes `family@__default__` instead. Codex accounts default
  to the ChatGPT/Google email before the backend label; every other account
  falls back to label → email → localized `Account N` (never the provider
  name, a fingerprint, or a masked token). Notes are single-line,
  capped at 48 code points, survive reconcile/archive/delete, and are shared
  by Settings, Customize, dashboard pills, overview/expiry details, spend
  details and the auth center. Auth-center rows edit that same note from
  their `⋯` menu (`renameAuthAccount`: note first, backend label as the
  fallback); local read-only rows keep no menu but still display a note set
  elsewhere. Provider notes remain the card-title override.
  OAuth device/browser families (codex/grok/copilot/kiro/windsurf/trae/zed)
  route Customize to Settings/Auth center instead of rendering a second
  login UI.
- `renderDashboard` replaces `#providers.innerHTML` wholesale. It must
  capture/restore `scrollTop` (and the focused `[data-card-account]`) around
  the rebuild; otherwise every account-tab switch resets the list to the top,
  which reads as a violent window shake when the card is scrolled into view.
- `body.ui-scaled` applies CSS `zoom`. `getBoundingClientRect()` is in visual
  pixels while fixed `left/top`, `offsetWidth` and the layout viewport are
  unscaled; every fixed menu/tooltip must go through `placeMenuAt` /
  `placeFixedTip` (or `scaledUiRect` + `uiViewport`). Raw
  `rect + window.innerWidth` positioning is a drift bug at any scale ≠ 1.
- Skin packs: official sources live in `skins/<id>/` (`skin.css` + `assets/`)
  and are zipped into `site/public/skins/` with a sha256 manifest; the app
  bundle must never import them. Runtime packs live in
  `%APPDATA%\PaneAafff623\skins\<id>\`, and Rust serves each entry CSS with assets
  inlined as data URLs. `signal` / `ink` are ordinary packs: the workshop
  shows index entries as remote until installed, and a persisted shaped id
  survives boot (`readSkin`) while `migrateSkinFromIndex` downloads it.
  The index + zips must be deployed together before the flip; the demo's
  pack loading path is separate (issue 08). `node
  site/scripts/check-skin-packs.mjs [--live]` is the mechanical gate: it
  verifies local size/sha256 and, with `--live`, compares the deployed
  index (expected to report STALE until the release deploy). 0.8.8 still
  bundles signal/ink, and the workshop keeps installed packs usable even when
  the deployed index is stale; a stale index cannot reach released users as a
  downgrade. It matters only for the unbundled build. Deploy from a clean tree — `site/` may
  carry another session's WIP.
- Deleting a One/New API key/site purges backend config/cache, and the
  frontend immediately drops the key card from `lastSnapshots`/`lastSpend`/
  `accountsCache` then refreshes accounts and broadcasts `accounts-updated`;
  waiting for the next usage refresh left stale account pills visible.
- Evidence: legacy-vault resurrection red/green, archive/delete repeated
  refresh and failed migration tests, relay retirement failure test, account
  alias regression, and a headless Customize routing check. Latest serial
  proxy-free Rust run: **1170 passed / 0 failed / 18 ignored** (2026-10-26, after
  the identity migration, `/v1/limits` family/balance work, refresh-broadcast
  coalescing and the cc-switch UI tail). Node suite: **211 passed**; site
  suite: **21 passed**. Frontend build + skin-pack local check pass. The Codex
  backup-retirement and slot-retention corrections are covered by that run.
- Codex credential-source precedence is **live default > managed web store >
  `.pane-bak` fallback**. A backup import is marked `imported_from_backup`
  and is retired once a live default identity appears; retirement persists
  into `codex-backup-suppressed.json`, including backups not imported yet, so
  a temporarily missing `auth.json` cannot revive them. Tombstones are written
  ONLY by deliberate delete/archive (`account_lifecycle`) — an incidental list
  shrink, such as the vault-miss drop above, must never suppress the fallback.
  The legacy single OAuth slot is retired only after the account list is
  durably written.

## Codex managed accounts and callback semantics (2026-10-22)

- Codex has exactly **two user-facing credential sources** (no pasted API
  key, no ChatGPT Desktop import, no manual token import):
  1. **Local CLI home(s)** — `CODEX_HOME`, otherwise `~/.codex/auth.json`.
     The default home is the bare `codex` card and the read-only
     `CLI 登录` row; extra discovered homes become `codex@<hash8>` cards
     (OpenAI id_token provenance required). Refresh may renew and write
     the token back to `auth.json` with a `.json.pane-bak` copy.
  2. **Pane OAuth** — browser PKCE on `127.0.0.1:1455`; if the Codex CLI
     holds the port, it automatically falls back to device code. Every
     completed login is a managed `codex@<hash8>` account in
     `codex-accounts.json` + the OS vault; accounts can be added, renamed
     and removed independently.
- The legacy single slot (`%APPDATA%\PaneAafff623\oauth\codex.json`) is imported
  into the managed store and retired; it is not a third method.
- “Add account” writes a cross-window `panelIntent`, but a write made *by
  the panel itself* never echoes back as an event. `openFamilyLogin` must
  apply the intent locally (switch to the auth view + `openAuthLoginDialog`);
  relying on `applyPanelIntent` alone made the panel's add button a no-op.
  The dialog shows the copyable authorization URL, falls back to the device
  code when `:1455` is busy, auto-opens the browser, and cancels the backend
  flow on close.
- `account_list("codex")` puts the local CLI row first (id `codex`,
  `maskedKey: "CLI 登录"`) and then managed accounts, so a count of “1 个
  账号” can mean the CLI login alone.
- The boot cache applies the same fingerprint dedupe as the live fan-out:
  `configured_extra_account_ids` drops the stored `codex@<hash8>` that
  matches the current default CLI account id, so that account cannot paint
  twice until the fetch lands.
- The local CLI login is read from its Codex home (`CODEX_HOME`, otherwise
  `~/.codex/auth.json`), including a UTF-8 BOM. Auth Center keeps this local
  source read-only even when a managed subscription uses the same email;
  email alone cannot identify a ChatGPT workspace.
- Browser PKCE accepts only a GET to `/auth/callback` with the current,
  unique nonempty state and code. Unrelated requests and invalid state do
  not end the listener. Reads/writes are bounded, and the response tells
  users to confirm completion in Pane rather than claiming token storage
  already succeeded. A busy loopback port falls back to device code.
- Both PKCE and device-code completion save directly to `codex-accounts`.
  Cancellation/supersession checks precede persistence; frontend poll
  responses cannot overwrite a replacement flow. Re-login keeps custom
  labels, while per-account refresh serialization and credential-version
  checks prevent stale writeback after re-login or deletion.
- The old single OAuth slot is retired after durable account storage;
  managed OAuth accounts are queried from their own records, avoiding a
  second bare-card refresh of the same token. JSON persistence completes
  before deleted Codex vault slots are collected. Reset credits route to
  the managed account that owns the card.
- `codex-accounts.json` keeps only non-secret metadata once tokens move to
  the OS vault. If the vault lookup then misses, `read_accounts_from`
  retains only rows with a non-empty access token, so that account silently
  disappears from `account_list` (no re-login row). The local CLI row
  (`~/.codex/auth.json`, marked `CLI 登录`) is separate and stays visible —
  a ChatGPT Desktop sign-in is not a Codex CLI sign-in.
- These paths have synthetic loopback, temp-store and frontend regression
  coverage. Real two-account OAuth and TokenRhythm login require owner UI
  acceptance; automated tests are not evidence of completed consent.

## Provider onboarding descriptor (2026-10-19)

- **One source of truth** for "how does this provider connect":
  `src/providerOnboarding.ts` (`onboardingFor(family)` → credential kind +
  ordered actions + hint/steps i18n keys). Defaults derive from the catalog;
  `SPECIAL` overrides the families whose real flow differs from the flags.
- The Customize ⚙ panel (`renderCustConfig`) renders those actions. A family
  with no API key can never expose `pasteKey`: login/read-only families get
  `renderOnboardingConfig` (hint + numbered steps + "Add account / Sign in"),
  which routes to the auth center through the existing `panelIntent`
  cross-window message. Key families keep the paste/test/save form.
- **Fixed by this split** (all were live bugs before): 12 login-only families
  opened the paste-key dialog — 11 of them really wrote a key into
  `accounts/<family>.json` and then failed as `unknown provider`
  (codex, codebuddy, codebuddycn, workbuddy, workbuddyai, copilot, grok,
  kiro, windsurf, trae, qoder, zed); the settings page called `oauth_start`
  for nine browser/scan families that only implement `<family>_login_start`;
  `stepfun-plan`'s account/password panel was shadowed by the multi-account
  branch; `get_credential_status` read `accounts/<family>.json` for login
  families, so a signed-in CodeBuddy/WorkBuddy showed as unconfigured.
- Login families now always land in the auth center (one login UI, not two);
  Codex/Grok/Copilot keep the inline device-code block in the drawer.
- **Not migrated from Magpie**: the plugin runtime, `migrations.json`, the
  giant `Provider` struct, name-based stable ids, and risk-confirmation
  dialogs. Magpie's `mechanism facts` equivalent is weaker than Pane's.
- **Known limit**: opening one provider's ⚙ re-renders the whole 64-row
  drawer synchronously (~7 s in the debug WebView). Pre-existing; optimizing
  it is a separate ticket.

## 待确认

- **Web search / flex 分项计费保持关闭（#43-3，2026-10-27）。** `pricing.rs` 与
  `spend.rs` 里没有任何 web-search 或 flex 服务层的价目，厂商也没有发布可机器
  读取的单价表：这些调用在部分计划里按「次」计费、在另一些里已含在订阅内，
  同一个 slug 在不同账号上单价不同。按本票规则「无可靠价目则保留关闭并文档化，
  不猜价」，这类行继续按 token 计价、不额外加价；要开启必须先拿到官方单价来源。

- Z.ai (family `zai`) 的 pay-as-you-go 余额端点没有官方确认：无 Coding Plan
  的 key 保持 `needs_setup` 的「待确认」文案（`no_plan_setup`），余额回退
  在确认端点前不实现、不猜端点、不造 0%（#48-3，2026-10-26）。
- Qoder 国际版 (family `qoder`) 的 Windows 配置目录按命名惯例取
  `com.qoder.app.stable`（CN 版为 `com.qodercn.app.stable`），备选 `Qoder`；
- **Qoder CN 本机读数（`qoder_sessions.rs`）不依赖任何版本号。** 数据库位置靠发现：
  `%APPDATA%\com.qodercn.app*` 里最近更新过的 `main.sqlite` 才算候选，安装目录
  `.qoder-versions/<版本>` 从不参与解析。
- **厂商本地不存实测 token 数**（2026-10-23 在 0.4.11 实测）：
  `chat_session_context_usage` 自己声明 `tokenCountsAvailable=false`、
  `chat_session_messages.payload_json` 内 0 处数字型 `*_tokens`、转录
  `runtime-config.contextWindow=null`。因此 token 总量只能是
  `context_usage_ratio × 窗口` 的推导值，`credits` 才是远端实测的钱。窗口本身会被升级打掉：
  本机 12 个模型只有 3 个还发布非空 `context_window`。`QoderSpend.telemetry` 把
  「模型总数 / 有窗口数 / schema 版本 / 是否有实测 token / 估算基数」一起吐出去，
  卡片必须显示基数；缺窗口只计入 `tokens_unavailable_trips` 并显示「—」，
  **绝不代填默认窗口**（升级后自检：`node temp/scripts/verify-qoder-drift.mjs`）。
  端点 `openapi.qoder.sh` 与 API 路径 (`/api/v2/user/plan`、
  `/api/v2/quota/usage`) 已由参考源证实，但目录名待真实装机核验 (2026-10-08，Batch 2)。
- Trae 国际版 (family `trae`) 的用量端点先试 v2 再回退 v1
  (`/trae/api/v{1,2}/pay/ide_user_ent_usage`)；CN 版实测 v2，cockpit 参考为 v1，
  国际版真实版本待装机核验 (2026-10-08，Batch 2)。
- `pnpm` is the package manager in use; `pnpm-lock.yaml` and
  `package-lock.json` (CI's `npm ci` gate) are both tracked as of
  2026-10-10 — the earlier "only package-lock tracked" question is closed.

## Resolved decisions

- 2026-09-05 — canonical dev-startup entry is `docs/dev-startup.md`;
  README's "Build from source" now points there instead of the old
  `scripts/dev-pane.cmd`. *(Amended 2026-10-11: `scripts/dev-pane.cmd` was
  deleted — it served `dist/` via `npx serve`/`python http.server` and
  launched `pane.exe` with a plain `start`, both contradicting the canonical
  dev rules; use `temp/scripts/restart-pane-dev.ps1`.)*
- 2026-09-05 — governance assets (`AGENTS.md`, `CLAUDE.md`, `CONTEXT.md`,
  `docs/dev-startup.md`, `temp/` contract files) are tracked in Git; the
  one-off `run_test.cmd` launcher moved to `temp/scripts/` (local-only).
  *(Amended 2026-10-11: the whole `temp/` tree is now local-only and
  untracked — the contract files stay on disk but are not tracked.)*
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
  header is cached in `%APPDATA%\PaneAafff623\doubao_cookies.json` and re-extracted
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

## Local panel/account validation (2026-10-21)

- Proxy clients (`http` / `http_no_redirect`) re-read the effective proxy
  and replace the current pool when its URL changes. Direct fallback keeps
  `no_proxy()`. A loopback mock-proxy regression checks both redirect modes;
  real system-proxy switching still belongs to owner acceptance.
- **Proxy discipline for new code:** every outbound HTTP client goes through
  `providers::http()` / `http_direct()` (or a provider-local wrapper that
  calls them). Never build a bare `reqwest::Client` for a public endpoint —
  it silently ignores the app-wide proxy. Loopback requests always set
  `.no_proxy()`.
- Settings tabs share a top-aligned reading column. Release history lives
  in Settings → About; the popover dashboard no longer contains a timeline.
- Managed login lifecycle commands share `account_lifecycle.rs`; the parsing
  harness exercises real persistence, archive/restore failure rollback,
  same-account re-login, and last-account deletion. This is not an end-to-end
  test of Tauri IPC or browser login.

## Settings shell and official WorkBuddy requests (2026-10-24)

- The settings panel (`panel` window) is borderless/transparent, non-maximizable,
  default 980×700 logical (min 860×620), remembers `%APPDATA%\PaneAafff623\panel-window.json`.
  On this dev machine Tauri's 150% scale and WebView2's 169.5% DPR make that a
  ~868×620 CSS viewport; the 200px rail collapses to 170px at ≤1024 CSS.
- `index.html` no longer contains `<aside id="settings">`. Every legacy control
  lives in `src/settings/controls.ts`; `mountSettingsControls()` appends it as an
  inert hidden source and the panel moves each node into the skeleton. The
  popover keeps the source but never shows a settings sheet — `#settings-btn`
  opens the panel.
- Skeleton classes: `.set-section` (18/600 tab title + 12/400 description),
  `.set-section__card`, `.set-row` (14/600 label + optional 12/400 description),
  `.set-shell__nav/__head/__scroll/__frame`. Search indexes every rendered
  `[data-setting-anchor]`; coverage is checked at runtime and in
  `tests/settings-search.test.mjs`.
- `updateCheckHours` is a real config key (4/24/168/0, default 4). The Rust
  background checker re-reads it every 30 minutes; manual checks never wait.
- WorkBuddy CN's card adds one text-only `Official requests (30d)` row from
  `get-user-request-usage` (probe verified 2026-10-24, 26 rows). It never writes
  `record_meter_samples` and is never summed with `credit_history`; a truncated
  sweep says `· partial`.
- Gemini CLI spend now has a native parser (`spend::gemini`) for
  `~/.gemini/tmp/<project>/chats/session-*.json(l)`. cc-switch proxy days win;
  native rows only fill missing days. No local Gemini CLI sample exists on this
  machine, so `/v1/sources` still reports `needs_probe`.


## First-run onboarding and source detection (#109, 2026-10-28)

- **One first-run flow** — the separate provider picker no longer opens before
  the guide. Onboarding is four steps (welcome → detected sources → connect →
  done); the detection step reads `source_probe`, shows install evidence and
  credential source, and can rescan without restarting. The empty board and
  grouped board keep a "Continue setup" entry. Settings replay opens the same
  flow but does **not** apply provider picks.
- **Read-only probe** — `src-tauri/src/source_probe.rs` checks PATH/PATHEXT,
  known install directories and environment names; it never executes a CLI or
  opens a network connection. Environment is process + HKCU/HKLM, re-read on
  every call, so `setx` is visible after Rescan. Only names and presence are
  returned; secret values never enter the UI or `/v1/providers`.
- **States** — `installState` is `installed` / `missing` / `unknown` (unknown
  means no hint, never "not installed"). `credentialState`/`credentialSource`
  distinguish env, OS vault and local sign-in. Cards show "installed, not
  signed in" separately from "not installed".
- **API** — `/v1/providers` rows add `installState`, `installEvidence`,
  `credentialState`, `credentialSource`, `credentialEnvs`;
  `/v1/doctor` adds `installed_but_not_signed_in` as an `info` check with a
  remediation. Common keys without a Pane quota reader are reported as
  found-but-unsupported rather than hidden.
- **No scan writes** — a usage refresh never writes `disabled` or
  `removedProviders`. `no_credentials` is ambiguous (never configured vs a
  transient read failure), so hiding is left to explicit picker/onboarding
  confirmation and Customize toggles; `hideUnconfigured` folds empty cards
  visually without changing the saved list.
- **Provider help dialog** — a fixed header with a scrolling body: icon/name/
  credential badge, a 2×2 facts grid, three steps, a collapsed “how it works /
  limits” disclosure (Phosphor info + caret in a clickable row), then the
  actions in normal flow (no footer bar or divider). The actions must not live in a `<footer>`: the app-wide `footer`
  rule pins it absolutely and covers the body.
- **Failed-card UI** — every non-ok card renders one `repairPanelHtml`: quota
  status, a plain-language title/description, exactly one primary action
  (key/OAuth setups open Pane; local/session sources open login instructions),
  and expandable diagnostics (steps, source, install state, raw provider
  response). `credentialFailure` has an `empty` kind for a completed query
  with no quota data, separate from `unsupported`; rejected credentials also
  evict the last-good snapshot cache so an old plan cannot reappear.
- **Evidence** — real `source_probe` + `/v1/providers` + `/v1/doctor` checked
  in the running dev build; onboarding four steps, rescan after a temporary
  `setx`, and the detection env-key row were walked on the real window.
  `tests/onboarding-source.test.mjs` locks the detection-row/evidence
  contract. Full Node suite 244 passed; build and Rust check pass. The probe's
  Rust unit tests run inside the `parse-tests` harness (serial, proxy stripped:
  1200 passed / 0 failed / 18 ignored); the app crate's own test binary still
  cannot link on this machine (known GNU `STATUS_ENTRYPOINT_NOT_FOUND`).

## Default theme and skins (2026-10-24, 0.9.0)

- Default light/dark now use VS Code Dark+/Light+ neutrals: dark `#1e1e1e`
  background, `#252526` cards, `#3c3c3c` borders, `#d4d4d4` text, `#007acc`
  accent; light `#f3f3f3` workbench, `#ffffff` cards, `#d4d4d4` borders,
  `#333333` text, `#0066b8` accent. Tokens stay `--background/--card/...`; no
  second palette.
- signal / ink / comic-pixel keep their own token blocks. The settings shell's
  active-nav rule is class-only (`.set-shell__nav .panel-nav-item.active`) so a
  skin's four-class rule can own the colour. Under skins the `.set-section`
  wrapper has no border/background, because the section head + card already
  paint the skin frame (otherwise comic/ink showed double frames).
- Comic Pixel's installable pack is `site/public/skins/pane-skin-comic-pixel-1.0.0.zip`
  (manifest + CSS + two fonts); installed locally at `%APPDATA%\PaneAafff623\skins\comic-pixel`.

## Forward-only pricing pins and refresh single-flight (2026-10-25, #56/#54)

- **Observation timeline** — `%APPDATA%\PaneAafff623\pricing\schedules.json` stores
  every price Pane ever charged as an append-only `Observation` list
  (`scope`, normalized `model`, `tier`, `observed_at`, `pin`, `basis`). The
  file wraps the serialized list in a `payload` string with an FNV-1a
  `checksum` and `count`; any byte edited outside Pane is quarantined as
  `schedules.json.corrupt-<ms>` and cached dollars stay vouched until the
  owner runs the explicit rebuild. Writes use a process-unique temp file
  (overlapping dev instances must not share one `.tmp`).
- **Resolution** — `pricing_timeline::resolve(scope, model, tier, ts)` returns
  the last observation with `observed_at <= ts`; events before the first
  observation use it with `estimated = true`. All spend scanners price through
  `probe_price`/`effective_price` (`src-tauri/src/spend.rs`) at the event
  timestamp (day-granular sources use local 23:59:59.999). A later catalog
  change, price rise/cut, override or corrections bump only appends a new
  observation at "now" — never rewrites history.
- **Cache** — `PERSIST_VERSION` is 7 (v6 caches are discarded once and
  re-parsed; the first launch after the upgrade re-reads spend logs). Each
  entry records a 5×4 KiB FNV-1a content fingerprint (`sample_fingerprint`):
  mtime+size alone cannot see a OneDrive/rsync rewrite that preserves both.
  Unchanged files must match the fingerprint; a grown file may resume only
  when the prefix fingerprint over the recorded length and the 4 KiB boundary
  both match. Files under 20 KiB hash in full; UNC/WSL paths sample two points
  instead of five. Probes are `Pin { scope, key, tier, ts_ms, pin }`; replay
  is exact against the immutable timeline, so a catalog refresh no longer
  re-reads history. `CORRECTIONS_REV` changes keep the cache and append one
  `basis:"correction"` observation pass. Unpriced first sightings write no
  entry (the later fill happens once, marked estimated).
- **Scheduled prices** — `pricing::observe_scheduled_price_change` (spend.rs)
  turns a dated rate change into a timeline observation at the change instant;
  today only `gemini-4-argon` ($2→$4 input on 2026-12-01) uses it, so the rise
  is forward-only even when every catalog feed answers 304 that day.
- **UI/commands** — `rebuild_spend_pricing` (Settings → price overrides →
  重建花费历史, double-confirmed) drops `spend_cache.json` and forces a full
  reparse; unchanged days come back identical. `save_pricing_overrides` now
  appends forward-only `basis:"override"` observations. Estimated cells render
  `≈`/`≈Nd` (`ProviderDays.estimated` → `spend_history.ModelDay.estimated` →
  `RangeSpend.estimated_days` / `DailyModelSpend.estimated`).
- **Refresh gate** — `refresh_gate::run(id, kind, op)` is per-card
  single-flight (leader + waiter fan-out, 120 s leader timeout, panic →
  error). `refresh_provider` and the periodic per-provider spawn both use it;
  `usage_fetch_lock` still serializes whole cycles. Frontend card refreshes
  use `RefreshState.requestCard/finishCard/acceptCardResult`, and
  `usage-updated` merges per card instead of dropping the whole payload.

## Skin system: market transport, legibility and transitions (2026-10-26, #57)

- **Market transport** — `skin_pack_index` in `src-tauri/src/skin_packs.rs` fetches
  `https://pane.threetwoa.live/skins/index.json` through
  `send_with_direct_fallback`, validates id/version/https URL/sha256/size and
  rejects duplicates. `src/skins.ts::refreshSkinIndex()` uses `invoke` whenever
  `__TAURI_INTERNALS__` exists; browser `fetch` remains only for the web demo.
  The last good index still caches in `localStorage`. `site/worker.js` adds
  `Access-Control-Allow-Origin: *` for `/skins/*` (GET/HEAD/OPTIONS) — local
  code only until the site is deployed.
- **Pack state** — source changes are invisible until the packs are rebuilt and
  reinstalled. Local 1.0.2 zips are in `site/public/skins/`; the live index is
  at 1.0.2 after the 2026-10-27 pack publish (the live host was not re-checked
  in the #107 closeout). `skinShopEntries` compares versions numerically; a local
  pack newer than the index is never offered as a downgrade.
- **Comic legibility** — `--cp-yellow-ink:#080a0d`, `--cp-cyan-ink:#03231f`,
  `--cp-purple-ink:#ffffff`; every yellow/cyan background uses its ink pair
  (the three 1.09:1 spots are gone). Data/numeric readouts use
  `--cp-readable-font` at >=12px; display faces remain only on hero/module
  headings, settings section heads and workshop detail titles. Signal/Ink also
  lock data and form text to the readable stack while keeping their brush /
  post display faces.
- **Icon and material layers** — 26 original 16px SVGs live in
  `skins/comic-pixel/assets/icons/pixel/` and are used as pack-local CSS masks
  (inlined by Rust into the served CSS) for the rail, settings rail, card tools
  and workshop actions. Hero cards keep 3px frames + hard shadow; data cards
  use 1px soft lines; controls keep the sticker frame; states get side bars.
  Signal uses postage frames; Ink uses paper slips/brush marks.
- **Transitions** — `src/skinTransitionPlan.ts` maps skin → timeline
  (comic pixel grid 22px, signal stamp, ink blobs) and enforces a 350–650ms
  budget; `src/skinTransition.ts` runs one GSAP overlay, applies once,
  interrupts cleanly and falls back to an instant switch under
  `reduceAnimations`/`prefers-reduced-motion`. Wired to the Settings and
  workshop entries only.

## Skin workshop storefront (#107, 2026-10-27)

- **One entry, one surface** — the workshop is a Settings page
  (`#st-skins`, General → Panel skins). The old popover drawer market and its
  CSS/entry are gone. The popover palette writes
  `panelIntent = {view:"settings", page:"skins"}`; the panel applies it once and
  persists the applied stamp (`pane.panelIntent.applied`) so a failed config
  clear cannot replay the jump in a new window. The General page also carries a
  “Panel skins / Open workshop” row.
- **Data contract** — `skinShopEntries()` merges native + installed packs +
  distribution index. `applied` comes from the loaded pack; `selected` comes
  from `config.skinId`. A selected pack that is missing locally shows
  Selected / Not installed, native is not reported as applied, and the gallery
  explains it. A local pack named `native` cannot collide with the native row
  (`__native` sentinel).
- **Previews** — every thumbnail, hero and detail shot is the real panel
  component rendered at 360px and clipped into a phone frame; previews never
  read live accounts and remote skins show a download-to-preview placeholder.
  No ratings, downloads, authors or invented metrics.
- **Failure semantics** — `refreshInstalledPacks()` returns a boolean and keeps
  the last good registry; failed reads surface `skin.shopRegistryFailed` and
  must not call `applySkin` (which would repaint native). `refreshSkinIndex()`
  is single-flight (`skinIndexRefreshing()`); the shop shows checking/offline
  states. Download/remove errors stay visible in the gallery, not only in the
  detail page. Skin actions persist with `patchConfig(..., {markDirty:false})`,
  so they never arm the Settings “Save” button.
- **Cross-window** — install/remove broadcasts `pane.skins.packs` through
  localStorage; the other window re-reads the registry before applying. The
  site demo routes the host overlay through `?settings=1&page=skins`; verified
  in a real browser (main demo → settings overlay → 4 cards).
- **Evidence** — `tests/skin-shop.test.mjs` (17 cases: states, ordering,
  escaping, missing selection, download/remove success and failure, index
  loading/offline). Full Node suite **237 passed**; site suite **50 passed**;
  `pnpm build` and `cargo check` pass. Real window 1059×768 verified gallery,
  grid, search, empty state, detail, zoom + Esc, apply/restore, Save staying
  disabled, and the popover palette entry. Not re-tested: real pack
  download/remove UI, 860×620, English/Russian visuals, reduced motion.

## Settings round 2, usage topology and per-source cache revisions (2026-10-26)

- **Settings rail** — the panel nav is a 4-group tree (General / Data &
  allowances / Network & privacy / About) with `grid-template-rows: 0fr→1fr`
  folding, `aria-expanded`, localStorage memory and ArrowUp/Down/Left/Right
  navigation. The owner approved absorbing the window-form radio group into a
  segmented pill; all select-backed pills and the new pills share a sliding
  thumb (underline for existing select pills, filled thumb for `.set-pill`).
- **Appearance prefs** — text size is four steps (100/110/125/150%) on the
  existing window-scale multiplier; code font + weight apply only when set
  (`body.code-font-custom`, `--code-font`), and colorblind mode adds
  dash/pattern cues without changing hues. The settings panel emits
  `ui-prefs-changed` so the popover applies these immediately across webviews.
- **Usage topology (#60)** — a read-only page opened from the button beside the
  Quota Overview title. Rows come from `orderedSnapshots()` and each row's
  endpoints from `ProviderSpend.last30.models`; empty-model rows are hidden.
  `usage-updated` marks rows live for 12s; wires are hand-built S beziers into
  an SVG layer, recomputed by a ResizeObserver. Motion is CSS-only (hub
  breathing only while a row is live; flow lines while live) and stops under
  reduced motion. No routing, no configuration, no proxying.
- **Window token estimate (#59-02)** — vendor-reported token allowances show
  exact remaining tokens; otherwise a `≈` estimate appears only for windows
  >=24h whose start is covered by full local spend days, with `p` outside
  (1,99) or missing local tokens suppressing the line. Sub-day windows are
  deliberately not extrapolated.
- **Per-source parser revisions (#59-03)** — `PersistEntry` carries
  `source` + `parser_rev`; `parser_rev_for(source)` bumps one scanner family
  and drops only that family's cached files. `PERSIST_VERSION` remains for
  cross-source/cache-format changes. Entries without a source are re-parsed
  once.

## ZCode borrow (#64) and cockpit semantics (#63) (2026-10-26)

- **Usage stats page** — new settings page id `stats` (block `st-stats`; `usage`
  is still the keys page). It owns the 52-week token heatmap (daily / weekly /
  cumulative), the 365-day KPI strip, a self-drawn SVG top-6 model trend and
  donut, and the per-day provider/model detail. `renderSpendDetailOverlay`,
  `data-spend-details` buttons and `spendDetailOpen` are gone; the popover
  cards keep only share/fold. Pure chart rules live in `src/statsMath.ts` and
  are covered by `tests/stats-math.test.mjs`.
- **ZCode metrics** — `fetch_zcode_usage_metrics` reads `turn_usage` and
  `tool_usage` from `~/.zcode/cli/db/db.sqlite` over the same 32-day window as
  the model scan. Semantics match ZCode's Settings → Usage page: sessions =
  distinct session ids, longest session = max completed-turn duration sum,
  tool calls = `tool_usage` rows. Missing tables return `None` (KPI “—”).
- **Onboarding** — config `onboardingSeen` / `onboardingDismissed`; the
  three-step wizard only auto-opens from a fresh-install gate
  (`initialTourPending`) and replaces the old spotlight tour for new users.
  Settings → General has a replay entry. Escape and Skip record dismissed;
  neither path changes on replay.
- **Shortcut kernel** — `src/shortcutKernel.ts` (Apache-2.0 ZCode semantic
  attribution in `LICENSE`) owns canonicalization, platform-aware CmdOrCtrl,
  `event.code` fallback, IME/repeat filtering, reserved keys, conflicts,
  takeover and legacy migration. Config `shortcutBindings: Record<id, string[]>`
  is the new store; an explicit empty array means cleared. `localShortcuts`
  migrates once and remains only for legacy compatibility. A single capture
  dispatcher routes the command table; recording and onboarding short-circuit
  it.
- **Adaptive cadence** — `adaptive_interval` shortens the background loop to
  60 s when any healthy card is under 20% remaining or a reset is within
  30 minutes. Any active provider backoff suppresses acceleration. No session
  file activity probe (deliberate decision).
- **Cockpit mechanisms** — `identity_guard` (three-valued match + SHA-256
  fingerprint), `account_token_lock` (per-account double-check; no refresh
  worker exists in Pane yet), `retry_after_secs` (delta + HTTP-date, capped at
  1 h; claude and apikey quota paths write `retry_after_s=`). The auth center
  now shows cached quota on failed refreshes, locks rows while a flow runs,
  auto-dismisses success after 1.6 s, aggregates status into the row title and
  has a local-only identity mask. A discovery banner surfaces a changed local
  account identity from a folded baseline; it never imports credentials.
- **Cockpit runtime notes** — all provider HTTP status errors append
  `retry_after_s=` when the vendor sends the header. Quota responses in the
  field expose no account id; the identity guard's Missing branch is therefore
  the normal path, and Codex/Grok/Copilot fetches reload the account by id
  under their own account lock. Codex is the only browser callback flow Pane
  owns: its login dialog now accepts a pasted callback URL/path/query and
  validates `state`; the dual-stack advertising rule is unit-tested, while the
  vendor-registered redirect URIs remain `http://localhost:1455` and are not
  rewired. Pending OAuth state is memory-only, so disk quarantine is not
  applicable. Codex's `login_refresh_lock` already provides the per-account
  refresh lock + reload double-check; `account_token_lock.rs` generalizes it
  for future workers.

## Correctness, data and API sweep (#66–#76) (2026-10-26)

- **Claude windows** — `utilization` is normalized (0–100; only the open
  (0,1) legacy fraction is converted), >101/negative/non-finite is a text
  “Usage unavailable”, and an expired `resets_at` becomes “Window expired —
  awaiting refresh”. Pace returns unknown for an expired reset, and alerts
  check expiry before the 100% run-out shortcut. New tests live in
  `claude::window_tests`, `pace` and `alerts`.
- **Leak guard** — `Metric::progress` rejects >101 as text instead of clamping
  an epoch-scale leak into a fake 100%. Copilot's JSONL fallback now computes
  per-(session, model) deltas across resume shutdowns; the DB source is
  unchanged.
- **Pricing trust** — models.dev arbitration is (trust, score). `pricing.rs`
  carries a baked official-owner list, an optional supplement
  `modelsdev_trust` override, and re-parses the cached models.dev document when
  the trust set changes (bumping GENERATION + LAST_INGEST_MS).
- **Upstream status** — `providers/upstream_status.rs` probes only Claude's
  Statuspage endpoint for stale/error cards, 5-minute TTL, 30-minute last-good,
  6s timeout, no credentials; `Snapshot.upstream_status` feeds the stale
  tooltip. Other vendors need their own evidence before adding endpoints.
- **Burn curve** — `meter_samples.rs` keeps 45 days of 5-minute remaining-%
  samples per (card, metric), dedupes flat values, starts a new series when the
  window moves, and least-squares-fits an ETA only after 3 points / 1 hour.
  `fetch_meter_samples` feeds the per-card SVG.
- **Month projection** — `spend_history::project_month_from_series` uses the
  median of the last 7 effective days (> $0.004), excludes today from the
  remaining-day count, and is exposed via `fetch_spend_projection`; the range
  column shows it only for the cost metric.
- **/v1/limits** — `limits.rs` emits `openusage.limits.v1` with progress/usage/
  quota projections and errors; `/v1/usage` is byte-shape unchanged. Balance
  quotas are attached at the onenewapi/relaybalance/aihubmix parse points.
- **Skill usage** — `skill_usage.rs` scans Claude transcripts for Skill
  tool_use only, dedupes (message id, request), sums the row's token buckets
  and `costUSD`, never stores args, caches 5 minutes and exposes
  `fetch_skill_usage`; the General settings page renders the list.
- **WSL** — `wsl.rs` discovers distros (`wsl.exe -l -q` UTF-16LE) and their
  homes, returns native + `\\wsl.localhost\…` roots, and is wired into the
  Claude/Codex/Copilot scanners. `spend.wsl_mode` / `wslMode` defaults to both;
  native-only is available for scans that must not touch WSL.
- **API hygiene** — providers/status/doctor/sources carry `schemaVersion: 1`;
  provider rows carry `statusCode` (ok 200, no_credentials 401, needs_setup
  424, error 503); MCP tools advertise read-only annotations; a failed port
  6736 bind raises a system notification.
- **WDA** — `platform::set_capture_excluded` applies
  `WDA_EXCLUDEFROMCAPTURE` to main/panel while share detection is active,
  re-reads the setting each 3s tick and clears on stop.
- **Not done in this sweep** — #65 architecture remains at the FetchStrategy
  skeleton only (no family migration/gatekeeper/capability typing); #61-03/04
  auth avatars and rich help tips, #62-03/04/05 transactional settings/CI
  release gate/error struct, and #58-05 native screenshots remain open.
