---
name: pane
description: Diagnose and explain Pane — the Windows tray app that tracks AI coding-plan quotas, balances, reset times, and local CLI token spend. Use when a user says a Pane card is empty, stale, or shows an error; the popover will not open; an update failed; or the spend panel looks wrong. Reads Pane's local read-only API and config; never touches credentials.
metadata:
  version: "1"
---

# Pane support

Pane is a local-first Windows tray app (`pane.exe`, Tauri). It queries each
configured provider's own API for quota/usage and scans local CLI logs for
token spend. Everything is on this machine; nothing goes to a Pane server.

Use this skill to **diagnose from evidence** and explain findings in plain
language. Do not guess, and do not ask the user to reinstall before the
evidence points there.

## Where the facts live (read-only, loopback only, no auth)

| Source | What it gives |
|---|---|
| `http://127.0.0.1:6736/v1/doctor` | **start here** — checks shaped `{name, ok, classification, detail, remediation}`; `healthy` is the one-line verdict |
| `http://127.0.0.1:6736/v1/providers` | every card with `status`, `stale`, `error`, `warning`, `plan`, `lines`, plus the `disabled` / `removed` id lists |
| `http://127.0.0.1:6736/v1/status` | version, uptime, refresh interval + last-refresh age, provider rollups, proxy **source** (never the address) |
| `http://127.0.0.1:6736/v1/sources` | spend sources present vs absent on this machine |
| `http://127.0.0.1:6736/v1/usage` · `/v1/usage/<id>` | usage numbers only (macOS-compatible wire format) |
| `%APPDATA%\PaneAafff623\config.json` | `disabled`, `removedProviders`, `refreshMinutes`, `proxy` |
| `%APPDATA%\Pane\last_snapshots.json` | fallback when the API is unreachable: the same snapshots as a map |

Always call with `curl -s --noproxy '*'` — a system proxy can otherwise swallow
loopback requests.

**Never read or echo** these files — they hold credentials:
`onenewapi.json`, `sensenova.json`, `stepfun-plan.json`, `doubao_cookies.json`,
`tavily-keys.json`, `accounts/*.json`, `keyvault.json`, `antigravity-token.json`.
Report only whether a credential is *present*, never its value. The API already
URL-strips provider error text; do not re-add endpoints from other sources.

If `pane.exe` ships a `pane` CLI on this machine, prefer `pane doctor --json` /
`pane providers --json` (authoritative, structured). If it does not, use the
sources above. See `references/symptoms.md` for the full decision tree.

## Triage in order

1. **Is Pane running?**
   `curl -s --noproxy '*' http://127.0.0.1:6736/v1/doctor`
   - JSON with `checks` → running. Read `healthy`; for every check with
     `classification:"failed"`, apply its `remediation` — each failure names
     its own fix.
   - Connection refused → the app is not running, **or** the port is occupied
     (a fresh instance silently fails to bind while the old socket sits in
     TIME_WAIT). Check the process, then the port.
2. **Read `GET /v1/providers`** and classify each card by `status`:
   - `ok` → fine (check `stale` and the age of `fetchedAt`).
   - `error` → read the card's `error`/`warning` text; route by provider in
     `references/providers.md`.
   - `no_credentials` → the user has not configured that provider's key/account.
   - **card absent from `providers`** → look it up in `disabled` / `removed`.
     Off ≠ broken — say which.
   - **all cards stale** → the refresh loop is stuck; compare
     `/v1/status` `refresh.ageSecs` against `refresh.minutes`.
3. **Popover will not open?** Follow the window/hotkey checks in
   `references/symptoms.md` (the minimized-window zombie state is the classic).
4. **Update failed?** Follow `references/update.md` (proxy chain).
5. **Spend looks wrong?** Check `GET /v1/sources` first (is the tool's log even
   present?), then `references/spend.md`.

## Reporting

Separate three things explicitly:
- **Observed** — what the JSON/API actually says (quote the field).
- **Inferred** — your conclusion and the reasoning.
- **Unknown** — what the evidence does not establish.

If the problem needs Pane's maintainers, assemble a reproducible evidence pack:
Pane version, Windows version, the symptom, the relevant JSON fragment, and the
steps already tried. Then point the user at the issue tracker. Never paste
credentials or full provider URLs.

## Read only the relevant reference

- Symptom → cause → fix decision tree: `references/symptoms.md`
- Per-provider quirks and credential expectations: `references/providers.md`
- Spend/token coverage and wording: `references/spend.md`
- Update channel and proxy pitfalls: `references/update.md`
