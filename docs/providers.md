# Providers: exactly what Pane reads and calls

One section per provider: which credentials are read from your PC, which
endpoints they are sent to, and what comes back. Each provider's code
lives in [`src-tauri/src/providers/`](../src-tauri/src/providers/) in a
file of the same name — this page is the plain-English version of that
code.

Ground rules that apply to every provider:

- A credential is only ever sent to **its own vendor's API**, over HTTPS
  (One/New API sites may use `http://` only with private, loopback, or
  link-local IP addresses).
- If no credential is found, the provider shows a "connect me" hint (new
  installs auto-disable everything undetected except Claude and Codex).
- Expired OAuth tokens are refreshed against the vendor's own token
  endpoint and written back to the CLI's credential file, keeping the CLI
  signed in — identical to what the CLI does itself.

---

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
  skipped. Reset-credit redemption always uses the
  account whose card offered the credit.
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
- **Calls:** `cursor.com` / `api2.cursor.sh` usage APIs; the dashboard's
  usage-events CSV export (for spend). `GetCreditGrantsBalance` cents
  fields may be strings or numbers.
- **Shows:** Cursor Models / Other Models bars; a **Credits** progress
  row when the account has promo grants (`totalCents` vs remaining);
  a **Bonus** text row behind Show more when `planUsage.bonusSpend` is
  reported — free provider-sponsored usage, with the pool estimate
  (derived from `totalPercentUsed`) as context when it is sane; Total
  usage text on bucket-era personal plans; per-day spend.

## OpenCode (Go plan)

- **Reads:** the Go key from
  `%USERPROFILE%\.local\share\opencode\auth.json`;
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

## Z.ai

- **Reads:** pasted key, env var, or the Z.ai CLI's key file.
- **Calls:** `api.z.ai` quota + subscription endpoints.
- **Shows:** Session/Weekly, monthly Web Searches quota, plan.

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

## Kimi Code

- **Reads:** `%USERPROFILE%\.kimi-code\credentials\kimi-code.json` (honors
  `KIMI_CODE_HOME`; falls back to `~\.kimi\credentials\kimi-code.json`).
  That is the official CLI's OAuth login — Pane never asks you to paste
  the plan token. Refresh tokens rotate on use and are written back
  beside the CLI's file (`*.pane-bak` first), same as Claude/Codex.
- **Calls:** `api.kimi.com/coding/v1/usages` (Session + Weekly request
  windows); `auth.kimi.com/api/oauth/token` (refresh); and, when a
  Moonshot/Kimi API key is saved, `api.moonshot.ai|cn/v1/users/me/balance`
  for the API bar. This is the Kimi Code *subscription* plus the
  pay-as-you-go wallet on the same card.
- **Shows:** Session (5-hour) and Weekly bars with reset pacing, plus the
  membership plan name from `user.membership.level`. Names match
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
- **Shows:** a card with last-used model, which backend billed it
  (AihubMix, MiniMax, a custom URL, …), and session count. Today /
  Yesterday / Last 30 Days spend (with a per-model breakdown on hover)
  sit behind Show more, same as other cards.
  MiniMax-routed sessions still join the MiniMax spend slice, OpenRouter-
  routed join OpenRouter (including a custom URL pointed at those hosts);
  AihubMix and other custom OpenAI-compatible URLs
  (including a custom URL that points at aihubmix.com) stay on this card.

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

---

Provider request formats were researched from two MIT-licensed macOS
projects: [robinebers/openusage](https://github.com/robinebers/openusage)
and [steipete/CodexBar](https://github.com/steipete/CodexBar) — both
credited in [LICENSE](../LICENSE).
