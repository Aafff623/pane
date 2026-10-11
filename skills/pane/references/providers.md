# Provider quirks and credential expectations

Use this to interpret an `error` / `no_credentials` status. Pane supports many
providers; the ones below are the ones that most often need explanation. For the
full list, see the app's Customize drawer and `docs/providers.md`.

## Credential kinds

- **API key** (most coding plans): pasted in Settings; stored in the OS credential
  store (service "Pane", entry `provider:<family>`), not in plaintext JSON.
- **Multi-account** (`family@<fnv1a>` cards): several keys/accounts under one family
  (e.g. `clinepass`, `commandcode`, `onenewapi`); each card is independent.
- **OAuth / device-code** (codex, cursor, kimi, qoder, trae, zed, windsurf, codebuddy,
  kiro): a login flow; tokens can rotate or expire, producing an auth error.
- **Web-session / console** (doubao, sensenova, trae/qoder CN): read from a local
  credential file or a pasted console token — these break when the vendor changes
  its file format or rotates the session.

## Known traps

- **One/New API relay sites**: a wrong access token returns HTTP 200 with
  `success:false`. Token-only sites (no relay key) can only show the subscription
  endpoint. Never trust a 200 alone.
- **ClinePass**: `percentUsed` is already 0–100 (do not scale); `resetsAt` is
  absent while a window has zero usage.
- **APIGOTO**: `is_active` covers key+user, not subscription liveness — a lapsed
  subscription still returns 200 with the subscription gone.
- **Kimi**: the membership-wide monthly limit is never in the usage API; it only
  surfaces as failed inference, and the wall refreshes on the **27th**.
- **Trae CN**: credit packs stack; only the pack currently being drained carries
  `credits_amount`, so only it renders as a progress row.
- **Command Code GOAT**: queries the CLI's own `/alpha/billing/*`; an idle 5h
  window reports `resetAt: 0` (not an error).

## What to report

State the credential **kind** and whether it is **present**, then the status and
error text. Never print the key, token, cookie, or full provider URL.
