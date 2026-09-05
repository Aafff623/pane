# Pane — Agent Rules

> This file is auto-read by AI coding agents (Antigravity, Codex, Claude, etc.)
> on session start.  Keep it up to date whenever startup or build steps change.

## Critical: always read first
**Full startup guide → [`docs/dev-startup.md`](docs/dev-startup.md)**

## TL;DR startup sequence (frontend changes only)

```powershell
# Step 1: build
cd D:\code\pane && pnpm build

# Step 2: kill old pane + clear WebView2 cache
Stop-Process -Name pane -Force -ErrorAction SilentlyContinue
Remove-Item -Recurse -Force "$env:LOCALAPPDATA\com.jazii.pane\EBWebView" -ErrorAction SilentlyContinue

# Step 3: start Vite dev server (NOT python http.server)
# Run in a separate/background shell:
pnpm dev

# Step 4: launch pane.exe on the interactive desktop
# Must use CreateProcess with lpDesktop="WinSta0\Default"
# See docs/dev-startup.md for the full PowerShell snippet
```

## Non-negotiable rules

1. **Two processes required**: Vite on `:1420` + `pane.exe` on `:6736`.
   Neither works without the other.
2. **Always use `pnpm dev`** for serving the frontend — Python's `http.server`
   causes permanent WebView2 cache locks.
3. **Launch `pane.exe` via `CreateProcess(WinSta0\Default)`** — using
   `Start-Process` or `&` makes the window appear on a non-interactive
   station (invisible to user).
4. **Clear `%LOCALAPPDATA%\com.jazii.pane\EBWebView`** whenever you see
   stale UI. WebView2 caches assets aggressively.
5. **Do not commit `crate-type = ["rlib"]`** in `src-tauri/Cargo.toml` —
   that is a local MinGW workaround only.

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

## Project map (where things live)

| Area | Location |
|------|----------|
| Frontend UI — rendering, refresh loop, settings (single file, ~5k lines) | `src/main.ts` |
| Provider catalog (frontend mirror of the Rust one) | `src/providerCatalog.ts` |
| Brand icons / colors | `src/providerVisuals.ts` |
| All Tauri commands, snapshot cache, account-swap guards | `src-tauri/src/lib.rs` |
| Local CLI-log spend scanner + model pricing | `src-tauri/src/spend.rs`, `src-tauri/src/pricing.rs` |
| OAuth — device-code (codex/copilot/xai) · PKCE (Cursor) | `src-tauri/src/oauth.rs`, `src-tauri/src/cursor_oauth.rs` |
| Multi-account stores (card ids are `family@<fnv1a>`) | `src-tauri/src/accounts.rs`, `antigravity_accounts.rs`, `cursor_accounts.rs` |
| One/New API relay subsystem | `src-tauri/src/providers/onenewapi/` |
| Local HTTP API `127.0.0.1:6736/v1/usage` (secret-redacting, tested) | `src-tauri/src/httpapi.rs` |
| Runtime data — never commit anything from here | `%APPDATA%\Pane\` (config.json, accounts/, onenewapi.json, last_snapshots.json, usage_history.json, telemetry.json …) |

Verified domain facts, vocabulary, and hard constraints → [`CONTEXT.md`](CONTEXT.md).

## Authoritative vs local-only files

- **Canonical startup guide**: `docs/dev-startup.md`. This file only summarizes it.
- **README.md** is upstream-facing copy inherited from ItsJazii/pane; its Features/Providers text can lag this fork — trust `src/main.ts` + `src-tauri/src/provider_catalog.rs` over README.
- **Local-only, never commit**: the `crate-type = ["rlib"]` line in `src-tauri/Cargo.toml` (rule 5 above), `.codegraph/` (auto-syncs at turn end — don't hand-sync), `.agents/mcp.json`, and everything under `temp/` except its `README.md` / `AGENTS.md`.
- **Resolved (2026-09-05)**: `AGENTS.md`, `docs/dev-startup.md` and the governance assets (`CLAUDE.md`, `CONTEXT.md`, `temp/` contract files) are tracked in Git; the one-off `run_test.cmd` launcher moved to `temp/scripts/` (local-only).

## Validation & delivery

1. Frontend: `pnpm build` must pass (tsc + vite).
2. Rust: `cargo +stable-x86_64-pc-windows-gnu check` (fast) or `build`, mingw64 PATH prepended as above.
3. Provider/parsing unit tests run through the scratch harness — the full Tauri app cannot link on this machine (no MSVC):
   ```powershell
   cd src-tauri\target\parse-tests
   $env:PATH = "D:\Tools\mingw64\bin;$env:PATH"
   cargo test
   ```
   The harness compiles the real `src-tauri/src` files via `#[path]` plus a `tauri-stub` crate.
4. **UI acceptance is done by the user personally.** Agents deliver build/test evidence plus a short acceptance checklist — never claim "done and verified" from code reading alone.
5. Non-trivial changes get a code-review pass plus a redundancy/simplifier scan before delivery, then re-test.

## Uncertainty & existing work

- Never invent endpoints, file paths, response shapes, or UI copy — read the code first; anything unverifiable gets marked `待确认` in `CONTEXT.md`.
- Check `git status` + recent commits before editing; leave unrelated work-in-progress alone.
- New features go on a `codex/<feature>` branch and merge back to `main` only after verification.
- Destructive operations (delete/overwrite/clean) require the user's explicit confirmation first.
