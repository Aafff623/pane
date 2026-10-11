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

Last reconciled: 2026-10-11 (Pane 0.7.0).

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
- Version 0.7.0 (`package.json` + `src-tauri/tauri.conf.json`), identifier
  `com.jazii.pane`, productName `Pane`.
- Privacy boundary (asserted by tests in the code): tokens go only to their
  own vendor's API; pasted keys live in `%APPDATA%\Pane`; the 6736 HTTP API is
  loopback-only, no CORS, Host-checked, and redacts One/New API origins and
  secrets; telemetry is anonymous, opt-out, and must never affect the app
  (every failure path is silent; disabling deletes the state file).

## Key code paths and relationships

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
  `["rlib"]` (desktop only). Unit tests run from repo-root `parse-tests/`
  (673 passing, 13 ignored as of 2026-10-11; run with
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
- **Provider/account keys in the OS vault (2026-10-08)** — the 22 providers'
  API keys and every multi-account key now live in the OS credential store
  via `keyring` (`secretstore.rs`, service "Pane"; entry names
  `provider:<family>` / `account:<card-id>` / `archived:<card-id>` on the
  stable `<family>@<fnv1a>` ids). JSON files keep only non-secret fields;
  reads lazily migrate old plaintext — the vault write must succeed before
  the field leaves the file, and a failed vault write keeps the plaintext
  file as the fallback. `reveal_provider_key` / `reveal_account_key` demand
  the master password (same canary + session as the vault; "查看" in the
  panel) — the password gates human viewing, background refresh never needs
  it. `change_master_password` verifies the old password first. The
  parse-tests harness swaps in an in-memory store via the `harness` feature.
  Provider-specific credential files (onenewapi sites, OAuth stores) are NOT
  migrated yet.

## Update mechanism

- Signed with this project's own minisign key (private key in the
  `TAURI_SIGNING_PRIVATE_KEY` repo secret; local backup in
  `temp/secrets/` — losing it kills update signing). The only endpoint is
  this repo's GitHub Releases `latest.json`; the detached upstream's
  trypane.xyz/ItsJazii endpoints were dropped. New versions raise an in-app
  popup (`maybePromptUpdate`, once per version per run) beside the footer
  update button. Since 2026-10-10 the updater follows the app-wide proxy
  chain (config `proxy.url` → Windows system proxy,
  `providers::outbound_proxy_url`) and caps the manifest check at 20 s —
  the plugin's reqwest is built without the system-proxy feature, so
  system-proxy users previously updated over a direct github.com link.
  Transport failures render the localized `update.networkHint`. Channel
  details: `docs/update-mechanism.md`.

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
- Feature sections embed three screenshots from `attachments/*.png` (quota
  overview, model spend, daily-use heatmap), captured 2026-10-10 from the
  rebuilt demo frontend with demo data.
- README banners and screenshot strips were removed on 2026-10-04 so the
  document stays searchable, lightweight, and accurate as the dashboard UI
  changes. Product proof now lives in the feature sections and linked design
  documents.

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
