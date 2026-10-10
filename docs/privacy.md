# Privacy

Pane is built on one rule: **your data is nobody's business, including
ours.** There is no Pane account, and no Pane backend receives your quotas,
spend, keys, or provider data. The Pane-hosted update endpoint and the
minimal, anonymous, opt-out daily statistic are described in full below.

## Every network call Pane can make

This is the complete list. Anything not listed here does not happen.

| Destination | When | What is sent |
|---|---|---|
| Each provider's own API (Anthropic, OpenAI/ChatGPT, cursor.com, GitHub, x.ai, opencode.ai, Devin, MiniMax, OpenRouter, Z.ai, Google, DeepSeek, Moonshot, Kimi Code, ElevenLabs, Codebuff, Kilo, AihubMix, StepFun, SiliconFlow, Novita, Alibaba Model Studio…) | Every refresh (default 5 min), only for providers you have enabled | That provider's own token/key, exactly as its official tool would send it. Full per-provider detail: [providers.md](providers.md) |
| User-configured Custom Balance relay origins | Every refresh, only when Custom Balance is enabled | Subscription + usage requests send that account's API key as Bearer to the configured relay origin only; public origins must use HTTPS, and plaintext HTTP is limited to private, loopback, or link-local IP literals. The origin and key are never sent to Pane servers. |
| User-configured One/New API origins | Status probe when saving/changing a site, plus one unauthenticated backfill if a stored site has no display unit; billing on every refresh for enabled keys | `/api/status` with no key. Subscription + usage send that site's key as Bearer, **only to that origin** — never to Pane servers. |
| `raw.githubusercontent.com` (LiteLLM), `models.dev`, `robinebers.github.io` | ~Daily | Anonymous GET for public model price tables (no identifying data) |
| `github.com/Aafff623/pane/releases/latest/download/latest.json` | On launch + every 4 h | Anonymous GET for this project's own update manifest, carrying no app data. GitHub may log the request as the host; Pane runs no counter. See "The update check" below. |
| `us.i.posthog.com` | Once per day (unless switched off) | The two anonymous daily-statistic events described in "Anonymous usage statistics" below — a random ID, version, enabled-provider list, and per-provider success/failure counts. Never usage amounts, spend, keys, or error text. |
| `127.0.0.1:11434` (your own PC) | Every refresh, if Ollama is enabled | Local-only query of your Ollama server |

Notably absent: session recording, event streams, A/B flags, autocapture
of any kind — none of it exists in the codebase, and the daily statistic
above is the entire analytics surface.

## Anonymous usage statistics

Settings → Privacy → **"Share anonymous usage statistics"** (on by
default; turning it off is a hard stop — nothing is counted, nothing is
written, and the stored random ID is deleted so re-enabling starts over
as a brand-new anonymous install).

When on, Pane sends at most two kinds of event per day to PostHog (the
same disclosed-and-toggleable approach as the Mac app Pane is a port of):

- **`app_daily_active`** — once per day: "this install was alive today",
  the app version, which providers are enabled, which metrics you
  starred (stable IDs only), appearance/density/refresh settings.
  Multi-account installs report only the plain provider family
  ("claude", once) — account-scoped card ids are derived from your
  account identity and never leave the machine, on any field of either
  event.
- **`provider_refresh_daily`** — per provider, summarizing the previous
  day: how many refreshes succeeded, went stale, or failed, with failure
  *categories* only (auth / rate-limit / server / network / other).
  The raw error text never leaves your machine — it can contain paths
  or account details, so only the category enum is sent.
  One/New API (`onenewapi`) is reported as that family id at most once
  per refresh. Site ids, key ids, configured counts, labels, and origins
  never leave the machine.

The identity attached to these events is a **random UUID** generated on
your machine — derived from nothing (not your hardware, not your IP, not
your account), linked to nothing, stored in `%APPDATA%\Pane\telemetry.json`.
Every event also instructs PostHog not to build a person profile, and the
PostHog project is configured to **discard client IP addresses** at
ingestion (country-level GeoIP resolves first, then the IP is dropped).

What is *never* sent, with this toggle on or off: your quotas, usage
percentages, spend amounts, model names from your logs, tokens, keys,
file paths, or any free-form text. The entire implementation is one
auditable file: [`src-tauri/src/telemetry.rs`](../src-tauri/src/telemetry.rs)
— no SDK, just two documented POSTs.

## The update check

Pane has to ask *somewhere* "is there a newer version?" — that request
existed from day one. Current builds ask exactly one endpoint: this
project's own GitHub Releases manifest
(`github.com/Aafff623/pane/releases/latest/download/latest.json`, also
pinned in `tauri.conf.json`). The upstream `trypane.xyz` /
`pane.jazii.dev` endpoints were dropped when the project detached — their
builds are signed with a key this app no longer trusts, so querying them
could only produce signature failures. Every update is
signature-verified against the minisign key baked into the app.

Pane itself operates no counting server and stores nothing about the
check. GitHub, as the host of the manifest, may log the request (IP,
timestamp) under [GitHub's own privacy
statement](https://docs.github.com/site-policy/privacy-policies/github-general-privacy-statement);
that logging is GitHub's, not Pane's, and Pane receives only the manifest
JSON. No machine ids, no usernames, no usage data — your quotas, spend,
and provider data never leave your PC, same as always.

## What stays on your PC

- **Credentials**: read from the files the official CLIs already maintain
  (see [providers.md](providers.md)); pasted API keys live in the OS
  credential vault (Windows Credential Manager / macOS Keychain / Linux
  Secret Service), and the JSON files keep only non-secret fields — a row
  whose vault write fails keeps its plaintext as the documented fallback.
  Login-based accounts (OAuth / device-code) are different: their tokens
  sit in `%APPDATA%\Pane\login-accounts\<family>.json` as plaintext JSON,
  protected by your user profile ACL rather than the vault. One/New API
  sites and keys use a
  nested `%APPDATA%\Pane\onenewapi.json`, written owner-only (Windows
  protected DACL / Unix `0600`) so a permissive Pane directory cannot
  expose the secrets; Settings lists names, URLs, and
  key labels only — never secrets or fragments. Sent only to their own
  vendor (or the user-configured One/New API origin). Status fingerprint
  (`GET /api/status`, no key) is structural only; billing is
  `/v1/dashboard/billing/subscription` and `/usage` (cents → dollars;
  sentinel `100000000` = unlimited). Some panels return the same
  user-level quota for every key; OneAPI with
  `DisplayTokenStatEnabled=false` can be account-level or inaccurate —
  Pane does not call native token usage/log APIs to compensate. AihubMix
  stays a separate built-in provider; One/New API values are not folded
  into Total Spend. `https://` is allowed for any valid host; `http://` is
  limited to private, loopback, or link-local IPv4/IPv6 address literals.
- **Refreshed OAuth tokens**: written back to the CLIs' own credential
  files so your tools stay signed in — same behavior as the CLIs
  themselves.
- **Usage snapshots & spend cache**: `%APPDATA%\Pane\` — cached locally so
  the app opens instantly; never uploaded.
- **Spend accounting**: computed by reading the CLIs' local log files on
  your disk. The logs never leave your machine; only the public price
  tables are downloaded.

## The local HTTP API

`http://127.0.0.1:6736/v1/usage` exists so your own scripts and widgets
can read your usage. It is loopback-only (nothing on your network can
reach it), serves usage numbers only (never credentials or keys), and
sends **no CORS headers**, and refuses non-loopback `Host` headers — so
websites you visit cannot read it through your browser, not even via
DNS rebinding. One/New API key cards are addressable by full snapshot
id (`onenewapi@<key-id>`) and omit dashboard URL, origin, and secrets.
Details: [local-http-api.md](local-http-api.md).

## Verifying all of this

Pane is MIT-licensed and this repository is the entire codebase. Search
it: there is no analytics SDK import, and every `http` call site lives
in a provider module ([`src-tauri/src/providers/`](../src-tauri/src/providers/)),
the pricing engine ([`src-tauri/src/pricing.rs`](../src-tauri/src/pricing.rs)),
the updater registration ([`src-tauri/src/lib.rs`](../src-tauri/src/lib.rs)),
or the one-file statistics module ([`src-tauri/src/telemetry.rs`](../src-tauri/src/telemetry.rs)).
