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
- Version 0.4.68 (`package.json` + `src-tauri/tauri.conf.json`), identifier
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
- **Trae CN credit packs** — `user_entitlement_pack_list` stacks several
  hard-expiry packs (verified 2026-09-17): `usage.credits_amount` exists
  ONLY on the pack currently being drained (others `{}`), so only that
  one can render as a progress row; `end_time` is epoch seconds.
  `entitlement_base_info.available_endpoint` 0/1 = general
  (TraeCode+TraeWork) vs TraeWork-only scope — same-name packs can
  differ in scope, so pack rows merge by display_desc+scope and Work
  rows get a " (Work)" label suffix. The summed "Credits" row carries
  NO resets_at (mixed expiries; a single date would misread) —
  expiry-as-reset on a merged row stays the convention only for
  single-expiry sums (Doubao earliest-expiry, Codex per-credit rows).
  The 免费 pack is feature flags (no `credits_limit`), not credits.
- **AccountEntry** — generic api-key account (`label`, `apiKey`, optional
  `baseUrl` for relaybalance), stored in `accounts/<family>.json`.
- **AgSlot** — Antigravity captured Google OAuth bundle (the IDE keeps one
  token in Windows Credential Manager; slots capture one account each).
  Identity = `refresh_token`. File: `antigravity-accounts.json`.
- **CursorAccount** — Cursor OAuth account. Identity = `access_token`.
  File: `cursor-accounts.json`.
- **Quota Overview (配额总览)** — pinned dashboard section aggregating every
  provider that has a rolling reset window (generalized from the initial 5-hour
  overview in commit `5d03a82`). Header badge = three disjoint chips
  (`● N 可用` green = off-peak available, `● N 满额` red shown only when
  non-zero, `● N 高峰` yellow = available AND inside the family's peak
  window, always rendered even at 0), plus a `5h` / `Weekly` capsule.
  The sectioned board splits the available set into 可用 / 高峰 / 不可用
  (peak section header always renders, even empty); the availability
  chips count the same predicates, so chips, dots and sections can never
  disagree. Default / 5h tab picks the most binding quota
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
- **Peak hours (高峰)** — time-of-day billing windows for 5 families,
  rules table + Beijing-time (UTC+8) checker in `src/peakHours.ts`
  (verified against official docs 2026-09-15): zai/linkso Mon–Fri
  14:00–18:00; commandcode Mon–Fri 09:00–12:00 & 14:00–18:00 (only
  DeepSeek-routed models; weekends never peak); qodercn & traecn daily
  08:00–22:00 (weekend daytime IS peak — both use a "daily" wording, no
  weekday/weekend split). A yellow status dot (.acct-dot/.overview-dot/
  .peak-dot/.trail-badge variants) appears ONLY when the family would
  otherwise read green; red (maxed/error) and gray never turn yellow.
  The overview tile tooltip appends the per-family multiplier rule
  (i18n `peak.rule.*`). Provider edges: doubao/kimi have NO time-of-day
  billing (rolling windows only); GLM/DeepSeek-official weekends are
  all off-peak while Qoder/Trae keep daytime standard rate.
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
- **Kimi monthly anchor** — Kimi's membership-wide monthly limit is never
  reported by the usage API; it only surfaces as failed inference. The
  wall refreshes on the **27th** (owner member-dashboard screenshot,
  2026-09-16; date precision only, modeled at local midnight) —
  `MONTHLY_RESET_DAY` in `providers/kimi.rs`. Probe states: healthy =
  one tiny probe per refresh (the only wall-discovery channel), walled
  with anchor in the future = zero probes (cached monthly row with
  countdown), anchor passed = 30-min re-probe rhythm until quota
  returns. A pane restart clears the memo and forces a fresh probe.
- **uiFont** — custom UI font family in `config.json` (`""` = stock
  stack). Enumerated by `fonts.rs` via font-kit (DirectWrite; localized
  names like 微软雅黑 appear as separate families), validated on write
  (trim, ≤100 chars), applied by `applyUiFont()` in `src/main.ts`
  prepending the family to the stock stack so missing glyphs fall back.
  The Settings dropdown is a body-level fixed menu (the settings
  accordion clips absolutely-positioned children), each option rendered
  in its own face.

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

- The README uses the existing product icon at `src/assets/pane-icon.png` as
  its only project-local visual. The icon is square, already shipped with the
  app, and should remain the canonical README identity mark.
- README banners and screenshot strips were removed on 2026-10-04 so the
  document stays searchable, lightweight, and accurate as the dashboard UI
  changes. Product proof now lives in the feature sections and linked design
  documents.

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

## Provider credential semantics (2026-10-03, three handoff providers)

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

## Search/MCP quota system (2026-10-03, searchquota + keyvault unification)

- **Key resolution**: the vault (`keyvault.json`) is the PRIMARY key source
  for bocha/tavily/firecrawl/brave — entries carry the user label shown on
  per-key rows; legacy sources (env, tavily-keys.json, ZCode MCP config)
  stay as deduped fallbacks. `keyvault_add/remove` calls
  `searchquota::invalidate_service` so the next tick refetches with the new
  key set (no 45-min cache stall).
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
  set would poison the day-diffing.
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

## UI repair facts (2026-10-04; context-menu anchoring added in 0.4.68)

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
  state. ClinePass uses the generated `users-three` Phosphor mark in
  `src/assets/providers/clinepass.svg`.
- Spend details render the selected period as a single active column; local
  shortcut settings cover six focused-window actions and reject duplicate
  bindings. Pooled-account count badges use blue informational styling.
