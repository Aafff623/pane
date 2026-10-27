# Local HTTP API

Pane serves your usage as JSON so your own scripts, widgets, and overlays
can read it. This HTTP API is the single machine-readable surface; the
optional MCP server is a thin read-only adapter over it (see
[ADR 0004](adr/0004-extension-surfaces.md)).

```
GET http://127.0.0.1:6736/v1/usage          # all enabled providers
GET http://127.0.0.1:6736/v1/usage/:id      # one provider (e.g. /claude, /onenewapi@<key-id>)
```

## Diagnostic endpoints

Four more read-only endpoints exist so a coding agent (or you) can triage
Pane without reading files. Same security posture as `/v1/usage` — loopback
only, no auth, **never credentials**.

```
GET http://127.0.0.1:6736/v1/status     # version, uptime, refresh age, provider rollups, proxy SOURCE
GET http://127.0.0.1:6736/v1/providers  # every card with status/stale/error/warning + disabled/removed id lists
GET http://127.0.0.1:6736/v1/doctor     # checks[{name, ok, classification, detail, remediation}] + healthy
GET http://127.0.0.1:6736/v1/sources    # local spend sources: detected vs absent
GET http://127.0.0.1:6736/v1/limits     # openusage.limits.v1 envelope (structured limits)
GET http://127.0.0.1:6736/v1/limits/:id # exact card id, or every card in a family
```

- `/v1/doctor` is the entry point: read `healthy`, then apply the
  `remediation` of every check whose `classification` is `failed`. Only facts
  this process can establish become ok/fail; anything not decidable in-process
  is `classification: "info"` with the honest reason — never a guessed verdict.
- `/v1/providers` carries the fields the macOS-compatible `/v1/usage` wire
  format deliberately drops (`status`, `stale`, `error`, `warning`), so an
  agent can tell "no data" from "disabled / no credentials / auth failed /
  network". Provider-supplied error text has URLs stripped to `<endpoint>`:
  reqwest embeds the request URL, which can carry a private relay host.
- `/v1/status` reports the proxy as a **source** (`config` / `system` /
  `none`), never the address.
- `last_snapshots.json` in `%APPDATA%\PaneAafff623\` still works as a fallback when
  the API is unreachable.

Wire format (compatible with the macOS OpenUsage API):

```json
[{
  "providerId": "claude",
  "displayName": "Claude",
  "plan": "Max",
  "fetchedAt": "2026-07-08T01:30:00Z",
  "lines": [{
    "type": "progress",
    "label": "Session",
    "used": 22.0,
    "limit": 100,
    "format": { "kind": "percent" },
    "resetsAt": "2026-07-08T04:39:59Z",
    "periodDurationMs": 18000000
  }]
}]
```

## Machine-readable limits (`openusage.limits.v1`)

`/v1/limits` is separate from the macOS-compatible `/v1/usage` rows. It
carries only structured, typed readings — a display string never becomes a
quota. The envelope has exactly four top-level fields:

```json
{
  "schema": "openusage.limits.v1",
  "generatedAt": "2026-10-26T12:00:00Z",
  "providers": {
    "claude": {
      "displayName": "Claude",
      "plan": "Max",
      "fetchedAt": "2026-10-26T12:00:00Z",
      "stale": false,
      "resources": {
        "Session": { "kind": "consumption", "unit": "percent", "used": 22, "limit": 100, "remaining": 78, "utilization": 0.22, "windowSeconds": 18000 },
        "Extra credits": { "kind": "balance", "unit": "usd", "remaining": 7.5, "available": true }
      }
    }
  },
  "errors": [{ "providerId": "devin", "code": "error", "message": "…" }]
}
```

- A value that is not known is **omitted**, never zeroed; `estimated` is
  reserved and currently never emitted.
- `:id` prefers an exact card id (`onenewapi@<key-id>`); otherwise it returns
  every card in that provider family (`onenewapi`). `/v1/usage/:id` keeps its
  exact-only behavior for macOS parity. A card that exposes no structured
  resource is simply absent from `providers`; an unknown id is
  `404 provider_not_found`.
- An untouched window has no `resetsAt`: the reset instant is a full period
  away, not a countdown. The card surfaces the same fact as “Not started”.
- Free text (error messages, dashboard URLs, keys) never enters resources.
  Error strings are URL-stripped by the same sanitizer as `/v1/providers`.
- If port 6736 is already taken, the later process stays silent; `/v1/limits`
  is unavailable with the rest of the API.

## Security posture

- **Loopback only.** Binds `127.0.0.1` — nothing on your network can
  reach it.
- **Usage numbers only.** Snapshots of what the dashboard shows — never
  credentials, tokens, or keys. One/New API entries use the full snapshot
  id (`onenewapi@<key-id>`) as `providerId` and the card title as
  `displayName`; dashboard URL, origin, site id, and secrets are omitted.
- **No CORS headers.** Unlike the macOS app (which sends
  `Access-Control-Allow-Origin: *` and documents that any web page can
  read your usage), Pane sends no CORS headers — so browsers block web
  pages from reading this API. PowerShell, curl, Rainmeter, and native
  apps are unaffected; CORS only constrains browsers.
- **Loopback Host headers only.** Requests whose `Host` header isn't a
  loopback spelling (`127.0.0.1`, `localhost`, or `[::1]`, with or
  without `:6736`) get `403 {"error": "forbidden_host"}`. This blocks
  DNS rebinding — a malicious page pointing its own hostname at
  127.0.0.1 to sidestep the browser's cross-origin rules. Plain
  scripts are unaffected (curl and PowerShell send a loopback Host
  automatically; a missing Host header is also fine), but if you call
  the API through a hosts-file alias or a proxy that rewrites Host,
  use one of the loopback spellings instead.
- **No authentication.** Any program running as your Windows user can
  read this API — that is what makes zero-config widgets and scripts
  possible, and it is a deliberate trade-off. What such a program gets is
  usage percentages and reset times; your credentials are never served.
  (A local process that could steal anything meaningful could also read
  the CLIs' credential files directly — the API adds no new exposure.)
- If port 6736 is already taken, the API is silently unavailable for that
  session; everything else works normally.
