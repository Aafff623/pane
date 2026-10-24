# 05 - Merge RTK into Total Spend and add harness setup

Status: implemented (2026-10-24) - automated green; owner visual acceptance pending

## Scope

- Add RTK savings as the third Total Spend metric beside dollars and tokens.
- Keep RTK out of the donut, spend ledger, quotas and daily summary.
- Show the RTK metric in its normal accent when installed and muted when missing.
- When missing, switch the card body to setup guidance with:
  - the official Windows `winget` package `rtk-ai.rtk`;
  - the RTK website link;
  - the full supported harness list, disabled until RTK is installed.
- Run harness setup only through a fixed backend whitelist mapped to
  `rtk init -g ... --auto-patch`.
- Follow `showTotalSpend`: hiding Total Spend also hides RTK.
- Configure RTK in a popup dialog with larger text and each harness's brand
  icon. Show a green status dot for detected integrations and red for missing.
- The dialog follows CC Switch's visual grammar: modal header with brand tile,
  bordered white-card sections, neutral icon buttons, thin dividers and larger
  body text.
- Detection has two levels:
  - exact: known hook/config file contains the official `rtk hook <tool>`
    marker;
  - fuzzy: a known config file/directory contains an RTK plugin, rules file or
    integration marker. Exact is checked first.
- Manual refresh forces a new RTK read; the card's main figure uses two-decimal
  `M` formatting so small additions are visible.
- Codex's current hook matcher is `Bash`; the dialog states that PowerShell
  commands do not increase RTK counts.
- Every command row is grouped by channel (`rtk_cmd`) and source
  (`project_path`). Top + other + unclassified rows close to the CLI total.
  Harness provenance is not stored by RTK and is therefore not invented.

## Supported harness targets

Codex, Claude Code, Cursor, Trae, Windsurf, Cline, Kilo Code, Antigravity,
Kimi Code, GitHub Copilot, Gemini CLI, OpenCode, Factory Droid, Mistral Vibe,
Hermes, Pi, Oh My Pi.

This list comes from `rtk init --help` and `rtk hook --help` (RTK 0.50.0 on
this machine; winget currently offers 1.0.1).

## Verification

- Rust: fixed target list and command arguments, no arbitrary target strings.
- Frontend: build and all Node tests.
- Live: installed state, selected RTK metric, project breakdown, management
  panel with 17 targets, and missing state with muted tab and install guide.
- Owner: visual acceptance still required.
