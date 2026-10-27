# Pane — Agent Rules

> This file is auto-read by AI coding agents (Antigravity, Codex, Claude, etc.)
> on session start.  Keep it up to date whenever startup or build steps change.

## Critical: always read first
**Full startup guide → [`docs/dev-startup.md`](docs/dev-startup.md)**
**Verified engineering facts → [`docs/project-facts.md`](docs/project-facts.md)** — scope, code paths, hard constraints, failure modes, credential/vault semantics.
**Task tracker → [`docs/agents/issue-tracker.md`](docs/agents/issue-tracker.md)** — tickets live under `.scratch/` (Matt-configured 2026-10-11; the old `TODO.md` board is retired — git history and a local `temp/` archive only). Handoff parcels in `temp/handoff/` may still cite old `T-XXXX` IDs as historical links.

## TL;DR startup sequence (local development only)

```powershell
# Step 1: build
cd D:\code\pane && pnpm build

# Step 2: kill old pane + clear WebView2 cache
Stop-Process -Name pane -Force -ErrorAction SilentlyContinue
Remove-Item -Recurse -Force "$env:LOCALAPPDATA\com.aafff623.pane\EBWebView" -ErrorAction SilentlyContinue

# Step 2–3: start Vite and the debug Pane together. This is the local
# development launcher; it is not the installed release executable.
& .\temp\scripts\restart-pane-dev.ps1

# Step 4: press Alt+2 (or the configured shortcut) and verify the popover and
# the Pane tray icon on the interactive desktop.
```

## Non-negotiable rules

1. **Two processes required**: Vite on `:1420` + `pane.exe` on `:6736`.
   Neither works without the other.
   These are local development services. Published installers and release
   executables are a separate track and are not used for normal debugging.
2. **Always use `pnpm dev`** for serving the frontend — Python's `http.server`
   causes permanent WebView2 cache locks.
3. **Use `temp/scripts/restart-pane-dev.ps1` to launch `pane.exe`** — it uses
   WMI with log redirection so the process survives the Agent shell and stays
   on the interactive desktop. A bare `Start-Process` or `& pane.exe` is not a
   valid development launch path.
4. **Clear `%LOCALAPPDATA%\com.aafff623.pane\EBWebView`** whenever you see
   stale UI. WebView2 caches assets aggressively.
5. **`src-tauri/Cargo.toml` crate-type stays `["rlib"]`.** This is a
   desktop-only crate (no mobile JNI). `cdylib`/`staticlib` made GNU and
   the macOS/Linux lib targets try to link `advapi32`. Do not add them
   back.

## Port reference

| Port | Service |
|------|---------|
| `1420` | Frontend (Vite dev server) |
| `6736` | pane.exe local usage API |

## Rust changes?
```powershell
cd src-tauri
$env:PATH = "D:\Tools\mingw64\bin;$env:PATH"
cargo +stable-x86_64-pc-windows-gnu build
```
Then re-run the launch step.

## Build artifacts (`target/`)

A healthy `src-tauri/target` is ~14 GB and `parse-tests/target` ~7 GB — that is
this crate graph's dependency tree, not a defect. It only grows abnormally in two
ways, and cargo never reclaims either on its own:

- **Stale duplicate app-crate artifacts.** Every build whose flags or feature set
  differ from the last one emits a new `pane-*.exe` (~495 MB) plus
  `libpane_lib-*.rlib` (~270 MB) pair under a new hash, and the old ones stay on
  disk. Keep build invocations identical to avoid accumulating them.
- **Incremental caches.** Disabled via `[profile.dev] incremental = false` in
  both `src-tauri/Cargo.toml` and `parse-tests/Cargo.toml`. Measured: one
  app-crate rebuild writes ~460 MB of cache and saves ~37 s (81 s vs 118 s), so
  the caches had reached 23 GB. cargo still creates an empty `incremental/`.

Shrink with `cargo clean -p pane` (app crate only; the dependency tree is kept).
Avoid a full `cargo clean` unless you have time to rebuild everything — and note
that cleaning `parse-tests/target` only costs that harness's rebuild.
Local `release` profiles are never needed: releases are built by CI.

## Project map (where things live)

| Area | Location |
|------|----------|
| Frontend UI — rendering, refresh loop, settings (single-file entry point, sectioned by comment banners; by far the largest file in the repo) | `src/main.ts` |
| Provider catalog (frontend mirror of the Rust one) | `src/providerCatalog.ts` |
| Brand icons / colors | `src/providerVisuals.ts` |
| OS seam (secrets, locale, processes, paths, WebView hint) | `src-tauri/src/platform/` |
| All Tauri commands, snapshot cache, account-swap guards | `src-tauri/src/lib.rs` |
| Local CLI-log spend scanner + model pricing | `src-tauri/src/spend.rs`, `src-tauri/src/pricing.rs` |
| RTK command-output savings (CLI + project DB + daily snapshots) | `src-tauri/src/rtk_savings.rs` |
| OAuth — device-code (codex/copilot/xai) · PKCE (Cursor) | `src-tauri/src/oauth.rs`, `src-tauri/src/cursor_oauth.rs` |
| Multi-account stores (card ids are `family@<fnv1a>`) | `src-tauri/src/accounts.rs`, `antigravity_accounts.rs`, `cursor_accounts.rs` |
| Panel-window auth center — grouped account rows (read-only assembly) | `src-tauri/src/auth_center.rs` (+ `auth_center_list` in lib.rs) |
| One/New API relay subsystem | `src-tauri/src/providers/onenewapi/` |
| Local HTTP API `127.0.0.1:6736/v1/usage` (secret-redacting, tested) | `src-tauri/src/httpapi.rs` |
| Provider/parsing test harness (compiles the real sources via `#[path]`) | `parse-tests/` |
| Runtime data — never commit anything from here | `%APPDATA%\PaneAafff623\` (config.json, accounts/, onenewapi.json, last_snapshots.json, usage_history.json …) |

Domain vocabulary (the glossary) → [`CONTEXT.md`](CONTEXT.md); verified engineering
facts, constraints and failure modes → [`docs/project-facts.md`](docs/project-facts.md).

## Authoritative vs local-only files

- **Canonical startup guide**: `docs/dev-startup.md`. This file only summarizes it.
- **README.md** Features/Providers copy can lag the code — trust `src/main.ts` + `src-tauri/src/provider_catalog.rs` over README.
- **Local-only, never commit**: `.codegraph/` (auto-syncs at turn end — don't hand-sync), `.agents/mcp.json`, and the **entire `temp/` tree** (boundary docs `temp/README.md` / `temp/AGENTS.md` included — they stay on disk locally but are untracked).
- **Task tracker**: the issue-tracker authority is [`docs/agents/issue-tracker.md`](docs/agents/issue-tracker.md) — local markdown under `.scratch/` (since 2026-10-11; triage labels in `docs/agents/triage-labels.md`, domain docs in `docs/agents/domain.md`). The old `TODO.md` board was **retired on 2026-10-11**: it is archived locally at `temp/board-archive-20261011-TODO.md` (untracked) and its one open ticket was migrated into `.scratch/` — do not recreate it or mirror between the two systems. Handoff parcels still live in `temp/handoff/` and cite task IDs; there is no index file (newest handoff = latest filename timestamp).
- **Resolved (2026-09-05, amended 2026-10-11)**: `AGENTS.md`, `docs/dev-startup.md` and the governance assets (`CLAUDE.md`, `CONTEXT.md`) are tracked in Git; the `temp/` contract files were untracked on 2026-10-11 (owner decision: the whole `temp/` tree is local-only, remote has no `temp/` at all); the one-off `run_test.cmd` launcher moved to `temp/scripts/` (local-only).

## Secrets (persist on receipt)

- Any key, token, or credential that appears in a session is saved to `temp/secrets/` (Git-ignored) in the same turn — save silently, report the path, never ask for confirmation.
- Before requesting a credential, search `temp/secrets/` and Git-ignored env files first; never ask for the same value twice.
- Secrets never enter Git-tracked files, logs, or replies, and never leave this machine.

## Validation & delivery

1. Frontend: `pnpm build` must pass (tsc + vite).
2. Rust: `cargo +stable-x86_64-pc-windows-gnu check` (fast) or `build`, mingw64 PATH prepended as above.
3. Provider/parsing unit tests run through the `parse-tests` harness — the full Tauri app cannot link on this machine (no MSVC), and a GNU-linked test exe dies at startup with `STATUS_ENTRYPOINT_NOT_FOUND`:
 ```powershell
 cd parse-tests
 $env:PATH = "D:\Tools\mingw64\bin;$env:PATH"
 cargo +stable-x86_64-pc-windows-gnu test
 ```
 The harness compiles the real `src-tauri/src` files via `#[path]` plus a `tauri-stub` crate (current counts live in [`docs/project-facts.md`](docs/project-facts.md), the engineering-facts authority). It used to live under `src-tauri/target/parse-tests`, where `cargo clean` eventually ate it; it is tracked at the repo root now.
4. **Quality layer** — issue templates (`.github/ISSUE_TEMPLATE/`), root
   `.editorconfig`, `.github/dependabot.yml` and `.github/workflows/codeql.yml`
   are the governance baseline (issue #52). Local re-check = the YAML files
   parse; `gh workflow view codeql`, the new-issue template page and Dependabot
   PRs only apply once the files reach the default branch. Rust is deliberately
   NOT in the CodeQL matrix (left to cargo-audit, separate decision).
5. **UI acceptance is done by the user personally.** Agents deliver build/test evidence plus a short acceptance checklist — never claim "done and verified" from code reading alone.
6. Non-trivial changes get a code-review pass plus a redundancy/simplifier scan before delivery, then re-test.
7. **Release authorization gate** — local implementation and validation must finish first; the user must personally perform UI acceptance and give explicit authorization in the conversation. Until both conditions are present, do not create a release, tag, commit, or push. A build, port check, screenshot, or agent opinion is not user acceptance.
8. **Release format** — every published release must follow [`docs/release-format.md`](docs/release-format.md): theme headline + download matrix table (system × architecture, with real sizes) on top, then `## 新增` / `## 变更` / `## 修复` sections written per the wording rules. `CHANGELOG.md` is the single source of content and is reconciled before tagging. `release.yml` must pass the body via `--notes-file`; bare `--generate-notes` (compare-link-only body) is a release-process defect.
   **Two release modes, decided by the tag alone**: `v0.8.4` = formal (becomes GitHub Latest — updater, website download, winget and the CN mirror all follow it); `v0.9.0-rc.1` (a hyphen in the version) = preview (same installers, marked pre-release, so no auto-update, no website download, no winget, no mirror — share the link with testers). Details and the version-number policy: release-format §6.

## Uncertainty & existing work

- Never invent endpoints, file paths, response shapes, or UI copy — read the code first; anything unverifiable gets marked `待确认` in [`docs/project-facts.md`](docs/project-facts.md).
- Check `git status` + recent commits before editing; leave unrelated work-in-progress alone.
- New features go on a `codex/<feature>` branch and merge back to `main` only after verification.
- Destructive operations (delete/overwrite/clean) require the user's explicit confirmation first.
