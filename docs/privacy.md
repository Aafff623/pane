# Privacy

Pane is built on one rule: **your data is nobody's business, including
ours.** There is no Pane account, no Pane backend receives your quotas,
spend, keys, or provider data, and this build sends no analytics or
telemetry. The Pane-hosted update endpoint is described in full below.

## Every network call Pane can make

This is the complete list. Anything not listed here does not happen.

| Destination | When | What is sent |
|---|---|---|
| Each provider's own API (Anthropic, OpenAI/ChatGPT, cursor.com, GitHub, x.ai, opencode.ai, Devin, MiniMax, OpenRouter, Z.ai, Google, DeepSeek, Moonshot, Kimi Code, ElevenLabs, Codebuff, Kilo, AihubMix, StepFun, SiliconFlow, Novita, Alibaba Model Studio…) | Every refresh (default 5 min), only for providers you have enabled | That provider's own token/key, exactly as its official tool would send it. Full per-provider detail: [providers.md](providers.md) |
| User-configured Custom Balance relay origins | Every refresh, only when Custom Balance is enabled | Subscription + usage requests send that account's API key as Bearer to the configured relay origin only; public origins must use HTTPS, and plaintext HTTP is limited to private, loopback, or link-local IP literals. The origin and key are never sent to Pane servers. |
| User-configured One/New API origins | Status probe when saving/changing a site, plus one unauthenticated backfill if a stored site has no display unit; billing on every refresh for enabled keys | `/api/status` with no key. Subscription + usage send that site's key as Bearer, **only to that origin** — never to Pane servers. |
| `raw.githubusercontent.com` (LiteLLM), `models.dev`, `robinebers.github.io` | ~Daily | Anonymous GET for public model price tables (no identifying data) |
| `pane.threetwoa.live/update/latest.json` (this project's own Cloudflare edge route), then `github.com/Aafff623/pane/releases/latest/download/latest.json` | On launch + every 4 h | Anonymous GET for this project's own update manifest, carrying no app data. The edge route serves the same manifest but rewrites the Windows installer URL to the Gitee mirror for CN clients; GitHub is the reachable-everywhere fallback. Cloudflare and GitHub may each log the request as the host; Pane runs no counter. See "The update check" below. |
| `status.claude.com` | Only when a Claude card is stale or failed, and the status probe is on | Anonymous GET for the public Statuspage incident summary; no app data. |
| `127.0.0.1:11434` (your own PC) | Every refresh, if Ollama is enabled | Local-only query of your Ollama server |

Notably absent: session recording, event streams, A/B flags, autocapture
of any kind — none of it exists in the codebase. There is no analytics
SDK, no crash reporter, and no telemetry endpoint.

## The never-sent list

The following never leaves the machine through Pane, on any setting, in
any payload, log or crash report:

- credentials, API keys, OAuth tokens, cookies, vault contents and master
  passwords;
- quotas, usage percentages, spend amounts, token counts, balances and
  reset times;
- model names read from session logs, prompts, completions, tool arguments,
  file contents, file paths and project names;
- account identities, labels, site ids, relay origins and configured
  counts;
- window titles, screenshots, clipboard contents and keystrokes.

Pane has no analytics payload. If a future feature needs to send
something new, it must be added to the network table first and called out
in this list if it changes the boundary.

## No usage statistics

The fork previously forwarded the upstream project's anonymous daily
rollup to PostHog. That channel was removed in 0.11.0 so this build no
longer sends any data to the upstream author's analytics project, and
there is no telemetry setting because there is nothing to toggle.

## The update check

Pane has to ask *somewhere* "is there a newer version?" — that request
existed from day one. Current builds ask a short ladder of this
project's own endpoints, tried in order (the updater only advances when
a request fails or answers non-2xx, never for slowness):

1. `pane.threetwoa.live/update/latest.json?source=gitee` — the project's
   Cloudflare edge route, explicitly asked for the mirror. Used when a
   proxy is configured: a CN user behind a global proxy exits abroad, so
   the country-based branch would miss exactly the users who suffer most
   from GitHub's slow release assets.
2. `pane.threetwoa.live/update/latest.json` — the plain edge route; it
   serves the same GitHub manifest, except CN clients (by request
   country) get the Windows installer URL rewritten to the Gitee mirror.
   The route probes that the mirror asset exists before offering it, so a
   missing mirror falls back to the GitHub URL — it can cost speed, never
   availability.
3. `github.com/Aafff623/pane/releases/latest/download/latest.json` —
   GitHub's own manifest, the reachable-everywhere fallback (also the URL
   pinned in `tauri.conf.json`; the app overrides it with the ladder
   above).

The upstream `trypane.xyz` / `pane.jazii.dev` endpoints were dropped when
the project detached — their builds are signed with a key this app no
longer trusts, so querying them could only produce signature failures.
Every update is signature-verified against the minisign key baked into
the app.

Pane itself operates no counting server and stores nothing about the
check. Cloudflare and GitHub, as the hosts of the manifest, may log the
request (IP, timestamp) under their own privacy statements
([Cloudflare](https://www.cloudflare.com/privacypolicy/),
[GitHub](https://docs.github.com/site-policy/privacy-policies/github-general-privacy-statement));
that logging is theirs, not Pane's, and Pane receives only the manifest
JSON. No machine ids, no usernames, no usage data — your quotas, spend,
and provider data never leave your PC, same as always.

## What stays on your PC

- **Credentials**: read from the files the official CLIs already maintain
  (see [providers.md](providers.md)); the secrets Pane itself persists —
  pasted API keys, multi-account keys, and Pane-managed login tokens
  (OAuth / device-code) — live in the OS credential vault
  (Windows Credential Manager / macOS Keychain / Linux Secret Service),
  and the JSON files keep only non-secret fields: a row whose vault write
  fails keeps its plaintext as the documented fallback, and legacy
  plaintext entries migrate on first read (the vault write must succeed
  before the file loses them). Two further stores are deliberately
  outside the OS vault, each with its own protection: the master-password
  key vault (AES-256-GCM sealed file) and search/MCP keys (plaintext
  `search-keys.json`). The WorkBuddy web-session cookie you
  explicitly import follows the same vault-first pattern through a session
  slot (`token:web/workbuddy`); `workbuddy_web.json` is kept only while a
  vault write fails. One/New API
  sites and keys use a
  nested `%APPDATA%\PaneAafff623\onenewapi.json`, written owner-only (Windows
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
- **Usage snapshots & spend cache**: `%APPDATA%\PaneAafff623\` — cached locally so
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
it: there is no analytics SDK or telemetry module, and every `http` call
site lives in a provider module ([`src-tauri/src/providers/`](../src-tauri/src/providers/)),
the pricing engine ([`src-tauri/src/pricing.rs`](../src-tauri/src/pricing.rs)),
or the updater registration ([`src-tauri/src/lib.rs`](../src-tauri/src/lib.rs)).
