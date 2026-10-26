<div align="center">

<a href="https://pane.threetwoa.live/en/"><img src="src/assets/pane-logo.png" width="112" alt="pane"></a>

# Pane

### A local-first usage and quota workspace that lives in your tray

**Usage, quotas, balances and resets — one desktop workspace.**

[![Release](https://img.shields.io/github/v/release/Aafff623/pane?style=flat-square&label=Release&color=2563EB&labelColor=172033)](https://github.com/Aafff623/pane/releases) [![CI](https://img.shields.io/github/actions/workflow/status/Aafff623/pane/ci.yml?branch=main&style=flat-square&label=CI&color=2563EB&labelColor=172033)](https://github.com/Aafff623/pane/actions/workflows/ci.yml) [![Downloads](https://img.shields.io/github/downloads/Aafff623/pane/total?style=flat-square&label=Downloads&color=2563EB&labelColor=172033)](https://github.com/Aafff623/pane/releases) [![License](https://img.shields.io/badge/License-MIT-10B981.svg?style=flat-square)](LICENSE)

[![Windows](https://img.shields.io/badge/Windows-10%20%7C%2011%20(x64)-0078D4.svg?style=flat-square&logo=windows&logoColor=white)](https://github.com/Aafff623/pane/releases/latest) [![macOS](https://img.shields.io/badge/macOS-dmg-8B5CF6.svg?style=flat-square&logo=apple&logoColor=white)](https://github.com/Aafff623/pane/releases) [![Linux](https://img.shields.io/badge/Linux-AppImage%20%7C%20.deb-172033.svg?style=flat-square&logo=linux&logoColor=white)](https://github.com/Aafff623/pane/releases) [![Providers](https://img.shields.io/badge/Providers-71-2563EB.svg?style=flat-square)](docs/providers.md)

**[Download](#quickstart)** &nbsp;·&nbsp; **[Live demo](https://pane.threetwoa.live)** &nbsp;·&nbsp; **[Docs](#docs)** &nbsp;·&nbsp; **[Community](#community)**

**[English](README_EN.md)** · **[简体中文](README.md)**

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="attachments/hero-dark.jpg">
  <img src="attachments/hero-light.jpg" width="100%" style="border-radius: 14px;" alt="Pane floating panel and settings window: quotas, usage, fonts and keys">
</picture>

<sub>Floating panel <kbd>Alt + 2</kbd> · settings window · v0.8.9</sub>

</div>

---

**Contents** · [Is Pane for you?](#intro) · [Quick start](#quickstart) · [What it does](#features) · [Supported sources](#providers) · [How it fits together](#how) · [Documentation](#docs) · [Privacy](#privacy) · [Community](#community)

## <a id="intro"></a>Is Pane for you?

Every AI tool keeps its quota in its own console: Claude counts 5-hour sessions, Codex a weekly allowance, relays meter per key, and CLI spend is scattered across local logs. Answering "how much is left, when does it reset, which account is near the cap, what did I spend today" means five browser tabs and three local files.

**Pane brings all of it into one desktop panel summoned with `Alt + 2`:**

- **One overview** — 5-hour / 7-day / monthly quotas, balances, reset countdowns and account status; `Shift` cycles periods;
- **Local first** — tokens, cookies and API keys stay in the OS credential store and your user directory; queries go straight to each provider's endpoint, with no Pane relay in between;
- **Resident, not intrusive** — lives in the tray, auto-refreshes every 5 minutes (adjustable); failures surface as explicit error states instead of greyed-out unknowns.

| | Five consoles | **Pane** |
| :-- | :-- | :-- |
| What's left | One tab per platform | One panel; 5-hour / 7-day / monthly at a keystroke |
| When it resets | Do the math | Countdown on the card |
| Multiple accounts | Switch by hand | Independent cards, stable identity |
| What you spent | Dig through CLI logs | Local scan, split by model / tool |
| Where data goes | Uploaded to a cloud | Stays on this machine; no relay backend |

If two of those describe you, Pane is worth a try. Pane is a Windows desktop app first (macOS / Linux builds ship with each release).

## <a id="quickstart"></a>Quick start

1. **Download & install** — grab the installer for your platform from [Releases](https://github.com/Aafff623/pane/releases/latest). The Windows 10/11 (x64) `*-setup.exe` is the primary development and verification platform; Pane lives in the tray after install and needs no account of its own.
2. **Connect your sources** — open settings with `Ctrl + S` and follow each provider's hint to complete a local login (Copilot / Codex / Claude, …) or paste an API key; unenabled cards can be verified first with the "⚡ test connectivity" action.
3. **Daily use** — `Alt + 2` summons or dismisses the panel, `Shift` cycles 5-hour / 7-day / monthly periods, `T` shows the expiring list, `Esc` puts it away.

Don't want to install first? The [website demo](https://pane.threetwoa.live) is driven by a real build of the frontend — all nine shortcuts and both themes work, with fixed demo data and no real accounts.

<details>
<summary><b>Build from source</b></summary>

```powershell
git clone https://github.com/Aafff623/pane.git
cd pane
pnpm install
pnpm build          # frontend (TypeScript check + Vite build)
pnpm tauri build    # full desktop app
```

Dev mode needs two processes: Vite serves the frontend on `127.0.0.1:1420` and `pane.exe` serves the local usage API on `127.0.0.1:6736`. Full notes: [development startup guide](docs/dev-startup.md).

</details>

## <a id="providers"></a>Supported sources

The catalog currently holds **71 sources** in three auth shapes:

| Shape | Coverage | How it reads |
| --- | --- | --- |
| Local login state | Claude, Codex, Cursor, Copilot, Antigravity, … | Credentials from the local IDE / CLI + official usage API |
| OAuth device flow | Grok, Kiro, SenseNova, … | Started in-app; tokens rotate and persist |
| API key | DeepSeek, Kimi Code, StepFun, SiliconFlow, Novita, … | Key from settings, environment or CLI config |

Multiple accounts per provider each get an independent card (ClinePass, One/New API, Custom Relay, …), and local CLIs (Claude Code, Codex CLI, ZCode, OpenCode, …) contribute spend scanned straight from their logs. Fresh installs start with the mainstream subscription families enabled (Claude, Codex, Cursor, Grok, Z.ai, MiniMax, Antigravity, Kimi, …); everything else is one toggle away in settings.

<details>
<summary><b>All 71 providers</b> (mirrors <code>src/providerCatalog.ts</code>)</summary>

Claude, Codex, CodeBuddy, CodeBuddy CN, WorkBuddy, WorkBuddy AI, Cursor, OpenCode, Copilot, Grok, Devin, MiniMax, OpenRouter, Z.ai, Antigravity, DeepSeek, Kimi API, ElevenLabs, Ollama, Codebuff, Kilo, AihubMix, One/New API, Qwen Code, Hermes, Kimi Code, StepFun, StepFun Step Plan, Shandianshuo, SiliconFlow, Novita AI, Custom Relay, Qoder CN, Trae CN, Command Code, Doubao, ClawsGO, BochaAI, Tavily, Firecrawl, Brave Search, GIPHY, ClinePass, SenseNova, APIGOTO, MiniMax Code, Amp, AWS Bedrock, Chutes, Deepgram, Kiro, OpenAI API, Poe, TokenRhythm, Venice, Vertex AI, Warp, Windsurf, MiMo, Trae, Qoder, Zed, Droid, JetBrains AI, Groq, Hugging Face, LongCat, sub2api, Mistral, Perplexity, Volcengine Ark

</details>

## <a id="how"></a>How it fits together

<div align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="attachments/how-dark.png">
    <img src="attachments/how-light.png" width="900" alt="Pane architecture: local core querying provider endpoints directly">
  </picture>
</div>

The Rust core schedules quota queries that **hit each provider's official endpoint directly** — Pane never sits in the middle. Token spend is scanned from local CLI logs with no cloud service involved; credentials live in the OS credential store and your user directory; the read-only diagnostics API listens on `127.0.0.1` only. What each source can report depends on its API or local data — per-provider notes in the [Provider catalog](docs/providers.md).

## <a id="features"></a>What it does

### Quotas and spend, one glance

The floating panel answers "how much is left, when does it reset" in a single overview: green / yellow / red states tell available, peak and unavailable apart, with remaining quota and reset countdowns right on the card; `Shift` cycles 5-hour / 7-day / monthly periods. Failed queries surface as explicit error states, never as grey unknowns; providers without a monthly quota fall back to weekly in the month view, clearly labeled.

The same panel switches to **By tool / Dollars**, so quota and spend never need separate windows.

<table>
  <tr>
    <td width="50%" align="center" valign="top">
      <picture>
        <source media="(prefers-color-scheme: dark)" srcset="attachments/overview-dark.png">
        <img src="attachments/overview-light.png" width="390" alt="Quota overview · 5-hour view">
      </picture><br/>
      <sub>5-hour session view</sub>
    </td>
    <td width="50%" align="center" valign="top">
      <picture>
        <source media="(prefers-color-scheme: dark)" srcset="attachments/spend-tool-dollar-dark.png">
        <img src="attachments/spend-tool-dollar-light.png" width="390" alt="Spend breakdown · by tool / dollars">
      </picture><br/>
      <sub>By tool / dollars</sub>
    </td>
  </tr>
</table>

**Long-wall mirroring**: when a weekly / monthly wall is maxed, the short-period ring mirrors the wall (100% + refresh countdown), killing the "5-hour ring is green but every request is rejected" illusion; exhausted quotas get a readable explanation (e.g. `Weekly limit reached · resets in 2d 21h`) instead of a bare red ring with no detail.

### Providers

71 sources connect through three auth shapes, and every card has a "⚡ test connectivity" action (works on unenabled cards too, reporting `connected · N ms` or the failure reason) plus a "? quota how-to" that explains the key / OAuth / local-login steps for its auth shape with console links. The Customize drawer adds in-place search, custom groups (🥚🐑) and two-level notes:

> Category tabs switch between **Coding Agents / Productivity / MCP-search**; removed providers land in the drawer's footer and restore in one click.

<div align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="attachments/customize-dark.png">
    <img src="attachments/customize-light.png" width="390" alt="Customize drawer: search, groups and toggles">
  </picture>
</div>

<details>
<summary><b>Multiple accounts</b> — add accounts for the same provider and inspect each quota</summary>

- Add accounts in Settings → Usage through the provider's own entry point: supported API-key pools and login providers include Codex, Cursor and TokenRhythm;
- Codex reads the local CLI login and can add other subscriptions through OAuth. The local login is a read-only source; Pane-managed accounts can be renamed, archived and deleted individually;
- Each account keeps a stable local identity. The overview folds accounts under their provider and expands to let you switch between them;
- Right-click to archive an account and restore it in Settings. Deletion offers an option to keep usage history;
- "Provider name" retitles the card; "sub-account name" changes one account's note. API-key pools also support choosing a default account.

</details>

### Spend, fully traceable

Local CLI logs are scanned to compute spend — no cloud service involved; dollar and token views both work, with a per-model breakdown:

| Capability | What it gives you |
| :--- | :--- |
| Today / yesterday / last 30 days | Windowed token and dollar totals |
| Model breakdown | Spend and tokens per model; unknown prices keep the facts and are marked unpriced |
| Provider trend | The quota trend shows that provider's consumed credits or measured tokens |
| 365-day heatmap | Daily intensity at a glance |
| Tiered icons | Spend-card icons tier by daily token volume; thresholds customizable |
| Copy as image | The total-spend card asks for a range first (today / yesterday / 7 days / 30 days / all) and then renders the share image; provider cards snapshot instantly |

Per-CLI metering is documented in [Token spend coverage](docs/token-spend-coverage.md).

### Skin workshop

Light / dark themes switch in one keystroke (`Ctrl + L`); **the skin market is experimental and hidden by default** — enable it with a second confirmation in settings. Two skins ship built in: **Cloud Post Office** (hand-drawn stamps, a cat courier) and **Ink & Paper** (ink-wash mountains, red koi, paper grain); both follow light and dark. The workshop is a gallery layout: the selected skin's full preview (a total-spend panel sample) on top, thumbnails below, and "use this skin / restore native" pinned in the action bar; day / night / side-by-side preview modes retain scroll position and keyboard focus across switches.

<table>
  <tr>
    <td width="50%" align="center" valign="top">
      <picture>
        <source media="(prefers-color-scheme: dark)" srcset="attachments/skins-dark.png">
        <img src="attachments/skins-light.png" width="390" alt="Cloud Post Office skin: hand-drawn stamps and a cat courier">
      </picture><br/>
      <sub>Cloud Post Office · hand-drawn stamps</sub>
    </td>
    <td width="50%" align="center" valign="top">
      <picture>
        <source media="(prefers-color-scheme: dark)" srcset="attachments/skins-ink-dark.png">
        <img src="attachments/skins-ink-light.png" width="390" alt="Ink & Paper skin: ink-wash koi and paper grain">
      </picture><br/>
      <sub>Ink &amp; Paper · ink-wash koi</sub>
    </td>
  </tr>
</table>

<details>
<summary><b>Resource-pack format</b></summary>

Skins distribute as resource packs: a zip holding `manifest.json` (id, name, version, minAppVersion, entry CSS, per-file sha256) plus assets. Installing means unzip + per-file verification + manifest validation — any failure rejects the whole pack with no leftovers; installed packs live under `%APPDATA%\PaneAafff623\skins\<id>\` and reload from disk after restart. Wallpaper, mascot and exit interactions change as a set; turning it off hot-swaps back to the native skin. Design notes: [skin market design](docs/skin-market-design.md).

</details>

### Keys stay on your machine

Provider master keys, every sub-account key, **and all login tokens** (OAuth / login refresh tokens, Cursor / Codex account tokens, search keys) live in the **OS credential store** (Windows Credential Manager, …) — the JSON files keep non-secret fields only; legacy plaintext entries migrate automatically on first read, and the plaintext is dropped only after the store write succeeded. Each row on the "Keys & usage" page requires **master-password verification** to reveal, with plaintext never staying on screen and one-click copy; local-login rows show a green "connected" state naming where the credential is read from.

<details>
<summary><b>Credential storage, plaintext reveal and shortcuts</b></summary>

| | |
| :--- | :--- |
| **Credential storage** | OS credential store + user directory; never uploaded, never in Git |
| **Revealing plaintext** | Unlocked row-by-row after the master password; copyable, never parked on screen |
| **Shortcuts** | Nine built-ins, full-row conflict detection, per-key rebinding, one-click reset |

<div align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="attachments/settings-keys-dark.png">
    <img src="attachments/settings-keys-light.png" width="520" alt="Settings panel: keys and usage page">
  </picture>
</div>

</details>

<details>
<summary><b>Let your AI agent troubleshoot Pane itself</b> — read-only diagnostics API + troubleshooting skill</summary>

Pane ships a **read-only diagnostics API** (loopback-only, unauthenticated, **credentials never leave the machine** — URLs in error text are scrubbed too) and a troubleshooting skill. With both in place you can just ask your coding agent (Claude Code / Codex / Cursor / ZCode, …) "figure out which Pane card is broken" — it locates the cause and proposes the fix, no release cycle needed:

```powershell
curl -s --noproxy '*' http://127.0.0.1:6736/v1/doctor     # one-line health verdict + per-failure remediation
curl -s --noproxy '*' http://127.0.0.1:6736/v1/providers  # per-card status / error / staleness
curl -s --noproxy '*' http://127.0.0.1:6736/v1/status     # version / uptime / refresh age
curl -s --noproxy '*' http://127.0.0.1:6736/v1/sources    # local spend data sources
```

Install the skill one of three ways: automatically via the `install.ps1` installer, in settings → General → **AI Agent troubleshooting skill → Install**, or from the repo ([`skills/pane/`](skills/pane/); `npx skills add Aafff623/pane` works too). Full contract: [local HTTP API](docs/local-http-api.md).

</details>

<details>
<summary><b>Also</b> — onboarding, expiring list, tray, updates, settings & fonts, resize…</summary>

- **Onboarding tour** — a 5-step spotlight tour plays automatically on first install (back / next / skip, `Esc` exits); existing users are never interrupted, and settings → General → "Replay" runs it again on demand;
- **Expiring list** — summon with `T`, sorted by reset time; the top is the "act now" part;
- **Tray at hand** — right-click surfaces every provider's status row straight from the local snapshot (zero network) plus settings entries;
- **Non-intrusive updates** — a lightweight ↑ indicator on the total-spend bar when a new version lands; clicking downloads in place with a byte-accurate progress ring, then installs and restarts;
- **Settings & fonts** — every settings page shares one 940px content column and is vertically centered; the keys page gets an A–Z index; Latin and Chinese fonts are configured separately, with the stack ordered Latin → Chinese → system fallback;
- **Proportional corner resize** — enable corner handles in settings; dragging scales the window, type and icons together, remembers the size, and restores the default in one click;
- **Changelog timeline** — the bottom of the main card list shows each version's additions / changes / fixes in color, with the full log one click away;
- **Three UI languages** — 简体中文 / English / Русский, following the system or set manually;
- **Boot animation** — the brand animation when the panel opens can be turned off or replayed, freezing to a still frame under reduced-motion systems;
- **Font packs** — the same zip + manifest + sha256 verification as skins, downloaded from settings near the font picker and applied straight into the font dropdown; the official catalog is currently empty, so the recommended pick is MV Boli, which ships with Windows.

<details>
<summary><b>All nine built-in shortcuts</b></summary>

| Shortcut | Action |
| :--- | :--- |
| `Alt + 2` | Summon / dismiss the floating panel (default, customizable) |
| `Esc` | Dismiss the panel, back to work |
| `Shift` | Cycle 5-hour / 7-day / monthly quota periods |
| `Shift + 1` | Switch category tab |
| `Ctrl + S` | Open the settings panel |
| `Ctrl + E` | Open the customize drawer |
| `Ctrl + L` | Toggle light / dark theme |
| `Ctrl + R` | Refresh usage now |
| `T` | Show the expiring list |

</details>

<div align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="attachments/expiring-dark.png">
    <img src="attachments/expiring-light.png" width="390" alt="Expiring list">
  </picture>
</div>

</details>

## <a id="docs"></a>Documentation

| | |
| :--- | :--- |
| [Development startup guide](docs/dev-startup.md) | [Provider catalog](docs/providers.md) |
| [Local HTTP API](docs/local-http-api.md) | [Token spend coverage](docs/token-spend-coverage.md) |
| [Privacy notes](docs/privacy.md) | [Skin market design](docs/skin-market-design.md) |
| [Engineering facts & constraints](docs/project-facts.md) | [Domain glossary](CONTEXT.md) |
| [README asset maintenance](docs/readme-assets.md) | [Changelog](CHANGELOG.md) |
| [Provider intel](docs/provider-intel.md) | [Update mechanism](docs/update-mechanism.md) |

## <a id="privacy"></a>Privacy

Pane is a local-first tool: tokens, cookies and API keys live in the OS credential store and your user directory — never committed to Git, never uploaded; queries go only to the provider's own endpoint, and Pane has no central backend relaying your quotas; the local HTTP API listens on loopback only and redacts sensitive fields; the app sends no telemetry or analytics. Details in [`docs/privacy.md`](docs/privacy.md); disclosure process in [`SECURITY.md`](SECURITY.md).

## <a id="community"></a>Community

Issues, provider adapters, UI improvements and doc fixes are all welcome. Before submitting, state: which provider or user path the change affects, what local data or external APIs it uses, and how it was verified. New features land on `codex/<feature>` branches and merge back to `main` after verification — see [`CONTRIBUTING.md`](CONTRIBUTING.md). UI acceptance is done by the maintainer in person; real-interface evidence beats code reading.

<div align="center">

<a href="https://star-history.com/#Aafff623/pane&Date">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=Aafff623/pane&type=Date&theme=dark" />
    <img src="https://api.star-history.com/svg?repos=Aafff623/pane&type=Date" alt="Star History Chart" width="660" />
  </picture>
</a>

</div>

## <a id="license"></a>License

This project is licensed under the [MIT License](LICENSE). Copyright (c) 2026 Jazii (Pane for Windows)
