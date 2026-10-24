# Providers: exactly what Pane reads and calls

One section per provider: which credentials are read from your PC, which
endpoints they are sent to, and what comes back. Each provider's code
lives in [`src-tauri/src/providers/`](../src-tauri/src/providers/) in a
file of the same name — this page is the plain-English version of that
code.

Ground rules that apply to every provider:

- A credential is only ever sent to **its own vendor's API**, over HTTPS
  (One/New API sites and Custom Balance relays may use `http://` only with
  private, loopback, or link-local IP addresses).
- If no credential is found, the provider shows a "connect me" hint (new
  installs auto-disable everything undetected except Claude and Codex).
- Expired OAuth tokens are refreshed against the vendor's own token
  endpoint and written back to the CLI's credential file, keeping the CLI
  signed in — identical to what the CLI does itself.

---

## Tencent Cloud OrcaTerm

- **Reads:** `%APPDATA%\com.orcaterm-desktop.app\data.bin`, the official
  desktop client's local session. Pane never writes this file.
- **Calls:** `https://api.orcaterm.com/cgi/api` with
  `DescribeUserBillingStatus`; `OAuthRefreshToken` only when the token
  expires within 60 seconds. Refreshed tokens stay in memory and are
  discarded when the disk token changes.
- **Shows:** the returned plan, AI request quota and monthly reset,
  storage in GB, credential count and shared sessions (`-1` means
  Unlimited). There are no fixed plan limits and no automatic claims.
- **Unverified:** whether refreshing a still-valid token invalidates the
  desktop client's token. The owner must test this before release.
- **Icon:** a local terminal symbol, not an official OrcaTerm brand asset.

## Claude (Claude Code)

- **Reads:** `%USERPROFILE%\.claude\.credentials.json` (honors
  `CLAUDE_CONFIG_DIR`) — the OAuth token Claude Code saved when you logged
  in. **Multi-account:** dot-folders in your home directory and dirs under
  `%USERPROFILE%\.config` holding a Claude-shaped `.credentials.json` each
  become their own card, identified by the `oauthAccount` in that dir's
  `.claude.json` (a dir that can't name its account is skipped; the same
  account in two places stays one card). Extra cards are named from the
  account's organization or email; the default login keeps the plain
  `claude` id. Each account's spend comes from its own dir's logs.
- **Calls:** `api.anthropic.com/api/oauth/usage` (usage windows);
  `platform.claude.com/v1/oauth/token` (refresh, written back).
- **Shows:** Session + Weekly windows, per-model weeklies, Extra Usage
  overage; local spend from `~\.claude\projects\` logs. Persisted
  `claude -p` runs count too (`--no-session-persistence` runs write no
  log to read). Advisor work nested in a message's `usage.iterations`
  counts once under the advisor's own model; ordinary iterations stay
  inside the parent totals. Sidechain (subagent) logs that replay the
  parent's message under a fresh request id are deduplicated. Sessions
  of the pi coding agent that drove this Claude account
  (`~\.pi\agent\sessions`, providers `anthropic`/`claude-agent-sdk`)
  fold into this card's spend — pi's own recorded cost when present,
  catalog pricing otherwise.

## Codex (Codex CLI)

- **Reads:** `%USERPROFILE%\.codex\auth.json` (honors `CODEX_HOME`).
  **Multi-account:** same discovery as Claude — dot-folders and
  `~\.config` dirs each become their own card, but only when their
  `auth.json` *proves* it's an OpenAI login (its `id_token` carries
  OpenAI's own claim namespace) — another app's credential file that
  merely looks similar is never picked up. Cards are identified by
  `tokens.account_id` (or the id_token's ChatGPT account claim) and
  named by the account email; a dir that can't name its account is
  skipped. Pane also merges its own web-OAuth logins
  (`%APPDATA%\Pane\codex-accounts.json`) and `auth.json.pane-bak`
  backups by ChatGPT account id, so the desktop config and web logins
  appear as one card without double-counting; a backup whose token is
  dead shows an error row instead of disappearing. Reset-credit
  redemption always uses the account whose card offered the credit.
- **Calls:** `chatgpt.com/backend-api/wham/usage` (limits, Spark windows,
  credits); `.../wham/rate-limit-reset-credits` (reset credits, and
  `/consume` only when you click Use on a credit); OpenAI token refresh.
- **Shows:** Session/Weekly, Spark windows, credit balance, redeemable
  reset credits; local spend from `~\.codex\sessions\` logs. Child
  sessions (subagent spawns and forks) replay the parent's entire token
  history at spawn — those replayed lines are skipped, so subagent-heavy
  use doesn't inflate spend. Turns that ran on the fast/priority service
  tier (recorded per session in the rollout itself, never inferred from
  `config.toml`) price at each model's Codex priority multiplier, and
  supported GPT-5.4/5.5/5.6 requests above 272k prompt tokens use
  OpenAI's long-context rates for the whole request. Auto-review usage
  keeps the name `codex-auto-review` in the model breakdown; dollars use
  the dated GPT fallback for that day (gpt-5.5 from April 2026 onward).
  Daybreak Blue (`gpt-daybreak-blue-latest`) prices as GPT-5.6 Sol.
  Pi coding agent sessions that drove this Codex account (provider
  `openai-codex`) fold into this card's spend the same way they do for
  Claude. Turns logged as `kimi-oauth/…` or `moonshot-ai/…` (a router
  pointed at the Kimi plan) move to the Kimi Code card instead.

## Cursor

- **Reads:** Cursor's local state database
  (`%APPDATA%\Cursor\User\globalStorage\state.vscdb` — copied before
  reading, never modified).
- **Calls:** `api2.cursor.sh` Connect RPCs (`GetCurrentPeriodUsage`,
  `GetPlanInfo`, `GetCreditGrantsBalance`); the dashboard's
  usage-events CSV export (for spend). `GetCreditGrantsBalance` cents
  fields may be strings or numbers. When the RPC host is unreachable or
  hides `planUsage` (Enterprise/team), `cursor.com/api/usage-summary`
  (web session cookie) supplies the same plan figures; the pre-2025
  request-count endpoint `cursor.com/api/usage` is the last resort and
  only counts when it reports a real quota.
- **Shows:** Cursor Models / Other Models bars; a **Credits** progress
  row when the account has promo grants (`totalCents` vs remaining);
  a **Bonus** text row behind Show more when `planUsage.bonusSpend` is
  reported — free provider-sponsored usage, with the pool estimate
  (derived from `totalPercentUsed`) as context when it is sane; Total
  usage text on bucket-era personal plans; per-day spend.

## OpenCode (Go plan)

- **Reads:** the Go key from
  `%USERPROFILE%\.local\share\opencode\auth.json`, or a Go key pasted in
  Settings (`OPENCODE_GO_API_KEY` works too) for machines without the
  OpenCode editor installed; the auth.json entry stays preferred when
  both exist.
  `%USERPROFILE%\.local\share\opencode\opencode.db` (copied before
  reading) for spend — message costs your own OpenCode history already
  contains.
- **Calls:** `opencode.ai/zen/go/v1/usage` (the official account-wide
  usage API, shipped in anomalyco/opencode#16513) — Session / Weekly /
  Monthly percentages and resets counted on OpenCode's servers, the
  same numbers the Zen dashboard shows, so usage from your other
  devices and shared-subscription participants is included. If the API
  is unreachable, meters fall back to the old local computation from
  `opencode.db` (rolling 5-hour session, UTC Monday-start week, monthly
  cycle anchored to your first-ever Go usage) — the fallback counts
  this PC only. Dollar spend rows are always computed locally.

## GitHub Copilot

- **Reads:** gh CLI / Copilot tokens from Windows Credential Manager
  (`gh:github.com:<user>`) or legacy `hosts.yml` files — in every source,
  only the `github.com` entry; a GitHub Enterprise token sharing the
  file is never selected (this card only ever talks to api.github.com).
- **Calls:** `api.github.com/copilot_internal/user`.
- **Shows:** credits/quota and plan.

## Grok (Grok CLI)

- **Reads:** `%USERPROFILE%\.grok\auth.json`.
- **Calls:** `cli-chat-proxy.grok.com/v1/billing`,
  `/v1/settings`, and `/v1/user?include=subscription`;
  `grok.com/prod_mc_billing.ConsumerUiSvc/GetRemainingResets`;
  `auth.x.ai` token refresh (written back).
- **Plan:** prefers `subscription_tier_display` from settings, then maps
  `subscriptionTier` from the user endpoint. Plan lookup failures do not
  hide otherwise valid usage data.
- **Shows:** subscription plan, weekly pool, pay-as-you-go cap badge,
  remaining reset credits; local spend from `~\.grok\logs\`.

## Devin (Devin CLI)

- **Reads:** `%APPDATA%\devin\credentials.toml`;
  `%APPDATA%\devin\cli\sessions.db` (+ WAL/SHM sidecars, copied before
  reading) for local spend.
- **Calls:** Devin's `GetUserStatus` RPC.
- **Shows:** weekly/daily quota, extra balance, plan; local spend from
  Devin CLI sessions (cloud Devin sessions bill ACUs and keep no local
  logs, so they can't be priced).

## MiniMax

- **Reads:** pasted key (Settings), `MINIMAX_API_KEY`, or
  `%USERPROFILE%\.minimax\config.yaml` (exactly
  `provider.minimax.options.apiKey` — a same-named key under another
  provider's section is never used); local spend from
  `%USERPROFILE%\.minimax\sqlite.db` (the Agent CLI's per-turn
  token_usage table, snapshotted via SQLite's backup API — never
  modified) and from Claude Code sessions that ran against MiniMax's
  Anthropic-compatible endpoint (those log MiniMax models into
  `~\.claude\projects\` and are re-routed here from the Claude card).
- **Calls:** `api.minimax.io/v1/token_plan/remains` (+ regional fallbacks).
- **Shows:** 5-hour Session + Weekly plan windows; Today / Yesterday /
  30-day spend with per-model breakdown (the CLI's own cost_usd is
  preferred; catalog pricing otherwise).

## OpenRouter

- **Reads:** pasted key, `OPENROUTER_API_KEY`, or the key OpenCode stores.
- **Calls:** `openrouter.ai/api/v1/credits` and `/key`.
- **Shows:** balance, credits meter, key limit.

## Z.ai (GLM Coding Plan)

- **Reads:** pasted key (Settings → **Z.ai / GLM**), `ZAI_API_KEY` /
  `GLM_API_KEY`, or the Z.ai CLI's key file. A GLM Coding Plan key from
  `open.bigmodel.cn` works here too — no CLI login needed.
- **Calls:** `api.z.ai` quota + subscription endpoints.
- **Shows:** Session (5-hour rolling window) + Weekly plan quotas with
  reset times, monthly Web Searches quota, plan. The two token windows
  arrive as look-alike `TOKENS_LIMIT` rows told apart by their `unit`
  field; an idle session window reports no reset — it only starts
  counting on first use, so it shows as "not started". Older
  subscriptions report a single token window and show just the Session
  row.


## Antigravity

- **Reads:** the running IDE's local language server (loopback), or the
  `gemini:antigravity` token in Windows Credential Manager.
- **Calls:** the local language-server RPC when the IDE runs; otherwise
  Google's Cloud Code quota API (`cloudcode-pa.googleapis.com`) with
  Google's own token refresh.
- **Shows:** Gemini + Claude pool windows, plan.

## DeepSeek / Kimi API / ElevenLabs / Venice-class key providers

- **Reads:** pasted key or env var only (`DEEPSEEK_API_KEY`,
  `MOONSHOT_API_KEY`/`KIMI_API_KEY`, `ELEVENLABS_API_KEY`).
  The Customize toggle and Settings field are labeled **Kimi API**;
  the internal id is still `moonshot`.
- **Calls:** `api.deepseek.com/user/balance`;
  `api.moonshot.ai|cn/v1/users/me/balance`;
  `api.elevenlabs.io/v1/user/subscription`.
- **Shows:** balances / character quota with reset pacing; Kimi API and
  DeepSeek add a "Credits used" percent bar metered against the highest
  balance Pane has seen locally (top-ups raise it; feeds the Almost Out
  notification). Saving a key in Settings turns that provider on if
  Customize had it off.

## APIGOTO (RouterCode)

- **Reads:** a pasted `sk-` API key from Pane's credential store, or
  `APIGOTO_API_KEY`. The key is used only for the vendor's read-only
  usage endpoint — Pane never sends it to a model.
- **Calls:** `GET https://api.apigoto.com/v1/usage` (Bearer). No
  chat/completions or other model traffic, so the free-tier `40204`
  model gate is never exercised or bypassed.
- **Shows:** Plan, the subscription's largest window (RPMCredits
  `used`/`limit` and `window_sec`), the USD `remaining` equivalent and
  the subscription expiry. When the subscription is absent or expired
  the card shows `none (payg)` / `expired` plus the balance or points;
  it never paints a 0% healthy bar from payg fallback numbers.
- **Quotas:** RouterCode Free is about 25M TCredits/week with a 100M
  total; RouterCode PLUS is about 100M/week with a 400M total
  (¥29.90/month). `/v1/usage` returns only the largest window, not the
  monthly total, so Pane deliberately omits a total it cannot verify.
  Plan name, limit, window and expiry all come from the response.
- **Errors:** 401 means the key was rejected; 503 is a transient config
  store failure and the last-good snapshot stays visible.

## Kimi Code

- **Reads:** `%USERPROFILE%\.kimi-code\credentials\kimi-code.json` (honors
  `KIMI_CODE_HOME`; falls back to `~\.kimi\credentials\kimi-code.json`).
  That is the official CLI's OAuth login. Refresh tokens rotate on use and
  are written back beside the CLI's file (`*.pane-bak` first), same as
  Claude/Codex. **No CLI?** Paste your Kimi For Coding plan key in
  Settings → API keys → **Kimi Code** (stored in `%APPDATA%\Pane\kimi.json`,
  no env var is read). The login stays preferred when both exist; the key
  is the fallback, and it also takes over automatically when a CLI login
  has been rotated (dead refresh token) so the card recovers without a
  manual `kimi login`. This is the plan key, not the platform.kimi.ai
  wallet key — that one goes in the **Kimi API** field.
- **Calls:** `api.kimi.com/coding/v1/usages` (Session + Weekly request
  windows, sent the OAuth token or the pasted plan key as Bearer);
  `auth.kimi.com/api/oauth/token` (refresh, login only); and, when a
  Moonshot/Kimi API key is saved, `api.moonshot.ai|cn/v1/users/me/balance`
  for the API bar. This is the Kimi Code *subscription* plus the
  pay-as-you-go wallet on the same card.
- **Monthly-cap probe:** the usages endpoint never reports the plan's
  monthly limit, so every refresh also sends one `max_tokens=8`
  completion (`api.kimi.com/coding/v1/messages`, model `k3` — ~9
  tokens). `max_tokens` must stay at 2+: the server skips the quota
  pre-check entirely for `max_tokens=1` requests (live-verified).
  When the rejection names the monthly limit (English "monthly", or any
  "月" for CN accounts), the card grows a maxed Monthly row; the reset
  instant is parsed from the error text (ISO-8601, `YYYY-MM-DD HH:MM`,
  or a relative "in N days/hours/minutes"). While the wall stands the
  real probe re-runs at most every 30 minutes (the cached red row shows
  the next check time on the card). Rate limits, auth failures, and
  network hiccups are never mistaken for the monthly wall, and a probe
  failure never blanks the card.
- **Shows:** Session (5-hour) and Weekly bars with reset pacing, plus the
  membership plan name from `user.membership.level` (falls back to plain
  "Kimi Coding" when the API omits the level, as happens on the API-key
  path). A maxed monthly limit pins a red 100% Monthly row and folds the
  card until the parsed reset instant. Names match
  [kimi.ai/membership/pricing](https://www.kimi.ai/membership/pricing):
  Moderato ($19), Allegretto ($39), Allegro ($99), Vivace ($199).

  | API `user.membership.level` | Card name |
  |---|---|
  | `LEVEL_STANDARD` / `LEVEL_MODERATO` | Moderato |
  | `LEVEL_INTERMEDIATE` | Allegretto |
  | `LEVEL_ADVANCED` | Allegro |
  | `LEVEL_PREMIUM` | Vivace |

  Older codes still map if the API sends them (`LEVEL_FREE` /
  `LEVEL_BASIC` → Adagio, `LEVEL_ANDANTE` → Andante). Weekly request
  caps 1024 / 2048 / 7168 are a fallback only, when the API omits the
  level.
  Those two windows are the same clocks as Kimi's website "5-hour Code"
  and "7-day Code" rows; the coding usage API currently reports remaining
  as whole percents (`limit`/`remaining` of 100), so usage under 1% can
  still show as 0% in Pane. The website's **Total usage** bar is a
  separate monthly membership credit pool (Kimi chat + Code) and is not
  on this card. The **API** bar (credits used vs the highest balance Pane
  has seen) only appears when a Moonshot/Kimi API key is saved — plan-only
  installs never get a third quota row. Balance/Vouchers sit behind
  Show more on that bar. Local spend from
  `~\.kimi-code\sessions\**\wire.jsonl` (one usage.record per turn),
  priced at Moonshot's published API rates (K3 $3/$15/$0.30 cache;
  K2.7 Code $0.95/$4/$0.19, HighSpeed 2×; K2.6 $0.95/$4/$0.16;
  K2.5 $0.60/$3/$0.10; V1 8k/32k/128k at $0.20/$2, $1/$3, $2/$5).
  Plan logs name K3 `k3` / `kimi-code/k3` and K2.7 Code `kimi-for-coding`;
  those map to the same cards as `kimi-k3` and `kimi-k2.7-code`.
  Codex (and Claude) sessions that log `kimi-oauth/…` or `moonshot-ai/…`
  via a router move those spend rows here, prefix peeled, so they do not
  stay on the Codex/Claude card.
  The leftover Kimi API (Moonshot) card is hidden while this card is
  connected. Switching **Kimi API** off in Customize still skips the
  wallet fetch (no API bar, no `api.moonshot.ai|cn` call). If the Kimi
  card is off, local session spend stays on moonshot.

## Hermes (Nous Research desktop)

- **Reads:** the Hermes desktop app's local ledger
  (`%LOCALAPPDATA%\hermes\state.db`, `session_model_usage` table — model,
  billing route, token buckets, and the app's own cost per session).
  Detected when that file exists; no API key.
- **Calls:** nothing. This is a purely local source — Hermes records
  ZERO cost itself, so dollars are priced from Pane's shared catalog.
- **Shows:** a card with the two most recent user-selected models, their
  billing backends (AihubMix, MiniMax, a custom URL, …), and session count.
  Hermes's internal title/approval tasks still count toward real usage but
  do not replace that model summary. Today / Yesterday / Last 30 Days spend
  (with a per-model breakdown on hover) sits behind Show more.
  MiniMax-routed sessions join the MiniMax spend slice; OpenRouter-routed
  sessions join OpenRouter (including custom URLs pointed at those hosts).
  AihubMix and other custom OpenAI-compatible URLs stay on Hermes. Scoped
  AihubMix pricing covers HY4 Preview, Qwen3.8 Flash, and the dated
  Qwen3.8-Max-0902 spellings without leaking those gateway rates to other
  routes.

## Ollama

- **Reads:** nothing.
- **Calls:** your own PC only — `127.0.0.1:11434` (`/api/version`,
  `/api/tags`, `/api/ps`).
- **Shows:** installed models, loaded models.

## Codebuff

- **Reads:** `%USERPROFILE%\.config\manicode\credentials.json` (the
  `codebuff login` file) or a pasted key.
- **Calls:** `codebuff.com/api/v1/usage` + `/api/user/subscription`.
- **Shows:** credits, weekly limit, plan.

## Kilo

- **Reads:** `%USERPROFILE%\.local\share\kilo\auth.json` or a pasted key.
- **Calls:** `app.kilo.ai/api/trpc/user.getCreditBlocks,kiloPass.getState`.
- **Shows:** credit blocks, Kilo Pass window, tier.

## AihubMix

- **Reads:** pasted key (Settings), `AIHUBMIX_API_KEY`, or the `aihubmix`
  key OpenCode stores in its own `auth.json` (AihubMix is typically used
  through OpenCode as an OpenAI-compatible gateway).
- **Calls:** `aihubmix.com/v1/dashboard/billing/subscription` (spending
  limit) and `/usage` (month-to-date usage).
- **Shows:** usage metered against your account's spending limit, plan.
  Requests routed through OpenCode also appear in the Total Spend donut
  from OpenCode's local log, same as any other OpenCode model. Claude
  Code sessions pointed at AihubMix's Anthropic-compatible endpoint
  (qwen-family models in `~\.claude\projects\` logs, matched
  case-insensitively) are re-routed here from the Claude card, the same
  way MiniMax-routed sessions are. Claude Code logs don't record which
  gateway served a request, so this assumes qwen models reached Claude
  Code via AihubMix — sessions run through Alibaba's own
  Anthropic-compatible proxy would land here too.

## One/New API

- **Reads:** nested `%APPDATA%\Pane\onenewapi.json` (versioned
  `{ version, sites: [{ id, name, baseUrl, displayUnit, keys }] }`) — **not**
  `config.json`, and not the single-key `set_api_key` Settings list.
  Written atomically and stored owner-only (Windows protected DACL /
  Unix `0600`), not inherited from a permissive parent. Settings
  manages a hierarchy of sites and keys; pasted secrets never
  come back out of the UI after save.
- **Discovery:** `GET {origin}/api/status` with no API key, no redirects,
  and a bounded JSON body — only when creating a site or changing its
  base URL. Accepts branding-agnostic OneAPI / NewAPI payloads whose
  `success` is `true` and whose `data.version` or `data.system_name` is
  a non-empty string. Display unit (`USD` / `CNY` / tokens) is not
  required. Custom branding text is not required. HTTP 404 or a
  structural mismatch is shown as not a compatible OneAPI / NewAPI
  panel, not as a raw HTTP status.
- **Calls (per enabled key):** `GET {origin}/v1/dashboard/billing/subscription`
  and `GET {origin}/v1/dashboard/billing/usage` with
  `Authorization: Bearer <that key>` and **no date query parameters**.
  Do not call `/api/usage/token` or `/api/log/token`.
- **HTTP:** `https://` for any valid host. Plain `http://` is limited to
  IPv4 private/loopback/link-local addresses and IPv6
  loopback/unique-local/link-local addresses. Every HTTP hostname,
  including `localhost` and `*.local`, is rejected without DNS resolution.
- **Shows:** one dashboard/tray card per key, id `onenewapi@<key-id>`,
  titled `<site name> · <key label>`. Usage `{used} of {limit}` using the
  site's display unit from `/api/status` (`USD` → `$`, `CNY` → `¥`,
  tokens as integer counts, OneAPI `display_in_currency: false` as
  tokens). `total_usage` is always cents-style (`/ 100`) in every mode,
  matching NewAPI/OneAPI's OpenAI-compatible billing handlers. Plus an
  Expiry row when `access_until` is a positive timestamp. Empty sites
  produce no card and spawn no billing requests. Sentinel `>= 100000000`
  on `hard_limit_usd` / `system_hard_limit_usd` means unlimited: the
  card shows Used (including `$0.00` / `¥0.00` / `0` when usage is
  missing) instead of a fake percent bar. Display unit is stored on the
  site at fingerprint time; existing sites missing it get one
  unauthenticated status backfill.
- **AihubMix** stays its own family; an `aihubmix.com` origin may also
  be added here as a manual site — both cards can coexist and are not
  folded into Total Spend.
- **Shared quota:** some panels return the same user-level numbers for
  every key of one account. Pane shows each key's response independently
  and does not sum or dedupe equal values.
- **Accuracy:** OneAPI's `DisplayTokenStatEnabled=false` can hide
  token-level stats so the panel reports user-level quota instead; treat
  the card as what that credential observed. Pane does not call native
  `/api/usage/token` or `/api/log/token` to compensate.
- **Telemetry:** family `onenewapi` at most once per refresh. Site ids,
  key ids, origins, labels, and configured key counts never leave the
  machine.
- **Local HTTP:** each key is `GET /v1/usage/onenewapi@<key-id>`. The
  JSON is `providerId`, `displayName`, `plan`, `lines`, `fetchedAt` —
  no dashboard URL, origin, or secrets.

## StepFun

- **Reads:** pasted key (Settings) or `STEPFUN_API_KEY`.
- **Calls:** `api.stepfun.com/v1/accounts` (`api.stepfun.ai` as the
  fallback host) with the Bearer key.
- **Shows:** CNY balance, with the cash/voucher split as text rows when
  the account reports it. Adds a "Credits used" percent bar metered
  against the highest balance Pane has seen locally (top-ups raise it;
  feeds the Almost Out notification).

## StepFun Step Plan

- **Full mode:** account + password (Pane renews the 30-minute Oasis
  session) or a pasted Oasis session-token pair. Reads the console
  `QueryStepPlanRateLimit` / `GetStepPlanStatus` RPCs and shows the real
  window or Credit pool — the only way to see the remaining balance.
  The Credit row prefers the exact `credit_buckets` total/residual and
  the subscription expiry over the percentage field, so the expanded
  card says e.g. `6.66B of 8.00B credits left` and counts down to the
  expiry instead of showing a 1970 reset.
- **Limited API-key mode:** the provider is a web-session integration and
  the console RPC rejects an ordinary/Step Plan API key. Selecting
  **API Key** in Settings → StepFun Step Plan (with an explicit warning)
  stores the key instead and marks the snapshot `limited:
  "api_key_estimate"`.
  - No balance or progress/quota row is drawn.
  - Pane monitors local token usage only and shows the usage trend.
  - A Credits figure is estimated from the official StepFun CNY prices
    (1M Credits = ¥1; Step 5 Preview 7/0.7/20, Step 3.7 Flash
    1.35/0.27/8.1, Step 3.5 Flash 0.7/0.14/2.1 per 1M input/cache/output
    tokens). Because local rows lack the input/output/cache split, the
    estimate assumes 70% input / 30% output and charges input at the
    cache-miss rate; cache hits and Step 5 peak hours (2×) are not
    itemised. The row is always badged **estimated**.
  - A question-mark next to the plan explains why the balance is missing
    and that web login is required for exact quota/reset.
  - The two channels coexist in one credential file. Saving an API key
    keeps the stored web session; saving a web login keeps the key. The
    card uses the web session whenever it can read the exact quota and
    falls back to the API-key estimate only when no web session is
    stored or it has expired. Older `{apiKey}` files are migrated to the
    key slot on load, so they can never be mistaken for a session pair.
- **Credential hint:** the same snapshot shape supports future
  web-session providers: return `limited` from the provider and the card
  renders only the trend + help; no frontend per-provider branch.

## SiliconFlow

- **Reads:** pasted key (Settings) or `SILICONFLOW_API_KEY`.
- **Calls:** `api.siliconflow.cn/v1/user/info` (`api.siliconflow.com` as
  the fallback host) with the Bearer key.
- **Shows:** CNY balance (`data.totalBalance`), with the top-up-only
  portion (`chargeBalance`) as a text row. Same "Credits used" bar as
  StepFun.

## Novita AI

- **Reads:** pasted key (Settings) or `NOVITA_API_KEY`.
- **Calls:** `api.novita.ai/v3/user/balance` with the Bearer key.
- **Shows:** USD balance — the API reports `availableBalance` in units
  of 0.0001 USD, so Pane divides by 10000 before displaying. Same
  "Credits used" bar as StepFun.

## Custom Balance (relaybalance)

- **Reads:** a base URL and API key pasted together in Settings
  (Custom balance section), stored in
  `%APPDATA%\Pane\relaybalance.json` as `{"apiKey", "baseUrl"}`.
- **Calls:** `{base}/v1/dashboard/billing/subscription` and `/usage`
  (the form without `/v1` is tried as a fallback — relays exist in both
  flavors) with the Bearer key. Works with any relay/gateway that
  exposes the OpenAI dashboard-billing endpoints (DMXAPI, PackyCode,
  Micu, CrazyRouter, SudoCode.chat, XycAi, E-FlowCode, CherryIN,
  AICodeWith, …).
- **Shows:** usage metered against the account's `hard_limit_usd`
  spending cap, plus the remaining balance (`total_usage` is reported
  in cents and divided by 100, same convention as AihubMix).

## Qwen Code (Alibaba Coding Plan)

- **Reads:** pasted key (Settings), `BAILIAN_TOKEN_PLAN_API_KEY` (the env
  var Qwen Code itself uses), or `DASHSCOPE_API_KEY`; local spend from
  the CLI's own per-request ledger
  (`%USERPROFILE%\.qwen\usage\token-usage-YYYY-MM.jsonl`).
- **Calls:** the Model Studio console's Coding Plan RPC
  (`modelstudio.console.alibabacloud.com/data/api.json`, China-console
  fallback) — the same call the Coding Plan page makes; Alibaba publishes
  no dedicated quota API (approach credited to CodexBar's notes).
- **Shows:** the plan's three request-counted windows — rolling 5-hour
  session, weekly, monthly — with resets and plan name. If the console
  RPC rejects the key, the card falls back to local request/token counts
  for today and the month. Spend rows and the donut slice come from the
  local ledger either way.

## Qoder CN

- **Reads:** `%APPDATA%\com.qodercn.app.stable\auth.v1.dat` — the Qoder CN
  desktop app's sign-in, a Chromium `os_crypt` v10 blob (AES-256-GCM)
  whose key is DPAPI-wrapped in the sibling `Local State`
  (`os_crypt.encrypted_key`). Read-only: Pane never refreshes or writes
  the token, so it cannot sign the IDE out.
- **Calls:** `openapi.qoder.com.cn` (the CN OpenAPI — the global
  `openapi.qoder.sh` rejects CN tokens): `GET /api/v2/user/plan`,
  `GET /api/v2/quota/usage`, both with the session token as Bearer.
- **Shows:** plan tier (e.g. Pro) and the credit pool
  (`userQuota` total/used with its expiry), plus one row per active
  dedicated package (e.g. Qwen-only credits) with its own expiry. When
  the token expires (about a month), open Qoder CN once — its own refresh
  rewrites `auth.v1.dat` and Pane picks it up on the next cycle.

## Trae CN

- **Reads:** `%APPDATA%\Trae CN\User\globalStorage\storage.json`, key
  `iCubeAuthInfo://icube.cloudide` — the Trae CN IDE's sign-in blob,
  encrypted with ByteCrypto (AES-128-CBC; the per-value random key ships
  inside the blob). Read-only on purpose: Trae rotates the refresh token
  while running, and racing that rotation would sign the IDE out.
- **Calls:** the host stored beside the token (CN: `https://api.trae.cn`):
  `POST /trae/api/v2/pay/ide_user_ent_usage` and
  `POST /trae/api/v2/pay/ide_user_pay_status`, both with the
  `Cloud-IDE-JWT` authorization header — the same calls the IDE's own
  credits page makes.
- **Shows:** one row per credit pack plus the account tier (Free/Pro).
  The summed "Credits" row (`usage_summary` consumed/total) stays reset-
  free — packs expire on different days. Each pack renders separately,
  merged only with same-name same-scope siblings: the pack currently
  being drained (the only one carrying `usage.credits_amount`) is a
  progress row with a countdown; untouched packs are text rows
  "N credits · expires YYYY-MM-DD", soonest expiry first. Scope mirrors
  the official dashboard via `available_endpoint` (0 = general
  TraeCode/TraeWork credits, 1 = TraeWork-only, suffixed "(Work)").
  Feature-flag-only packs (e.g. 免费, no `credits_limit`) are skipped.
  Expired token → open Trae CN once and refresh.

## CodeBuddy family (CodeBuddy / CodeBuddy CN / WorkBuddy / WorkBuddy AI)

Four members of one Tencent CodeBuddy account system (one Keycloak realm,
one plugin billing gateway). They differ in **host**, the `platform`
handshake parameter, and — for WorkBuddy AI — the billing path prefix and
the desktop auth file. The family table in `codebuddy_login::FAMILIES` is
the single source of those rows:

| Card | Host | `platform` | Desktop auth file | Data root |
|---|---|---|---|---|
| CodeBuddy | `https://www.codebuddy.ai` | `ide` | — | — |
| CodeBuddy CN | `https://www.codebuddy.cn` | `ide` | — | — |
| WorkBuddy | `https://copilot.tencent.com` | `workbuddy` | `workbuddy-desktop.info` | `~/.workbuddy` |
| WorkBuddy AI | `https://www.workbuddy.ai` | `workbuddy-ai` | `workbuddy-desktop-ai.info` | `~/.workbuddy-ai` |

**WorkBuddy AI is not CodeBuddy International.** It is WorkBuddy's own
international build: a different product with its own OAuth platform, its
own billing prefix (`/billing/meter/...` first, `/v2/billing/meter/...`
only as a 404 fallback) and its own desktop sign-in file. CN and AI
credentials never cross: a scan whose token `domain` does not match the
family is refused before it is stored.

- **Reads:** the sign-in Pane manages itself in
  `login-accounts/<family>.json` (QR scan via the auth center). Each member
  becomes its own card (`family@<hash>`), so disabling one never disables
  another. WorkBuddy also reads two opt-in surfaces of the vendor's own
  apps — see the WorkBuddy-only routes below.
- **Calls:** `POST {host}/v2/plugin/auth/state?platform={platform}` to
  start the QR handshake, `GET {host}/v2/plugin/auth/token?state=…` to poll
  it, and `POST {host}/v2/plugin/auth/token/refresh` (refresh token in the
  `X-Refresh-Token` header, source in `X-Auth-Refresh-Source`: `ide-main`
  for the IDE builds, `plugin` for WorkBuddy — the AI build answers
  `invalid_grant` 12153 otherwise). Usage comes from the same host:
  `POST /v2/billing/meter/get-dosage-notify`,
  `POST /v2/billing/meter/get-payment-type` and
  `POST /v2/billing/meter/get-user-resource`. Every call carries a browser
  User-Agent: the billing gateway answers `403` / `code 10085` without one.
- **Shows:** a credits meter (used/total, countdown to the earliest future
  cycle end, rendered in UTC+8), the plan name, and package detail only
  where it adds information: a pack expiring within seven days gets its own
  row (at most three), and all remaining live packs collapse into **one**
  summary row (`13 packages · 2295 credits left · next expiry …`). A real
  account can hold a dozen near-identical packs, so one row per pack would
  bury the meter. Packs count as credits when `CapacityUnit` says so — and
  also when it is absent, since the older shapes carried no unit at all.
  Unlimited plans read "Unlimited"; with no total to meter against, the row
  degrades to plain credits left. Enterprise members get an empty resource
  list from the personal endpoint, so their usage is read from
  `POST /v2/billing/meter/get-enterprise-user-usage` — the same call the
  web dashboard's own `/profile/usage` page makes.
- **Provenance:** a reading that did not come from a Pane-managed login
  appends one `Source` row (`desktop app (read-only)` / `web session
  (explicit import)`). Pane login cards stay clean.

### WorkBuddy desktop sign-in (tier 1 — automatic)

- **Reads:** `%LOCALAPPDATA%\CodeBuddyExtension\Data\Public\auth\workbuddy-desktop.info`
  (CN) or `workbuddy-desktop-ai.info` (AI) — each variant reads only its own
  file, read-only. Fields the app re-encrypted into
  `{ "$wbEncrypted": 1, "envelope": … }` envelopes are refused, never
  decrypted — an enveloped sign-in simply shows the sign-in hint instead.
- **Calls:** the same billing gateway as the scanned route, with the
  desktop token.
- **Shows:** the same credits face as the scanned route (one shared row
  builder, `codebuddy::credits_face_metrics`) plus the `Source` row.
- **No import:** Pane does not copy this session into its own login store.
  The old "Import desktop sign-in" button and `workbuddy_import_desktop`
  command were removed on 2026-10-23; use the auth-center web login instead.
  The automatic tier-1 usage read above remains read-only.

### WorkBuddy web-session cookie (tier 3 — explicit opt-in)

- **Reads:** only after the user enables the channel and imports a cookie:
  Chrome's `Default/Network/Cookies` SQLite (copied to a temp snapshot that
  a `Drop` guard deletes on every exit path) + `Local State` for the
  DPAPI-wrapped `os_crypt` key — the shared `providers/chromium.rs` peel,
  also used by Doubao. Or a cookie pasted by hand. The channel is off by
  default and then makes **zero** requests. Store:
  `workbuddy_web.json` (cookie + detected Chrome major, never written
  without an import action).
- **Calls:** `POST https://www.workbuddy.cn/billing/meter/get-user-resource-summary`
  (root path, not under `/v2/plugin`) with a UA built from the local
  Chrome's major version; a rejection retries once with the previous major
  before reporting the session expired.
- **Shows:** the same credits face as the token route (spec D6 basis).

---

Provider request formats were researched from two MIT-licensed macOS
projects: [robinebers/openusage](https://github.com/robinebers/openusage)
and [steipete/CodexBar](https://github.com/steipete/CodexBar) — both
credited in [LICENSE](../LICENSE).

## 讯飞星火 (Spark-X2.6)

- **Reads:** Pane's isolated WebView2 login window for `maas.xfyun.cn`; the
  signed-in cookie jar is stored in `login-accounts/xfyun.json` plus the
  per-account OS-vault slot. Pane never reads a browser profile's
  SQLite/Cookies database, and there is no API-key input slot.
- **Calls:** `maas.xfyun.cn/api/v1/gpt-finetune/user` (login identity and
  `uid`), `/api/v1/gpt-finetune/api-services/free-quota?channel=spark-x2.6`
  (Spark-X2.6 free grant), and `/api/v1/gpt-finetune/api-services/usage`
  (last 7 days, text summary only).
- **Shows:** free-quota total / used / remaining / expiry and status;
  `claim=null` is a setup hint, not 0%; `EXHAUSTED` / `EXPIRED` keep their
  raw status; `code:4001` asks for a fresh web login. The real free-quota
  response is still awaiting the owner's first signed-in calibration, so
  unknown fields fail honestly instead of inventing numbers.

## 讯飞开放平台语音 (console.xfyun.cn speech packs)

- **Reads:** Pane's isolated incognito WebView2 on `console.xfyun.cn`; the
  cookie jar goes to `login-accounts/xfyun-speech.json` plus the OS vault, and
  the stable account id is the numeric `account_id` cookie (the value equals
  the message API's uid). `certification/get` also returns a uid but the same
  body carries name / ID-card / mobile, so that endpoint is never called for
  identity and no PII is ever stored or logged.
- **No official quota API exists.** The MaaS gateway answers 404 on
  `/v1|v2/usage|quota|balance|account`, so this reads the console's own
  dashboard endpoints: `user/profile/get` (session), `app/list` (enumerate
  every app), `meter/api-use-count/get`, `lfasr/getlfasrInfo`,
  `lfasr/get-rtasr-app-auth`. `Referer` / `Origin` / `User-Agent` do not
  affect the answer — only `Cookie` is required.
- **Envelope differs from Spark and must not be shared.** These endpoints
  always return HTTP 200; a logged-out session is `flag:false` +
  `code:80000`, which maps to "sign in again" and never to a 0% card.
  `code:20003` means we built the request wrong. `flag:true, data:null` is a
  normal "this channel has no such meter" state.
- **Params:** `app/list` takes exactly `currPage` / `keyword` / `type`;
  adding `pageSize` makes the console answer 20003. `app/default` is only the
  default app and must not stand in for the account. `serviceList` comes back
  `null`, so enabled services per app cannot be read — every channel is
  probed instead.
- **Remaining is the headline, and it matches the console.** Verified against
  the live page on 2026-10-23: the console's single "剩余服务量" is
  `left` (purchased pack remaining) **plus** `remainCount` (trial remaining) —
  `left 10000 + remainCount 500` rendered as 10500, and the page's
  "购买服务量" slot is empty. So the card shows that same sum and names the trial
  share (`10500 left · 500 trial`); it does not fabricate a percentage, because
  no total is published anywhere in these payloads ("cent" packs such as
  录音转写 do publish `count`, so only those keep a progress bar).
- **Units:** interaction counts map to plain numbers (`Requests`); 录音转写
  packs are hours → `Minutes` ×60; 实时转写 `leftCount` is **seconds** — shown as
  rounded minutes with the raw seconds in the text (never the 300.0166 decimal).
  `maxConnection` (lanes) is a separate capacity row, never a usage bar.
- **Three time encodings, all confirmed live:** `expireDate` is
  `YYYY-MM-DD HH:MM` (no zone), `dueDate` is `YYYY-MM-DD HH:MM:SS`, and
  `expireTime` is unix **seconds** (1824307199 ↔ 2027-10-23, matching the same
  pack's `dueDate`). Unparsable values yield no date at all rather than a
  guessed one; expiry never enters `resets_at` (one-off packs, not a periodic
  window).
- **Forbidden calls:** `webapi/getInfo/v1` and `app/getHttpSign` return the
  plaintext APIKey/APISecret. This provider does not need them and must never
  request them.
- **Calibrated against the live console on 2026-10-23** (read-only replay with
  the owner's own session): `tts`/`iat`/`ise` all `flag:true, code:0`,
  `usedCount 0`, `left 10000`, `threshold 500`, `remainCount 500`,
  `expireDate` 2027-01-21 (three separate claim times); `lfasr` one pack
  `count 5 / leftCount 5 / dueDate 2027-10-23`; `rtasr leftCount 18000` s,
  `maxConnection 1`, `expireTime` 2027-10-23. `lfasr`/`rta`/`rtasr`/`spark` on
  the count endpoint all answer `data:null`, as documented.
- **待确认:** `status` / `convertStatus` / `connectType` / `timeType` / `isDel`
  meanings; the timezone behind the naive strings; which pack is consumed first
  when several hour packs are live; whether multiple apps should be aggregated
  or listed per app. (The earlier handoff's `usedCount 9 / remainCount 491`
  reading no longer matches this account — the three packs now read 0/500,
  i.e. they were re-claimed or reset; that is account history, not an API
  change.)

## ZCode (Start Plan)

A separate provider from Z.ai above: Z.ai reads the official token plan with an
API key, while this card reads the **ZCode desktop client's account session**.
It manages only the **Start Plan gift quota** (the free/promotional buckets);
the paid Coding Plan belongs to Z.ai, not here.

The plan identity is always **Start Plan**; the campaign is a parenthetical so
a new promotion never renames the card (`Start Plan (1024 Dev Day)`,
`Start Plan (Weekend Build)`). The campaign is stripped from the vendor plan
name (`ZCode 1024 Dev Day` → `1024 Dev Day`); a plan whose entitlements carry a
`start-plan` id is recognized even when the campaign name is unknown. A Coding
Plan entry shares `plans[]` but is excluded by name/id and never becomes a gift
row.
  - **Reads:** `~/.zcode/v2/credentials.json` — the ZCode CLI's AES-256-GCM
    encrypted session JWT, decrypted locally; the first entry that decrypts to
    a JWT wins.
  - **Calls:** `zcode.z.ai/api/v1/zcode-plan/billing/balance`, which requires an
    `X-Device-Mid` header but only checks that it exists — Pane keeps its own
    UUID beside config.json.
  - **Payload:** `data.plans[]` (`name`, `status`, `starts_at`, `ends_at`,
    `entitlements[]` with `show_name` / `capabilities` / `grant_units` /
    `period: one_time` and `entitlement_id`) and `data.balances[]` mirroring
    each entitlement (`show_name`, `total_units`, `used_units`, `remaining_units`,
    `available_units`, `period_start`, `period_end`, `expires_at`, unix
    seconds). A bucket is paired to its plan by `plan_id` first — one account
    can hold several plans at once and they share `user_plan_id`, so matching
    on the user would fan every bucket out to every plan; the user id is only
    the fallback when a payload omits plan ids. A plan past `ends_at` is dead
    even while it still says `active`, and a bucket past its own window stops
    counting.
  - **Rows:** label = the bucket's `show_name` (so `GLM-5.3` and
    `GLM-5.3-Flash` stay distinguishable), bar = used/total, detail is the
    used/total only (the plan title already carries the campaign),
    expiring-soonest first, capped at six. A plan held without a granted
    bucket renders a "held · not granted (open ZCode to claim)" line; an active
    plan with no gift renders one "No gift in this period" row.
  - **Past campaigns stay visible:** an expired Start Plan campaign is not
    dropped. Its final bucket totals render as a history row
    (`Start Plan (Weekend Build)` · `expired · used of total used`) after the
    active rows. When only expired campaigns remain, the card is `needs_setup`
    (yellow) with those rows and a "gift has ended" hint — past gifts never
    masquerade as never granted.
  - **No Start Plan:** `needs_setup` with the plan title **Start Plan** and one
    row `Start Plan` → `No Start Plan` (zh: 没有), plus the sign-in/setup hint.
  - **Known campaigns so far** (names are evidence, never filters):
    - `ZCode Weekend Build` — the weekend token grant (measured 2026-10-12:
      `GLM-5.3-Flash`, 300,000,000).
    - `ZCode 1024 Dev Day` — 2026-10-23: `GLM-5.3-Flash` 300,000,000 plus
      `GLM-5.3` 200,000,000.
    - `ZCode Trust Build` — a gift window named in magpie's changelog; amount and
      dates 待确认.
    - model trials shipped with new models (e.g. `GLM-5.3-Trial` buckets).
  - **Caching:** rows are cached only for the session that fetched them (a
    hashed token tag) and for five minutes, so switching ZCode accounts or
    letting the session expire cannot paint one account's gifts on another's
    card.
  - **待确认:** the meaning of every `status` value and the timezone/unit of the
    naive timestamps; a gift whose `total_units` is 0 renders no row at all.

### WorkBuddy official request ledger (2026-10-24)

WorkBuddy CN's card adds one read-only text row, `Official requests (30d)`,
from `POST /billing/meter/get-user-request-usage` (30-day window, pageSize
3000, `startTime` advanced to the newest row). It reports the request count and
credit sum only; it is not a meter sample and is never added to the credits
balance or `credit_history`. Pagination, parsing and aggregation live in
`providers/workbuddy_official_usage.rs`. WorkBuddy AI was not probed, so its
card does not call this endpoint.
