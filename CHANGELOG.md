# Changelog

## Unreleased

### Changed
- **OpenCode Go meters say "this PC only"** — Go quotas are counted
  account-wide on OpenCode's servers, and there is no API to read
  them; the local meters can't see other devices or other participants
  on a shared subscription, and now say so instead of "local estimate".

## 0.4.18 — 2026-07-17

### Fixed
- **Update checks report the real app version** — Tauri's
  `{{current_version}}` URL template arrives percent-encoded and never
  substitutes in query strings, so 0.4.17 installs literally sent
  "currentversion". The endpoint URL is now built in Rust with the
  actual version stamped in.

## 0.4.17 — 2026-07-17

### Changed
- **Update checks go through pane.jazii.dev first** (GitHub stays as
  the automatic fallback, and every update remains signature-verified).
  The endpoint serves the same manifest and counts anonymous daily
  installs — distinct-install estimate, country code, app version, and
  nothing else; no IP addresses are stored. Full mechanics in
  docs/privacy.md ("The update check").

## 0.4.16 — 2026-07-16

### Added
- **Kimi K3 prices everywhere** — K3 usage through Devin, Cursor, the
  Kimi Code CLI, or the API now bills at Moonshot's published rates
  ($3 in / $15 out / $0.30 cache hit per MTok) via a built-in fallback,
  in every slug spelling the tools use. The live catalogs win
  automatically once they learn the model.

### Changed
- **"Others" thresholds scale with the period** — Today and Yesterday
  fold providers under $1 into the grey Others wedge; 30 Days folds
  under $5 (was a flat $10 everywhere, which swallowed most of a quiet
  day's ring). The hover breakdown states the active bar.

## 0.4.15 — 2026-07-16

### Added
- **Moonshot/Kimi spend** — the Moonshot card gains Today / Yesterday /
  30-day dollars and tokens, a per-model breakdown, and the usage trend,
  scanned from the Kimi Code CLI's local session logs (one usage record
  per turn). Models the price catalogs don't know yet count their
  tokens with the usual ⚠ until pricing lands.
- **Credit meters for pay-as-you-go cards** — Moonshot and DeepSeek get
  a "Credits used" percent bar metered against the highest balance Pane
  has seen (a top-up raises it automatically), so balance cards read
  like every other card — and the low-credit case now fires the same
  "Almost Out" notification the quota providers get.

### Fixed
- **Devin usage counts under the model that actually ran** — Devin
  rewrites a session's model label whenever you switch models, which
  retroactively relabeled (and mispriced) everything the session ran
  before. Each message's own recorded model now wins, so switching to
  Fable (or anything else) mid-session shows up correctly, priced at
  the right rates. Fable's Max mode slug also normalizes now.
- **Grok no longer shows the same meter twice** — the billing payload
  repeats one percentage under several keys; the card now keeps one
  row per label, and layouts saved while the duplicate existed repair
  themselves on the next refresh.

## 0.4.14 — 2026-07-16

### Changed
- **Small spenders fold into "Others"** — Total Spend providers under
  $10 in the visible period group into one grey wedge and legend row;
  hovering it lists exactly who spent what. Providers at $10 or more
  keep their own name and color. (A lone small spender stays named —
  a group of one is just a rename — and if everyone is under $10 the
  ring stays fully named rather than turning into one grey blob.)

### Fixed
- **Balance-only cards show their rows** — providers with no usage
  meters (Moonshot, DeepSeek, and other pay-as-you-go cards) tucked
  every row behind the "show more" caret, leaving an empty panel with
  a floating arrow. Their Balance/Voucher rows now stay visible, and
  already-saved layouts repair themselves on the next refresh.

## 0.4.13 — 2026-07-16

### Fixed
- **Cursor spend updates within minutes, not an hour** — the dashboard
  usage export was cached for a full hour, so a live Cursor session
  could work invisibly for up to 60 minutes while every other provider
  updated in minutes. The cache is now 5 minutes, and a failed refetch
  serves the last good export instead of blanking the Cursor spend
  rows entirely.

## 0.4.12 — 2026-07-16

### Added
- **MiniMax spend** — the MiniMax card now shows Today / Yesterday /
  30-day dollars and tokens like the other CLIs, fed by two local
  sources: the MiniMax Agent CLI's own per-turn usage store (its
  recorded cost is used as-is), and any Claude Code sessions run
  against MiniMax's Anthropic-compatible endpoint — that usage used to
  be counted (mislabeled) under the Claude card and now moves to the
  MiniMax card where it belongs.

## 0.4.11 — 2026-07-16

### Fixed
- **⚠ Outdated tooltips explain the problem and the fix** — hovering
  the warning now classifies what went wrong (sign-in expired, vendor
  rate limit, vendor outage, no connection) and says exactly what to
  do about it — including the right re-login command per provider —
  instead of showing a bare error code.
- **Total Spend always draws its ring** — a period with no usage now
  shows a quiet zeroed track with $0.00 in the center instead of
  collapsing to a bare "No spend in this period" line.
- **Dead Claude sign-in says what to do** — when another app rotates
  the Claude Code refresh token (leaving Pane's copy invalid), the card
  now says "run `claude` in a terminal once and Pane recovers
  automatically" instead of "token refresh failed: HTTP 400".

## 0.4.10 — 2026-07-14

### Fixed
- **Claude card recovers faster from rate-limit cooldowns** — when
  Anthropic's usage endpoint returns 429 (a plan change can trigger a
  ~25-minute cooldown), Pane now honors the vendor's own Retry-After
  timing instead of knocking every 5 minutes, and the card's note says
  how long the wait is.
- **Codex subagent replays no longer inflate spend** — when Codex spawns
  a subagent (or forks a session), the child's rollout file replays the
  parent's entire token history with fresh timestamps. Pane counted
  those replayed lines as real usage; they're now skipped via the log's
  own markers (the Mac app shipped the same fix after a ~20x inflation
  report). Re-emitted stale snapshots are skipped too, and turns that
  only report cumulative totals are recovered as deltas.
- **Codex fast/priority turns price at fast rates** — the service tier
  is read per session from the rollout's `thread_settings_applied`
  lines (never from `config.toml`, which would retroactively reprice
  history when toggled) and applies each model's Codex priority
  multiplier (GPT-5.5 ×2.5; GPT-5.4 and the GPT-5.6 family ×2).
  Supported Codex models switch to OpenAI's long-context rates above
  272k prompt tokens — the OpenAI boundary, not Anthropic's 200k.
- **Claude advisor usage counts under the advisor's model** — Fable-era
  logs nest advisor work in `usage.iterations`; advisor-message entries
  now count once under their own model without double-counting the
  parent totals. `<synthetic>` placeholder turns are never priced,
  sidechain logs replaying a parent message under a fresh request id
  are deduplicated, and persisted `claude -p` runs count like
  interactive usage.
- **OpenCode free-model usage shows up** — messages on free models
  record a real cost of $0 with real token counts; those tokens now
  appear in the token totals and Usage Trend instead of vanishing.

## 0.4.9 — 2026-07-11

### Added
- **Cost/MTok** — the Total Spend ring's metric now cycles Cost →
  Cost/MTok → Tokens on click (right-click cycles backward). The center
  shows your true blended rate — total dollars over total megatokens —
  and each legend row shows that provider's own $/MTok.
- Cursor's spend rows say **"estimated"** — its usage export aggregates
  requests, so per-request exactness isn't possible.
- Reset countdowns inside a minute read **"Resets soon"** instead of a
  dying timer.

### Fixed
- **Long-context requests price correctly** — models with 1M-token
  context bill the *whole* request at a higher tier once the prompt
  crosses 200k tokens; Pane now applies those tiers. Claude's 1-hour
  cache writes bill at twice the input rate (they were priced as
  ordinary writes before), and Claude fast-mode requests apply the
  published fast multiplier. Spend histories reprice automatically —
  expect Claude's 30-day figure to correct upward.
- **Codex percentages show as reported** — the old "fresh window
  reads 1%, call it 0" normalization masked real early usage and is
  gone (the Mac dropped it too). "Not started" now keys on the window
  still being full-length, and windows under 5% used never flash a
  red pace projection off a floored reading.

## 0.4.8 — 2026-07-11

### Fixed
- **The whole model-family surface prices now** — reasoning-effort
  tiers (light/low/medium/high/xhigh), Max **and Ultra** modes, the
  fast tier, and any composition of them ("gpt-5.6-sol-max-fast",
  "…-ultra-high", "…-max-fast-xhigh") resolve to the base model's
  rates, with fast keeping its real per-family multiplier in every
  composition. Previously composed slugs fell into the unpriced ⚠
  bucket — on the test machine that was ~$19 of one day's Cursor
  Max-fast usage hiding from the totals. Verified by a 51-slug
  regression matrix across GPT-5.6 Luna/Terra/Sol; the handling is
  generic, so other families (Grok fast tiers etc.) get the same
  guarantees.
- **"Outdated" stopped crying wolf** — a single failed refresh no
  longer tags every card; data under three minutes old serves
  silently, and the amber tag (with the real error on hover) appears
  only when staleness is real. Persistent failures surface exactly as
  before.

## 0.4.7 — 2026-07-10

### Added
- **Devin spend** — the Devin CLI's local session store now feeds the
  Total Spend donut, spend rows, per-model breakdown, and usage trend,
  priced with the live catalog like the other CLIs. Windsurf-style
  model names ("gpt-5-6-sol-max") are normalized so they price, and
  the store is read through SQLite's backup API so numbers stay
  correct while the Devin app is actively writing. Cloud Devin
  sessions bill in ACUs and keep no local logs, so only CLI usage
  appears.
- **Dollars ⇄ tokens** — click the Total Spend ring (or right-click)
  to flip the donut, legend, and center total between money and raw
  token counts; the choice persists.
- **Reorder without leaving the popover** — every card grows a drag
  grip in its header; drop it where you want and the order saves to
  the same layout Customize edits.

### Changed
- **The popover looks like the Mac's now** — provider cards are a
  clean header over an inset panel, the usage trend sits in a labeled
  row, and the Total Spend ring is rebuilt from true wedge segments:
  radial-cut ends with soft corners, hairline gaps, and tiny spenders
  that stay thin slivers instead of swelling into dots. Hovering a
  wedge (or its legend row) slides it outward and dims the rest.
- **Spend colors** — Codex blue, Grok green, Devin sky blue, and
  Cursor its brand black (flipped to white in dark mode so it stays
  visible).
- **Share cards** — the copied image is a curated composition: buttons,
  links, and spend chrome stripped, the canvas hugs the content, the
  header aligns with the panel's text column, and the footer carries
  the app icon with the full tagline.
- **Unpriced usage keeps its tokens** — requests on models with no
  public pricing now count their measured tokens in token totals and
  the trend; dollars still refuse to guess, and the ⚠ (on the
  provider's spend row only) explains it in plain words.

### Fixed
- **Grok spend works again** — the Grok CLI changed its log format and
  the old scanner silently matched nothing; the new one reads token
  counts from the CLI's turn events and attributes models per process,
  like the Mac app.
- **Cursor Max-mode models price correctly** — "-max" slugs bill
  token-based at the base model's rates, so they now resolve through
  the full pricing chain instead of landing in the unpriced bucket.
- **Kilo fresh accounts** — a just-created account shows a friendly
  "no credits yet" card instead of an error.

## 0.4.6 — 2026-07-09

### Fixed
- **Cursor spend tiles work again** — Cursor's usage-events export now
  requires an explicit date range and token strategy; Pane sends both
  (last 31 days), so Today / Yesterday / 30 Days dollars populate.
- **New models price within the hour, not within the day** — when spend
  events reference models the price catalog doesn't know yet (new Cursor
  slugs ship often), Pane now rechecks the catalog hourly instead of
  daily, and a catalog update re-prices already-scanned logs instead of
  waiting for them to change on disk.

## 0.4.5 — 2026-07-09

### Fixed
- **Cursor Pro/Pro+/Ultra/Teams accounts now show real usage** — percent
  of the plan's included usage, Auto/API usage, on-demand spend, and
  credits, with the actual billing-cycle reset date, via the same API
  Cursor's own dashboard uses. Previously modern accounts showed only a
  meaningless "Requests this cycle: 0" from the legacy request-counter
  era (old request-based plans still fall back to it). Session tokens
  are auto-refreshed in memory when stale, and reading Cursor's login
  no longer fails when Cursor briefly holds a lock on its database.

## 0.4.4 — 2026-07-08

### Performance
- Background refreshes no longer rebuild the popover interface while
  it's hidden in the tray (~99% of the time) — rendering now happens
  once, at the moment you open Pane. Same for the 30-second countdown
  ticks. Less idle CPU, all day.
- New **Settings → General → "Liquid glass effects"** toggle (on by
  default). Turn it off on slower PCs: the glass refraction and blurs
  become clean flat surfaces, and the expensive lens machinery never
  even initializes — from the very first frame of a cold start.

## 0.4.3 — 2026-07-08

### New
- **Pane's new logo** 🎯 — the ring, everywhere: installer, app and
  taskbar icons, tray, popover sidebar, and share-card footers.
- **Update checks on every open** — the footer version stamp re-checks
  each time you open Pane and becomes a blue **⬆ Update** button when a
  new release is out; one click installs and restarts. (Replaces the
  floating update banner.)
- Party mode is now a triple-click on the sidebar logo away. 🎉

### Fixed
- Tray strip pairs now render logo-then-numbers, left to right, like
  the macOS original (Windows inserts new tray icons leftward).
- The update flow can no longer freeze at "Installing…" if a release
  disappears mid-install — it fails visibly with a retry button.

## 0.4.2 — 2026-07-08

**⚠ Installs of 0.4.1 and earlier: this release is signed with a new
key, so the in-app updater will decline it. Reinstall once via
`irm https://pane.jazii.dev/install.ps1 | iex` — updates then resume
normally.**

### Changed (security)
- **Breaking for browser-page consumers:** the local HTTP API no longer
  sends CORS headers, so web pages can no longer read
  `127.0.0.1:6736` through a browser — previously any website you visited
  could silently read your usage data. PowerShell, curl, Rainmeter, and
  native apps are unaffected (CORS only constrains browsers). If a
  legitimate browser integration needs access, open an issue — the plan
  is an opt-in origin allowlist, not a permissive default.
- Release binaries are now built and published by GitHub Actions from the
  pushed tag (public build logs), instead of on the maintainer's machine.
- The updater signing key was rotated to a passphrase-protected key
  (2026-07-08). Installs of 0.4.1 and earlier trust the old public key, so
  their auto-updater will decline the first release signed with the new
  key — reinstall once via `irm https://pane.jazii.dev/install.ps1 | iex`
  (or the release installer) and updates resume normally.
- **Webview hardening pass** (from a community security review): a strict
  Content-Security-Policy replaces the previous `csp: null`; the Tauri
  capability set is trimmed to core IPC only (the UI never used the
  opener/updater/process plugin APIs); `withGlobalTauri` is off; the
  pinned-metric dropdown is built via DOM instead of HTML strings.
- **Rust-side input validation:** `set_config` only accepts known config
  keys; tray-strip updates only accept known provider ids (also fixes
  unstarred tray icons of newer providers not being removed); pricing
  supplement alias rules are size- and count-capped.
- **Credential safety:** CLI credential files are copied to `*.pane-bak`
  before Pane writes a refreshed token back.

### Added
- SECURITY.md (private vulnerability reporting), docs/privacy.md (every
  network call), docs/providers.md (per-provider: files read + endpoints
  called), docs/local-http-api.md, CONTRIBUTING.md.

### Fixed
- Share cards (⧉) now copy the card exactly as it looks on screen —
  donut, tabs, theme and all — framed with a Pane logo footer, instead
  of a simplified redrawn version that didn't match the UI.

## 0.4.1 — 2026-07-08

First-run and Customize fixes from fresh-install testing.

### Fixed
- Fresh installs now start with just Claude + Codex enabled (their
  "connect me" cards are the onboarding); a PC with zero detected AI
  tools no longer enables all 18 providers.
- Rapidly toggling several providers off kept only the last change —
  toggles now apply instantly and save through a serial delta queue.
- Disabled providers disappear immediately from the dashboard, the
  Total Spend donut, and the tray strip (previously they lingered until
  the next refresh).
- Total Spend shows a quiet "No spend data yet" card on machines whose
  CLIs haven't logged usage, instead of no card at all.

## 0.4.0 — 2026-07-07

### New
- **Three CLI-detected providers** (research credit: steipete/CodexBar, MIT):
  Codebuff (credits + weekly limit, `codebuff login` file or key), Kilo
  (credit blocks + Kilo Pass, CLI login file or key), and Kiro
  (experimental — reads `kiro-cli /usage`). **18 providers total.**
- **Auto-updater** — Pane checks GitHub releases on launch and every 4
  hours; a banner offers one-click download + restart. Updates are
  cryptographically signed and verified before install.
- **Deeper metric rows** (Mac-parity polish): Claude per-model weeklies
  from Anthropic's new `limits` API (Fable era) + Extra Usage overage
  dollars; Codex Spark / Spark Weekly windows + Extra Usage credit
  balance; Z.ai monthly Web Searches quota; Grok pay-as-you-go cap badge.
- **"Not started"** — untouched 5-hour session windows say so (with an
  explainer) instead of showing a countdown that hasn't begun; Codex's
  floored 1%-on-fresh-window quirk is normalized to a true zero.
- **Keyboard** — Esc backs out of Customize/Settings, Ctrl+R refreshes.

### Removed
- Deepgram, OpenAI, Venice, Poe, Chutes, Warp, Crof, Amp, Vertex AI, and
  AWS Bedrock providers — cut to keep the lineup focused on the AI coding
  tools people actually track. Saved layouts self-clean any retired ids.

### Fixed
- Terminal windows no longer flash during refreshes (provider CLI checks
  now run windowless or scan the filesystem instead of spawning `cmd`).
- Retired providers no longer linger as ghost rows in Customize.
- Startup crash at higher provider counts (fetch futures now heap-boxed).
- Clicking the tray icon always reopens on the main page, even if the app
  was left on Settings or Customize.

## 0.3.0

### Renamed to Pane
- The app is now **Pane** (formerly OpenUsage for Windows). Installs to
  `%LOCALAPPDATA%\Pane`; settings move automatically from
  `%APPDATA%\OpenUsage` to `%APPDATA%\Pane` on first launch — keys, layout,
  and caches all carry over.

### Accuracy (Wave 9)
- **Live model pricing** — per-model rates now come from LiteLLM, models.dev,
  and the OpenUsage pricing supplement (daily refresh, ETag caching, offline
  fallback) instead of a hardcoded table. Claude spend was overstated ~2.6×
  at old Opus rates; Codex fast-tier requests now get their real multiplier.
- **Unpriced events are excluded, not guessed** — models no catalog prices
  are left out of totals and flagged with ⚠ (count + model names on hover).
- **Cursor spend** — computed from the dashboard's usage-events CSV export,
  priced locally.
- **Codex dedupe** — archived session copies no longer double-count.
- **Backoff & cooldown** — failing providers are benched 60s (5 min for
  rate limits) while cached data is served with the reason on hover.
- **Reset all layouts** also re-detects installed AI tools.
- **Codex reset credits** — each banked credit shows its exact expiry and a
  Use button that redeems it (confirm-guarded, idempotent).
- Single-instance guard; popover reopens scrolled to the top.

### UI
- Auto-hiding liquid-glass sidebar (prasen.dev lens: SDF rim refraction,
  chromatic fringe) with magnetic minimap trail; glass footer with build
  stamp; full-window Customize and accordion Settings panels; ☀/☾ theme
  toggle with circular wipe; per-day trend tooltips; skeleton loading,
  staggered card entrances, and a full light-mode audit.

### New
- **Wave 11 provider pack** — seven providers that authenticate with a pasted
  API key (Settings → API keys) or nothing at all:
  DeepSeek (balance), Moonshot/Kimi (balance, .ai + .cn), ElevenLabs
  (character quota with reset pacing), Deepgram (project balances),
  OpenAI (org costs — needs an Admin key), Venice (USD/DIEM/VCU balances),
  and Ollama (local server: installed + loaded models, no key needed).
  Providers added by an update that have no credentials on this PC start
  disabled — enable them in Customize.
- **MiniMax provider** — Coding/Token Plan quota (5-hour Session + Weekly
  windows) via the same endpoint the official `mmx quota` command uses. Key
  auto-detected from the MiniMax CLI's config, `MINIMAX_API_KEY`, or the new
  Settings field.
- **Copilot CLI / modern gh detection** — GitHub tokens are now also read
  from Windows Credential Manager (`gh:github.com:<user>`), which is where
  current gh versions (and the Copilot CLI) keep them. hosts.yml-only setups
  keep working.
- **Motion pass** — cards slide in with a stagger and bars fill when the
  popover opens, skeleton shimmer while first data loads, hover elevation on
  cards, smooth caret/tab/button transitions. All entrance animations play
  only on open (never on background refreshes) and respect the system's
  reduced-motion setting.

### Fixed
- config.json parsing now tolerates a UTF-8 BOM and logs parse failures
  instead of silently resetting settings to defaults.

## 0.2.0 — 2026-07-07

Full feature parity with the macOS original, plus Windows-specific polish.

### New
- **Antigravity support** — reads quota from the IDE's local language server
  when it's running, and falls back to Google's Cloud Code API (token from
  Windows Credential Manager) when it isn't. Session / Weekly / Claude /
  Claude Weekly metrics plus plan name.
- **Provider quick links** — Status / Dashboard links at the bottom of each
  card, same targets as the Mac app.
- **Share cards** — hover a card and click ⧉ to copy it as a PNG image to the
  clipboard (works for Total Spend too).
- **Local HTTP API** — `GET http://127.0.0.1:6736/v1/usage` (and
  `/v1/usage/:providerId`) serves the latest snapshots in the Mac app's
  documented wire format. Scripts written for the Mac app work unchanged.
- **Appearance** — System / Light / Dark theme setting.
- **Compact layout** — tighter density option.
- **Global shortcut** — e.g. `Ctrl+Shift+U` to toggle the popover from
  anywhere.
- **Proxy** — optional `socks5://` / `http(s)://` outbound proxy.
- **First-launch detection** — a fresh install starts with only the providers
  that have credentials on the PC; the rest wait in Customize.

### Fixed
- The popover no longer sits on "Loading usage data…" while the spend engine
  scans session logs on a cold start — usage cards paint immediately and the
  Total Spend card fills in when the scan finishes.
- Last-good snapshots are now cached **on disk**, so a transient provider
  outage (or rate limit) right after an app restart shows amber "⚠ Outdated"
  data instead of an error card. Entries expire after 24 hours.
- Drag-and-drop in Customize works (Tauri's native drag interceptor disabled).

## 0.1.0 — 2026-07-06

First Windows release: 10 providers (Claude, Codex, Cursor, OpenCode,
Copilot, Grok, Devin, OpenRouter, Z.ai, Antigravity detection), local spend
engine with model breakdown and 30-day trend, pace projections, toast
notifications, tray strip with per-provider icons, Customize screen, NSIS
installer, autostart.
