<div align="center">

# Pane

**All your AI subscription limits in one liquid-glass tray popover for Windows.**

One click on the tray icon answers the questions every AI power user keeps
asking: *How much of my Claude session is left? When does my Codex weekly
reset? What did today actually cost me?*

**[pane.jazii.dev](https://pane.jazii.dev)** · [Install](#install) · [How it works](#how-it-works) · [Providers](#providers-21-and-counting) · [Features](#features) · [Privacy](#privacy--security) · [Credits](#credits)

<img src="docs/promo.png" width="760" alt="Pane — track all your AI subscription limits in one tray app: Total Spend donut with per-provider slices, usage cards with pace bars" />

</div>

---

## Why Pane

If you use AI coding tools seriously, you're juggling half a dozen separate
subscriptions — Claude Max, ChatGPT/Codex, Copilot, Cursor, and whatever
else this month brought. Each one hides its limits behind its own dashboard,
counts in its own units, and resets on its own schedule. The only time you
find out you're running low is when you hit the wall mid-task.

Pane puts all of them in one place, in your system tray, refreshed every few
minutes, with pace warnings *before* you hit the wall. It started as a
Windows rebuild of the excellent [OpenUsage for macOS](https://github.com/robinebers/openusage)
by [Robin Ebers](https://github.com/robinebers) and is growing into a
broader AI-workflow companion from there.

## Install

Every release binary is built and published by GitHub Actions straight
from the tagged source — public build logs, verifiable provenance.

### winget (recommended)

```
winget install Pane.Pane
```

Pane is in [Microsoft's official winget community repo](https://github.com/microsoft/winget-pkgs/tree/master/manifests/p/Pane/Pane) —
reviewed, hash-verified, no SmartScreen prompt.

### One-liner (PowerShell)

```powershell
irm https://pane.jazii.dev/install.ps1 | iex
```

Downloads the latest release, verifies its SHA-256, installs per-user
(no admin), and launches Pane. No SmartScreen prompt.

Piping a script straight into PowerShell runs whatever the server sends
at that moment. Prefer to look first? Same script, two steps:

```powershell
iwr https://pane.jazii.dev/install.ps1 -OutFile install.ps1
# read install.ps1 — one short, commented script — then:
powershell -ExecutionPolicy Bypass -File .\install.ps1
```

(Or skip the script question entirely: `winget install Pane.Pane` above
is hash-verified by Microsoft's pipeline.)

### Installer (.exe)

1. Grab **`Pane_x.y.z_x64-setup.exe`** from the
   [latest release](https://github.com/ItsJazii/pane/releases/latest).
2. Run it. Pane installs per-user to `%LOCALAPPDATA%\Pane` — no admin
   rights needed.
3. Look for the Pane icon in the system tray (next to the clock). Click it.

> **SmartScreen note:** the installer isn't code-signed yet, so Windows may
> show "Windows protected your PC." Click **More info → Run anyway**. Code
> signing is on the roadmap.

Silent install (for scripts): `Pane_x.y.z_x64-setup.exe /S`

Whichever way you install, Pane keeps itself current — every install
checks for signed updates and offers a one-click restart when a new
version ships.

### Build from source

Prerequisites: Node.js 20+, Rust (stable-msvc), Visual Studio C++ Build
Tools, WebView2 (bundled with Windows 11).

```
git clone https://github.com/ItsJazii/pane
cd pane
npm install
npm run tauri dev     # run with hot reload
npm run tauri build   # installer lands in src-tauri/target/release/bundle
```

## How it works

Pane is a small Tauri v2 app: a Rust core doing the data work, a vanilla
TypeScript UI doing the glass. No Electron, no background services — one
~10 MB process idling around 90 MB of RAM.

**1. Finding your accounts.** The official CLIs and editors you already use
keep their login tokens in well-known per-user locations — Claude Code
writes `%USERPROFILE%\.claude\.credentials.json`, Codex CLI writes
`%USERPROFILE%\.codex\auth.json`, the GitHub CLI stores its token in
Windows Credential Manager, and so on. Pane reads those same files (or
takes an API key you paste into Settings) and shows a card for every tool
it finds. Tools it can't find start disabled — no dead cards.

**2. Asking the vendors.** Every few minutes, each provider's token is sent
to **its own vendor's API only** — the exact usage endpoints the vendors'
own apps use — and the card updates with sessions, weekly windows, credit
balances, and reset times. Expired OAuth tokens are refreshed and written
back, which keeps your CLIs signed in too. Failing providers get benched
briefly and their last good data is shown with an "Outdated" tag instead of
a blank card.

**3. Pacing the burn.** Every metric with a reset window gets a projection:
if you keep burning at this rate, will you make it to the reset? Bars turn
amber/red as the math worsens and optional Windows toasts fire once per
reset window ("Almost out", "Will run out").

**4. Counting the money.** Your CLIs already log every request locally.
Pane scans those logs (Claude, Codex, Grok, OpenCode, Devin CLI, Cursor
CSV, MiniMax CLI, Kimi Code, Qwen Code, the pi coding agent, the Hermes
desktop app), prices each
request with live per-model rates (LiteLLM /
models.dev, refreshed daily — hourly while unknown models are around, so
brand-new models price within the hour), and draws the Today /
Yesterday / 30-day donut with a per-model breakdown. Click the ring to
flip between dollars and tokens. On a flat-rate plan this shows what
your usage *would* cost at API prices — the best ad for your
subscription you'll ever see. Requests on models with no public
pricing keep their measured tokens in the counts, but no dollars are
ever guessed for them — a ⚠ on the provider's spend row says the real
cost runs a little higher than shown.

**5. Staying local.** All of the above happens on your machine. There is
no account, and your quotas, spend, and provider data never leave your
PC. Pane reports two things about itself, both anonymous: the update
check (country-level counting, no IPs stored) and an opt-out once-a-day
statistic (random ID, version, which providers are enabled, provider
success/failure counts — never amounts or error text) — see
[Privacy](#privacy--security) for the full contract and the off switch.

## Providers (21 and counting)

| Provider | How Pane connects |
|---|---|
| Claude (Claude Code) | `%USERPROFILE%\.claude\.credentials.json` + Anthropic usage API; multi-account — every discovered config-dir login gets its own card |
| Codex (Codex CLI) | `%USERPROFILE%\.codex\auth.json` + ChatGPT usage API, incl. reset-credit redemption; multi-account like Claude |
| Cursor | Cursor's local state database + cursor.com API |
| OpenCode (Go plan) | Official account-wide usage API (Go key from `auth.json`); local `opencode.db` for spend* |
| GitHub Copilot | Copilot editor login or GitHub CLI (Credential Manager) + GitHub API |
| Grok (Grok CLI) | `%USERPROFILE%\.grok\auth.json` + Grok billing/subscription APIs |
| Devin (Devin CLI) | `%APPDATA%\devin\credentials.toml` + GetUserStatus RPC; local CLI session store for spend |
| MiniMax | API key (Settings, env var, or CLI config) + token-plan API |
| OpenRouter | API key (Settings) or key stored by OpenCode |
| Z.ai | API key (Settings), CLI key file, or env var |
| Antigravity | Local language server, or Google Cloud Code API via Credential Manager |
| DeepSeek | API key (Settings) → balance |
| Moonshot (Kimi API) | API key (Settings) → balance (global + CN endpoints) |
| Kimi Code | Official CLI login (`kimi login`) → Session + Weekly plan bars; Moonshot API key → API wallet bar; local session spend |
| ElevenLabs | API key (Settings) → character quota with reset pacing |
| Ollama | Local server on :11434 — installed + loaded models, no key |
| Codebuff | `codebuff login` credentials file or API key → credits + weekly limit |
| Kilo | Kilo CLI login file or API key → credit blocks + Kilo Pass |
| AihubMix | API key (Settings or auto-detected from OpenCode) → usage vs spending limit |
| Qwen Code | Coding Plan key (Settings or env) → 5h/weekly/monthly request quotas + local spend |
| Hermes | Local ledger `%LOCALAPPDATA%\hermes\state.db` — last model, route, and catalog-priced spend |

*OpenCode's meters use the official usage API that shipped in
[anomalyco/opencode#16513](https://github.com/anomalyco/opencode/pull/16513)
— the same account-wide numbers as the Zen dashboard, so usage from your
other devices (or other people on a shared subscription) finally counts.
If the API is unreachable, Pane falls back to computing this machine's
usage locally from `opencode.db`, the same data `opencode stats` uses;
dollar spend figures are always local.

More on the way: IDE-database providers (Windsurf, JetBrains AI…) and
whatever the community asks for loudest.

## Features

- **Multi-account Claude & Codex** 🆕 — running a personal plan AND a
  work/enterprise seat? Keep the second login in its own folder (via
  `CLAUDE_CONFIG_DIR` / `CODEX_HOME`) and Pane shows one card per
  account — each with its own limits, plan, credits, and spend, named
  by its organization or email ("Claude — Acme"). The same account
  signed in twice stays one card, and your existing setup is untouched.
- **Pace projections** — colored bars and "will run out" warnings based on
  your burn rate within each reset window, plus optional Windows toasts.
- **Local spend** — Today / Yesterday / 30 Days donut with per-model
  breakdown and a 30-day trend, priced with live model rates. Hover a
  slice and it pops out with its legend row; click the ring to flip
  dollars ⇄ tokens.
- **Codex reset credits** — see each banked credit's exact expiry and
  redeem it with one click.
- **Signed auto-updates** — Pane checks for updates every time you open
  it (and every 4 hours in the background); when a release is out, the
  footer version stamp becomes an Update button — one click downloads,
  verifies the signature, and restarts.
- **Live tray numbers** — star up to two metrics per provider and they
  render as logo + percentage pairs directly in the tray.
- **Customize** — drag any card by its grip right in the popover to
  reorder, or open the Customize screen (☰) to reorder metrics, hide
  rows, and tuck rarely-needed ones behind an "On Demand" caret. Ctrl+Z
  undoes.
- **Liquid glass UI** — real SDF lens refraction on the auto-hiding
  sidebar and glass bars, magnetic minimap trail, circular day/night wipe.
- **Share cards** — hover a card, click ⧉, and paste anywhere: the copy
  is exactly what the card shows (bars, pace hints, trend — buttons and
  links stripped), framed with the Pane icon and tagline.
- **Quick links** — Status / Dashboard shortcuts on every card.
- **[Local HTTP API](docs/local-http-api.md)** — `GET
  http://127.0.0.1:6736/v1/usage` for scripts, Rainmeter widgets, stream
  overlays; same wire format as the Mac app, but with no CORS headers
  and a loopback-only Host check so web pages can't read it through
  your browser (not even via DNS rebinding).
- **Appearance** — System / Light / Dark, compact density, global shortcut
  (e.g. `Ctrl+Shift+U`), optional outbound proxy.

## Privacy & security

Pane reads credential files. You should not take our word for how it
treats them — verify it:

- **[docs/privacy.md](docs/privacy.md)** — the complete list of every
  network call Pane can make. No event streams, no session recording,
  no autocapture; the update check counts anonymous daily installs by
  country (no IPs stored), and an opt-out daily statistic reports
  version + enabled providers + refresh success/failure counts under a
  random ID attached to nothing. That document explains exactly how,
  field by field.
- **[docs/providers.md](docs/providers.md)** — per provider: exactly which
  files are read on your PC and exactly which endpoints they're sent to.
- **[SECURITY.md](SECURITY.md)** — how to report vulnerabilities
  privately, the security properties you can audit in source, and an
  honest list of current limitations (unsigned installer — the release
  binaries themselves are built by GitHub Actions from the tagged source,
  with public build logs).

The short version: tokens are sent only to their own vendor's API over
HTTPS; pasted keys live in `%APPDATA%\Pane`, readable only by your
Windows user; spend accounting parses your local logs locally; the HTTP
API is loopback-only with no CORS and a Host check; updates are
signature-verified.

## Settings (gear icon)

Refresh interval · Start with Windows · tray metric picker · appearance &
compact density · time format · global shortcut · notification toggles ·
outbound proxy · API keys.

## Credits

Pane exists because of
**[OpenUsage for macOS](https://github.com/robinebers/openusage)** by
**[Robin Ebers](https://github.com/robinebers)** (MIT). The hard part of a
tool like this — knowing which credential files to read, which
undocumented usage endpoints to call, and how to interpret their
responses — is research Robin did first and published openly. Pane is an
independent from-scratch rebuild for Windows (Rust + TypeScript instead of
Swift), but it stands on that research and gladly says so. If you're on a
Mac, use his app.

Additional thanks:

- [Tauri](https://tauri.app/) — the app shell that keeps Pane tiny.
- [prasen.dev](https://www.prasen.dev/) — the original SDF liquid-glass
  lens technique the UI's refraction is ported from.
- [LiteLLM](https://github.com/BerriAI/litellm) and
  [models.dev](https://models.dev/) — open model-price catalogs powering
  the spend engine.
- [shadcn/ui](https://ui.shadcn.com/) — the zinc design tokens the theme
  is built on.

Pane is not affiliated with or endorsed by Robin Ebers or any of the AI
vendors listed. Provider names and logos belong to their respective owners
and are used only to identify the services.

## License

[MIT](LICENSE) — © 2026 Jazii, with provider research credit to Robin
Ebers' OpenUsage (MIT).
