# Providers: exactly what Pane reads and calls

One section per provider: which credentials are read from your PC, which
endpoints they are sent to, and what comes back. Each provider's code
lives in [`src-tauri/src/providers/`](../src-tauri/src/providers/) in a
file of the same name — this page is the plain-English version of that
code.

Ground rules that apply to every provider:

- A credential is only ever sent to **its own vendor's API**, over HTTPS.
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
  Claude.

## Cursor

- **Reads:** Cursor's local state database
  (`%APPDATA%\Cursor\User\globalStorage\state.vscdb` — copied before
  reading, never modified).
- **Calls:** `cursor.com` / `api2.cursor.sh` usage APIs; the dashboard's
  usage-events CSV export (for spend).
- **Shows:** credits, usage meters, plan; per-day spend.

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
  `/v1/settings`, and `/v1/user?include=subscription`; `auth.x.ai`
  token refresh (written back).
- **Plan:** prefers `subscription_tier_display` from settings, then maps
  `subscriptionTier` from the user endpoint. Plan lookup failures do not
  hide otherwise valid usage data.
- **Shows:** subscription plan, weekly pool, pay-as-you-go cap badge;
  local spend from `~\.grok\logs\`.

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

## DeepSeek / Moonshot / ElevenLabs / Venice-class key providers

- **Reads:** pasted key or env var only (`DEEPSEEK_API_KEY`,
  `MOONSHOT_API_KEY`/`KIMI_API_KEY`, `ELEVENLABS_API_KEY`).
- **Calls:** `api.deepseek.com/user/balance`;
  `api.moonshot.ai|cn/v1/users/me/balance`;
  `api.elevenlabs.io/v1/user/subscription`.
- **Shows:** balances / character quota with reset pacing; Moonshot and
  DeepSeek add a "Credits used" percent bar metered against the highest
  balance Pane has seen locally (top-ups raise it; feeds the Almost Out
  notification).

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
- **Shows:** Session (5-hour) and Weekly bars with reset pacing, plan
  name (Andante / Moderato / Allegretto when the weekly quota matches).
  The **API** bar (credits used vs the highest balance Pane has seen)
  only appears when a Moonshot/Kimi API key is saved — plan-only
  installs never get a third quota row. Balance/Vouchers sit behind
  Show more on that bar. Local spend from
  `~\.kimi-code\sessions\**\wire.jsonl` (one usage.record per turn).
  The separate Moonshot card is hidden while this card is connected.
  Switching Moonshot off in Customize still skips the wallet fetch (no
  API bar, no `api.moonshot.ai|cn` call). If the Kimi card is off,
  local session spend stays on Moonshot.

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
