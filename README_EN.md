<div align="center">

<img src="src/assets/pane-logo.png" width="110" height="110" alt="Pane Logo" />

# Pane

### A local-first usage and quota workspace that lives in your tray

**Usage, quotas, balances and resets — one desktop workspace.**

<br/>

**[English](README_EN.md)** • **[简体中文](README.md)**

[Website](https://pane.threetwoa.live/) • [English website](https://pane.threetwoa.live/en/) • [GitHub](https://github.com/Aafff623/pane)

<br/>

[![Release Version](https://img.shields.io/github/v/release/Aafff623/pane?style=flat-square&label=Release&color=2563EB&labelColor=172033)](https://github.com/Aafff623/pane/releases)
[![Platform](https://img.shields.io/badge/Platform-Windows%2010%20%7C%2011%20(x64)-0078D4.svg?style=flat-square&logo=windows)](https://github.com/Aafff623/pane/releases)
[![Tauri](https://img.shields.io/badge/Tauri-v2-24C8DB.svg?style=flat-square&logo=tauri&logoColor=white)](https://tauri.app/)
[![Rust](https://img.shields.io/badge/Rust-172033.svg?style=flat-square&logo=rust&logoColor=white)](src-tauri/Cargo.toml)
[![TypeScript](https://img.shields.io/badge/TypeScript-3178C6.svg?style=flat-square&logo=typescript&logoColor=white)](package.json)
[![Vite](https://img.shields.io/badge/Vite-646CFF.svg?style=flat-square&logo=vite&logoColor=white)](vite.config.ts)
[![Providers](https://img.shields.io/badge/Providers-66-2563EB.svg?style=flat-square)](docs/providers.md)
[![Tests](https://img.shields.io/badge/Tests-parse--tests-2563EB.svg?style=flat-square)](parse-tests/)
[![License](https://img.shields.io/badge/License-MIT-10B981.svg?style=flat-square)](LICENSE)
[![Language](https://img.shields.io/badge/Language-zh--CN%20%7C%20en%20%7C%20ru-8B5CF6.svg?style=flat-square)](#i18n)
[![Local First](https://img.shields.io/badge/Data-Local--First-172033.svg?style=flat-square)](docs/privacy.md)

<br/>

[📖 Introduction](#intro) • [🎬 Live demo](#demo) • [🌟 Overview](#highlights) • [✨ Features](#features) • [🔌 Supported sources](#providers) • [🚀 Quick start](#download) • [🛠️ Build & develop](#build) • [📂 Project layout](#structure) • [🔐 Privacy](#privacy) • [📋 Changelog](CHANGELOG.md)

</div>

---

## <a id="intro"></a>📖 Introduction

**Pane** is a local-first **usage and quota workspace**. It brings **usage, remaining quotas, balances, reset times and account status** from different tools and online services into one on-demand desktop panel for a consolidated view and account management.

The current catalog contains **66 sources**, spanning AI coding and model services, productivity tools such as speech services, and search, crawling and MCP services. Supported local CLIs also contribute **tokens and spend**. Available metrics depend on each source's API or local data.

Future integrations focus on queryable usage and resources: automation and workflows, office and business SaaS, cloud compute and databases, cloud drives and storage, network traffic and digital memberships are all candidates. Readable usage, quota, balance or expiry data provides a basis for evaluating an integration; access permissions, APIs and measurement rules still need to be checked and adapted. These directions do not imply current support.

Press `Alt + 2` to bring the panel up over any window, `Shift` to cycle **5-hour / 7-day / monthly** periods in the same overview, and `Esc` to get back to work. It never interrupts your flow; it only gives you accurate context when you need it.

> 💡 **Design priorities**:
> - **Local first**: tokens, cookies and API keys stay in the OS credential store and your user directory. Nothing is uploaded, and no Pane relay backend sits in the middle — queries go straight to each provider's endpoint.
> - **One desktop layer**: quota lookups, reset countdowns, the multi-account pool, total spend and key management all live in one panel, with settings as a standalone window rather than a web console.
> - **Resident, not intrusive**: lives in the tray, auto-refreshes every 5 minutes by default (adjustable), and reports failures as explicit error states instead of greyed-out unknowns.

## <a id="demo"></a>🎬 Live demo

Don't want to install first? The website ships an interactive demo **driven by a real build of the frontend** — nine shortcuts (`Alt + 2`, `Shift`, `Ctrl + S`, …), light/dark switching and the mobile layout can all be tried directly. The demo data is fixed and never connects to a real account; the desktop build supports global shortcuts.

<div align="center">

[![Live demo](https://img.shields.io/badge/Pane-🎬%20Open%20live%20demo%20pane.threetwoa.live-2563EB?style=for-the-badge&logo=googlechrome&logoColor=white)](https://pane.threetwoa.live/en/)

</div>

## <a id="highlights"></a>🌟 Overview

Pane compresses "how much is left, when does it reset, which account is close to the cap, how much did I spend today" into one quiet tray tool. The panel shows only the most important state by default; details expand when you need them.

| Capability | What you get |
| :--- | :--- |
| **Quota overview** | 5-hour / 7-day / monthly period switching, green/yellow/red states, reset countdowns, failures reported as errors |
| **Local spend** | Token and USD spend read back from local CLI logs, per-model breakdown, tiered icons |
| **Account pool** | Multiple accounts per provider, independent cards, two-level notes, default pin and archiving |
| **Categories & groups** | Coding Agent / Productivity / MCP-search tabs, customizable annotations, in-drawer search |
| **Key management** | Stored in the OS credential store, plaintext revealed after master-password verification, local login-state detection |
| **Shortcuts** | 9 built-in shortcuts, full-row conflict detection, per-key rebinding and one-click reset |
| **Theme & looks** | Light/dark themes, boot brand animation, experimental skin market (hidden by default) |
| **Provider access** | 66 providers, three auth shapes: local login state, OAuth device flow, API key |

> 📋 For a fuller version-by-version history, see [CHANGELOG.md](CHANGELOG.md).

---

## <a id="features"></a>✨ Features

### 1. 📊 Quota, all in one view

- Session (5-hour), weekly (7-day) and monthly quota are unified in one overview, switchable with `Shift`;
- Green, yellow and red states quickly separate available, peak-hour and unavailable;
- Failed lookups show as explicit error states instead of a greyed-out unknown;
- Providers without a monthly quota fall back to weekly in the monthly view, and the fallback is labelled.

<div align="center">
  <img src="attachments/overview-dark.png" width="420" alt="Pane quota overview panel" />
  <br/><sub>Static capture of the current frontend with sample data; no real accounts are connected.</sub>
</div>

---

### 2. 🧾 Spend, traceable

- Reads supported local CLI logs to compute today, yesterday and the last 30 days of spend, never through any cloud service;
- Supports both USD and token views with per-model breakdowns;
- Keeps the token fact and marks un-priced items when a model has no known price;
- Spend-card icons tier by daily token volume, with thresholds customizable in settings;
- How each CLI is measured and what is covered is documented in [Token spend coverage](docs/token-spend-coverage.md).

<div align="center">
  <img src="attachments/spend-dark.png" width="420" alt="Pane spend detail panel" />
  <br/><br/>
  <img src="attachments/heatmap-dark.png" width="720" alt="Pane spend heatmap" />
  <br/><sub>Spend and heatmap use sample data. Static images are used for now and can be replaced with GIFs later.</sub>
</div>

---

### 3. ⏱️ Reset, no more guessing

- Every window shows remaining quota plus a recovery countdown;
- **Wall mirroring**: when the weekly/monthly quota is exhausted, the short-period ring mirrors the wall state (100% + refresh countdown), eliminating the "the 5-hour ring is still green but every request is rejected" illusion (Kimi first, other families extended after measured verification);
- Exhaustion comes with a readable explanation (e.g. `Weekly limit reached · resets in Xd Yh`) instead of a bare red ring with empty details.

---

### 4. 🧩 Account pool, many accounts per provider

- API-key providers can store multiple accounts in settings, each rendered as an independent card;
- Every account has a stable local card identity — deleting or reordering never cross-links cards;
- Notes are two-level: "Provider name" only renames the card title, "sub-account name" only renames that account;
- Right-click an account capsule to pin it as the default main account, archive or restore it (credentials follow);
- The overview collapses to "main-account ring + usage bar + N/M badge" by default and expands on click.

---

### 5. 🗂️ Categories, annotations and card grouping

- The tab bar switches between **Coding Agent / Productivity / MCP-search** categories;
- Annotations are customizable so you can organize the board around your own workflow;
- The Customize drawer's search box filters in place by name, and group headers hide when empty;
- Every provider can be removed from the board, with "Removed providers" at the bottom ready to restore.

---

### 6. ⌨️ Shortcuts, at hand

| Shortcut | Action |
| :--- | :--- |
| `Alt + 2` | Summon / dismiss the floating panel (default, customizable) |
| `Esc` | Dismiss the panel and return to work |
| `Shift` | Cycle the 5-hour / 7-day / monthly quota periods |
| `Shift + 1` | Switch category tabs |
| `Ctrl + S` | Open the settings panel |
| `Ctrl + E` | Open the custom board |
| `Ctrl + L` | Toggle light/dark theme |
| `Ctrl + R` | Refresh usage now |
| `T` | View the expiring list |

- Shortcuts render as keycaps: click to record, `Esc` to cancel, `Backspace` to clear, `↺` to restore the built-in default;
- Conflicts are checked across the whole row and flagged in red; when the OS takes a binding, the last working one is kept.

---

### 7. 🔐 Keys, kept on this machine

- Provider master keys and all sub-account keys live in the **OS credential store** (Windows Credential Manager and equivalents); JSON files no longer store plaintext on disk;
- Every row on the "Keys & quota" page requires **master-password verification** before plaintext is shown, and plaintext can be copied in one click;
- Local-login providers (Copilot / Antigravity / Claude, …) show a green "Connected" inline and state which local files or configs are read.

---

### 8. 🎨 Themes & looks

- One-tap light/dark switching, with card-entrance and day/night transitions switchable;
- The main window plays a boot brand animation, switchable and replayable in settings, and holds on a film frame before exiting when the OS asks for reduced motion;
- **The skin market is experimental and hidden by default** — enabling it needs a confirmation in settings, and turning it off hot-swaps back to the native skin.

---

### 9. 🔌 Providers, read by auth shape

- The catalog covers **66 providers**; a fresh install enables 11 mainstream families by default (Claude, Codex, Cursor, Grok, Z.ai, …), the rest opt-in from settings;
- Three auth shapes: local login state, OAuth device flow, API key;
- The card menu's "⚡ Test connection" also works on disabled cards, with the status bar showing `OK · N ms` or the failure reason;
- "? How quota is queried" automatically offers key / OAuth / local step templates plus console links, with an embedded "Test now".

---

### 10. 🔄 Tray & updates

- The tray menu lists each provider's status straight from local snapshots (zero network requests) plus a settings entry;
- When a new version is detected, a lightweight ↑ indicator appears on the total-spend bar without an interrupting dialog; clicking it downloads in place, the ring fills by real byte progress, and the app auto-installs and restarts when done.

---

## <a id="providers"></a>🔌 Supported sources

Pane currently covers 66 providers. The most common categories are listed below; the full documentation is in the [Provider catalog](docs/providers.md).

The count of 66 refers to the current source catalog; released installers follow their own version's coverage. Available quota, balance and local spend data vary by source.

| Type | Sources | How it is read |
| --- | --- | --- |
| Subscription quota | Claude, Codex, Cursor, GitHub Copilot | Local login state + official usage API |
| API-key quota | DeepSeek, Kimi Code, StepFun, SiliconFlow, Novita | Key from settings, environment variable or CLI config |
| Multi-account pool | ClinePass, One/New API, Custom Relay, … | Independent accounts in `accounts/<provider>.json` |
| Local spend | OpenCode, Claude Code, Codex CLI, Hermes, … | Local database or CLI logs |
| Local service | Ollama | Model and service status on `127.0.0.1:11434` |

ClinePass uses an `sk_` key to query three rolling windows — five-hour, weekly and monthly; multiple keys under the same provider are shown separately, with the account count noted near the card.

<details>
<summary><b>All 66 providers</b> (click to expand — matches <code>src/providerCatalog.ts</code>)</summary>

Claude, Codex, CodeBuddy, Cursor, OpenCode, Copilot, Grok, Devin, MiniMax, OpenRouter, Z.ai, Antigravity, DeepSeek, Kimi API, ElevenLabs, Ollama, Codebuff, Kilo, AihubMix, One/New API, Qwen Code, Hermes, Kimi Code, StepFun, StepFun Step Plan, Shandianshuo (闪电说), SiliconFlow, Novita AI, Custom Relay, Qoder CN, Trae CN, Command Code, Doubao, ClawsGO, BochaAI, Tavily, Firecrawl, Brave Search, ClinePass, SenseNova, APIGOTO, MiniMax Code, Amp, AWS Bedrock, Chutes, Deepgram, Kiro, OpenAI API, Poe, Venice, Vertex AI, Warp, Windsurf, MiMo, Trae, Qoder, Zed, Droid, JetBrains AI, Groq, Hugging Face, LongCat, sub2api, Mistral, Perplexity, Volcengine Ark

</details>

---

## <a id="download"></a>🚀 Quick start & download

### Current mainline version: `0.5.0`

| Platform | Installer | Status | Download |
| :--- | :--- | :--- | :--- |
| **Windows 10 / 11 (x64)** | `*-setup.exe` (NSIS installer) | 🟢 Primary development & verification platform | [⬇️ Download latest](https://github.com/Aafff623/pane/releases/latest) |
| **macOS** | `.dmg` | 🟡 Builds published with each Release | [⬇️ Go to downloads](https://github.com/Aafff623/pane/releases) |
| **Linux** | `.AppImage` / `.deb` | 🟡 Builds published with each Release | [⬇️ Go to downloads](https://github.com/Aafff623/pane/releases) |
| **Older releases** | All historical artifacts | 📂 Version rollback & comparison | [📂 Browse Releases archive](https://github.com/Aafff623/pane/releases) |

### Basic usage flow:

1. Download the installer for your platform from [Releases](https://github.com/Aafff623/pane/releases) and launch it — Pane goes straight into the system tray;
2. Press `Alt + 2` (or click the tray icon) to summon the panel; it appears centered on the desktop;
3. Open settings (`Ctrl + S`) and complete local login or paste an API key per provider; cards that are not enabled can also be verified first with "⚡ Test connection";
4. Use `Shift` to switch between 5-hour / 7-day / monthly periods and confirm quota and reset times;
5. Press `Esc` to dismiss the panel and return to work.

Pane needs no separate account registration. After the first run, data lives under the current user's `%APPDATA%\Pane\`.

### Run from source

```powershell
git clone https://github.com/Aafff623/pane.git
cd pane
pnpm install
pnpm dev
```

Development mode needs two processes: Vite on `127.0.0.1:1420` for the frontend, and `pane.exe` on `127.0.0.1:6736` for the local usage API. The full Windows startup and WebView2 cache instructions are in [`docs/dev-startup.md`](docs/dev-startup.md).

---

## <a id="i18n"></a>🌐 Interface languages

The interface language can be switched at any time in Settings → General:

| Code | Display name | Support |
| :--- | :--- | :---: |
| `zh` | 🇨🇳 简体中文 | 🟢 Full |
| `en` | 🇺🇸 English | 🟢 Full |
| `ru` | 🇷🇺 Русский | 🟢 Full |
| `auto` | 🖥️ Follow the OS language | 🟢 Full |

The website has Chinese and English entries: [`/`](https://pane.threetwoa.live/) and [`/en/`](https://pane.threetwoa.live/en/). Both languages are supported in the current source; live updates take effect with site deployment.

---

## <a id="build"></a>🛠️ Build & develop

### Requirements

- Windows 10 / 11 (x64)
- [Node.js](https://nodejs.org/) 20+ with `pnpm`
- [Rust](https://rustup.rs/) stable toolchain
- WebView2 Runtime (usually preinstalled on Windows 10/11)

### Compile & run

```powershell
# 1. Clone the repository
git clone https://github.com/Aafff623/pane.git
cd pane

# 2. Install frontend dependencies
pnpm install

# 3. Frontend only (TypeScript check + Vite build)
pnpm build

# 4. Full desktop app (frontend + Rust)
pnpm tauri build
```

### Rust-side check

```powershell
cd src-tauri
$env:PATH = "D:\Tools\mingw64\bin;$env:PATH"
cargo +stable-x86_64-pc-windows-gnu check
```

### Run the provider parsing tests

`parse-tests` compiles the real sources under `src-tauri/src` directly via `#[path]`, paired with a `tauri-stub` crate, so it runs without linking the full Tauri app:

```powershell
cd parse-tests
$env:PATH = "D:\Tools\mingw64\bin;$env:PATH"
cargo +stable-x86_64-pc-windows-gnu test
```

---

## <a id="structure"></a>📂 Project layout

```text
pane/
├── src/                             # Frontend (TypeScript + Vite, no framework)
│   ├── main.ts                      # Board rendering, refresh loop, settings, skin market
│   ├── i18n.ts                      # Three-language dictionary (zh / en / ru)
│   ├── providerCatalog.ts           # Frontend provider capability catalog
│   ├── providerVisuals.ts           # Provider icons, colors and brand visuals
│   ├── providerMechanisms.ts        # Per-provider read-mechanism copy
│   ├── peakHours.ts                 # Peak-hour detection
│   ├── uiIcons.ts                   # UI icons
│   └── styles.css                   # All styles (incl. narrow-window breakpoints)
├── src-tauri/                       # Desktop side (Rust / Tauri v2)
│   ├── src/lib.rs                   # Tauri commands, snapshot cache, account-swap guards
│   ├── src/providers/               # Per-provider quota query adapters
│   ├── src/accounts.rs              # API-key account pool and stable card identity
│   ├── src/keyvault.rs              # Master key and key vault
│   ├── src/secretstore.rs           # OS credential store wrapper
│   ├── src/spend.rs / pricing.rs    # Local CLI spend scanner and model pricing
│   ├── src/httpapi.rs               # Local API (127.0.0.1:6736/v1/usage)
│   ├── src/telemetry.rs             # Anonymous telemetry (can be disabled)
│   ├── src/platform/                # OS seam (secrets, locale, processes, paths)
│   └── src/*_login.rs / oauth.rs    # Per-provider login and OAuth device flow
├── parse-tests/                     # Provider parsing test carrier (compiles real sources)
├── site/                            # Website (zero-framework single file + Cloudflare Worker)
├── docs/                            # Startup, privacy, provider and design docs
├── attachments/                     # README screenshots
├── CHANGELOG.md                     # Full version changelog
├── CONTRIBUTING.md                  # Contribution guide
└── LICENSE                          # MIT License
```

---

## <a id="privacy"></a>🔐 Privacy

Pane is a local-first tool:

- Tokens, cookies and API keys stay in the Windows user directory and OS credential store, never committed to Git;
- Queries are only sent to the corresponding provider's endpoints; Pane has no central backend relaying your quota for you;
- The local HTTP API listens on loopback only and redacts sensitive fields;
- Telemetry never carries quota, spend or provider credentials, and can be disabled in settings.

See [`docs/privacy.md`](docs/privacy.md); for the security disclosure process, see [`SECURITY.md`](SECURITY.md).

---

## <a id="contributing"></a>🤝 Contributing

Issues, provider adapters, UI improvements and documentation revisions are all welcome. Please state before submitting:

1. Which provider or user path your change affects;
2. Which local data or external APIs it uses;
3. How to verify it, and which parts still need manual acceptance.

New features go on a `codex/<feature>` branch and merge back to `main` after verification. See [`CONTRIBUTING.md`](CONTRIBUTING.md).

### Related documentation

- [Development startup guide](docs/dev-startup.md) — Windows dual-process, cache and `pane.exe` launch
- [Provider catalog](docs/providers.md) — provider read methods and field notes
- [Local HTTP API](docs/local-http-api.md) — `127.0.0.1:6736/v1/usage`
- [Privacy](docs/privacy.md) — credentials, network requests and telemetry boundaries
- [Skin market design](docs/skin-market-design.md) — wallpapers, mascot and exit interaction
- [CONTEXT.md](CONTEXT.md) — verified domain facts and engineering constraints

---

## <a id="license"></a>📄 License

Released under the [MIT License](LICENSE).

Copyright (c) 2026 Jazii (Pane for Windows)
