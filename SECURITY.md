# Security Policy

Pane reads the credential files that AI CLIs and editors keep on your PC.
That is a serious responsibility, and this page explains how we handle it
and how to reach us when something looks wrong.

## Reporting a vulnerability

**Please do not open a public issue for security problems.**

Use GitHub's private vulnerability reporting: go to the
[Security tab](https://github.com/ItsJazii/pane/security) → *Report a
vulnerability*. You'll get a response as fast as humanly possible for a
small open-source project — usually within a couple of days.

Please include: what you found, how to reproduce it, and what an attacker
could do with it.

## Security properties you can verify

All of this is auditable in the source — links go to the exact code.

- **Tokens never leave their lane.** Each provider's credential is sent
  only to that provider's own API over HTTPS
  ([`src-tauri/src/providers/`](src-tauri/src/providers/) — one module per
  provider; see [docs/providers.md](docs/providers.md) for the exact files
  read and endpoints called).
- **Multi-account discovery widens where credentials are *found*, never
  where they are *sent*.** To support second logins kept via
  `CLAUDE_CONFIG_DIR`/`CODEX_HOME`, Pane scans dot-folders in your home
  directory and `~\.config` for Claude/Codex-shaped credential files —
  the places those official mechanisms put them. A discovered credential
  follows the exact same lane rule (its own vendor's API only, token
  refresh written back beside it), a file that cannot name its account
  is never used at all, and a broadly-discovered Codex candidate must
  additionally *prove* it's an OpenAI login (OpenAI's own claim
  namespace in its `id_token`) before any request — a look-alike
  credential file from another app never enters the lane.
- **No analytics SDK, no crash reporter, no event streams.** Pane's
  entire self-reporting surface is two anonymous channels, both
  documented field-by-field in [docs/privacy.md](docs/privacy.md): the
  update check (country-level install counting, no IPs stored) and an
  opt-out once-a-day usage statistic (random install ID, daily rollups,
  error categories only — one auditable file,
  [`src-tauri/src/telemetry.rs`](src-tauri/src/telemetry.rs)). Turning
  the statistic off is a hard stop and deletes the stored ID. The full
  list of network calls Pane can make is in
  [docs/privacy.md](docs/privacy.md).
- **The local HTTP API is loopback-only, CORS-locked, and Host-checked.**
  It binds `127.0.0.1:6736`, serves usage numbers (never credentials),
  sends no `Access-Control-Allow-Origin` header, and refuses requests
  whose `Host` header isn't a loopback spelling — so web pages you visit
  cannot read it from a browser, not even via DNS rebinding
  ([`src-tauri/src/httpapi.rs`](src-tauri/src/httpapi.rs)).
- **Updates are cryptographically verified.** The auto-updater accepts
  only releases signed by the project's minisign key — passphrase-protected,
  master copy held offline; the public key is baked into the app. The
  install script verifies the installer's SHA-256 against the release
  manifest and refuses to run on mismatch ([`install.ps1`](install.ps1)).
- **Links are restricted.** The app can only open `http(s)://` URLs in
  your browser — nothing that could launch a program.
- **The webview is locked down.** A strict Content-Security-Policy (no
  remote scripts, no eval, no frames) plus a minimal Tauri capability set
  — the UI can reach only Pane's own commands, no plugin APIs
  ([`src-tauri/tauri.conf.json`](src-tauri/tauri.conf.json),
  [`src-tauri/capabilities/default.json`](src-tauri/capabilities/default.json)).
- **Config writes are schema-bound.** The UI can only write known config
  keys; anything else is dropped and logged.
- **Credential files are backed up before write.** When Pane writes a
  refreshed OAuth token back to a CLI's credential file, it first copies
  the original to `*.pane-bak` — a bad write can never cost you a login.
- **API keys you paste** are stored in `%APPDATA%\Pane` on your PC,
  readable only by your Windows user, and sent only to their own vendor.

## Known limitations (honesty section)

- The installer is **not yet Authenticode-signed**, so SmartScreen warns
  on first run. Updates are minisign-verified regardless. A code-signing
  certificate is planned. (The `irm https://trypane.xyz/install.ps1 | iex`
  install path and winget don't trigger SmartScreen.)
- Release binaries are built by GitHub Actions from the pushed tag —
  see [.github/workflows/release.yml](.github/workflows/release.yml);
  every release links public build logs proving the binary comes from
  the tagged source.
- Pane refreshes OAuth tokens and writes them back to the CLIs' own
  credential files (keeping your CLIs signed in). This means Pane has the
  same access to those accounts as the CLIs themselves — that's inherent
  to what the app does.
- The install script's SHA-256 check verifies the download against
  `latest.json` **from the same GitHub release** — it defends against
  corrupted or swapped downloads, not against a fully compromised release
  (an attacker who can replace the installer can replace the manifest,
  and the script itself, too). The cryptographic guarantee is the
  auto-updater's minisign verification once installed; the first install
  ultimately trusts GitHub. Anyone who'd rather not pipe a script into
  PowerShell at all can use winget or download-and-read first — the
  README shows both.
- Model prices (LiteLLM, models.dev, the OpenUsage supplement) are
  fetched without signatures — tampered pricing data could at worst show
  wrong *display* dollars. Inputs are size-capped and never touch
  credentials or spend logs.

## Supported versions

Only the [latest release](https://github.com/ItsJazii/pane/releases/latest)
is supported. The auto-updater keeps installs current.
