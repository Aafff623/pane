# Local HTTP API

Pane serves your usage as JSON so your own scripts, widgets, and overlays
can read it.

```
GET http://127.0.0.1:6736/v1/usage          # all enabled providers
GET http://127.0.0.1:6736/v1/usage/:id      # one provider (e.g. /claude)
```

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

## Security posture

- **Loopback only.** Binds `127.0.0.1` — nothing on your network can
  reach it.
- **Usage numbers only.** Snapshots of what the dashboard shows — never
  credentials, tokens, or keys.
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
