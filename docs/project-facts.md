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

Last reconciled: 2026-10-22 (Pane 0.8.8 + account lifecycle, legacy API-key retirement, relay identity recovery).

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
- Version 0.8.8 (`package.json` + `src-tauri/tauri.conf.json`), identifier
  `com.jazii.pane`, productName `Pane`.
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
  secrets; telemetry is anonymous, opt-out, and must never affect the app
  (every failure path is silent; disabling deletes the state file).

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
  differenced so flat polling days do not count), and `current` (only when
  exactly one official-client local sign-in is visible).
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
  (959 passing in the latest serial proxy-free run, 14 ignored as of
  2026-10-22; run with
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
  `%LOCALAPPDATA%\com.jazii.pane\EBWebView` and restart.
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
  use the parse-tests harness instead of fighting the linker.

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
  never message text.
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
  chain (config `proxy.url` → Windows system proxy,
  `providers::outbound_proxy_url`) and caps the manifest check at 20 s —
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
- **Desktop sign-in import** (`workbuddy_import_desktop`): reads the
  variant's plaintext auth store, upserts by uid into Pane's login store
  (OS-vault-first) with `extra.sourceKind = "desktop"`, and never writes the
  vendor file. `$wbEncrypted` is reported, never decrypted. The desktop
  card is suppressed when a Pane login already owns the same uid, so one
  account cannot yield two snapshots with one card id.
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
  id plus the legacy `@__default__` alias, writes through `setAccountNote`
  to one canonical key, and falls back to label → email → localized
  `Account N` (never a fingerprint/masked token). Notes are single-line,
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
  `%APPDATA%\Pane\skins\<id>\`, and Rust serves each entry CSS with assets
  inlined as data URLs. `signal` / `ink` are ordinary packs: the workshop
  shows index entries as remote until installed, and a persisted shaped id
  survives boot (`readSkin`) while `migrateSkinFromIndex` downloads it.
  The index + zips must be deployed together before the flip; the demo's
  pack loading path is separate (issue 08). `node
  site/scripts/check-skin-packs.mjs [--live]` is the mechanical gate: it
  verifies local size/sha256 and, with `--live`, compares the deployed
  index (expected to report STALE until the release deploy). 0.8.8 still
  bundles signal/ink and `marketEntries` keeps bundled ids ahead of the
  index, so a stale deployed index cannot reach released users; it matters
  only for the unbundled build. Deploy from a clean tree — `site/` may
  carry another session's WIP.
- Deleting a One/New API key/site purges backend config/cache, and the
  frontend immediately drops the key card from `lastSnapshots`/`lastSpend`/
  `accountsCache` then refreshes accounts and broadcasts `accounts-updated`;
  waiting for the next usage refresh left stale account pills visible.
- Evidence: legacy-vault resurrection red/green, archive/delete repeated
  refresh and failed migration tests, relay retirement failure test, account
  alias regression, and a headless Customize routing check. Latest serial
  proxy-free Rust run: 959 passed/14 ignored after the backlog batch and
  backup hardening.

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
- The legacy single slot (`%APPDATA%\Pane\oauth\codex.json`) is imported
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

- Qoder 国际版 (family `qoder`) 的 Windows 配置目录按命名惯例取
  `com.qoder.app.stable`（CN 版为 `com.qodercn.app.stable`），备选 `Qoder`；
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

## Local panel/account validation (2026-10-21)

- Proxy clients (`http` / `http_no_redirect`) re-read the effective proxy
  and replace the current pool when its URL changes. Direct fallback keeps
  `no_proxy()`. A loopback mock-proxy regression checks both redirect modes;
  real system-proxy switching still belongs to owner acceptance.
- Settings tabs share a top-aligned reading column. Release history lives
  in Settings → About; the popover dashboard no longer contains a timeline.
- Managed login lifecycle commands share `account_lifecycle.rs`; the parsing
  harness exercises real persistence, archive/restore failure rollback,
  same-account re-login, and last-account deletion. This is not an end-to-end
  test of Tauri IPC or browser login.
