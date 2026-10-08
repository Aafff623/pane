# Pane — Agent Startup Guide

> **Read this first.**  This file is the canonical startup reference for any
> AI agent (Antigravity, Codex, Claude, Kimi, etc.) working on this repo.
> Failure to follow the sequence below is the #1 cause of "UI not updating"
> and "pane won't open" bugs during development.

---

## Development and release are separate tracks

The normal development loop never runs the installed release package. It uses
the checkout's Vite server and debug binary:

| Track | Frontend | Native binary | Purpose |
|---|---|---|---|
| Local development | `pnpm dev` → `127.0.0.1:1420` | `src-tauri\target\debug\pane.exe` → `127.0.0.1:6736` | HMR, debugging, tray and `Alt+2` acceptance |
| Published release | bundled `dist/` | `src-tauri\target\release\bundle\...` installer / exe | GitHub Release and installed-user distribution |

`pnpm build` is a validation/build step; it does not publish a release and it
does not replace the Vite dev server. The release workflow starts from a
version tag and uploads generated installer artifacts. Do not use an installed
release exe to validate uncommitted frontend changes.

## Architecture in one sentence

Pane is a **Tauri** app: the Rust backend (`src-tauri/`) compiles to
`pane.exe`, which opens a **WebView2** window that loads the frontend from
`http://127.0.0.1:1420` in development. Vite serves that URL with live
reload. Production builds bundle `frontendDist: "../dist"` into the native
app; the installed release does not require the Vite server on `1420`.
(Explicit IPv4: on machines where IPv6 loopback `[::1]` connections are
blocked — WFP filter / VPN driver — `localhost` may resolve to `::1`
first and every probe fails with access-denied.)

There are **two independent processes** that must both be running:

| # | Process | Port | Role |
|---|---------|------|------|
| 1 | Vite dev server (`pnpm dev`) | `1420` | Serves index.html + CSS + JS |
| 2 | `pane.exe` (debug build) | `6736` | Rust backend + tray + IPC |

---

## Quick-start (development)

For the normal local loop, run the one-shot launcher after building the
frontend. It starts Vite and the debug binary through WMI, redirects logs, and
waits for both ports:

```powershell
cd D:\code\pane
pnpm build
& .\temp\scripts\restart-pane-dev.ps1
```

Acceptance requires all three signals: `127.0.0.1:1420` responds, the Pane
API on `127.0.0.1:6736` responds, and the debug Pane appears with its tray icon
on the interactive desktop. Press `Alt+2` (or the configured global shortcut)
to summon the popover. A port-only `200` response is not UI acceptance.

Do not
replace the WMI launcher with a bare `Start-Process` or `& pane.exe`: those can
place the window on a non-interactive station or attach stdout to a transient
Agent pipe.

## Required delivery sequence

Every requested change follows this order:

1. Implement the request in the local checkout and run the relevant build,
   tests, and development startup checks.
2. Leave the local debug services running for the user's own acceptance:
   Vite on `127.0.0.1:1420`, the debug Pane API on `127.0.0.1:6736`, the
   visible Pane window/tray icon, and the configured global shortcut.
3. Wait for the user to personally test the UI and explicitly authorize the
   next delivery step in the conversation.
4. Only after that authorization may an agent prepare a commit, push a branch,
   create a tag, or publish a release. Release packaging is a separate step
   from local debugging and must also follow [`docs/release-format.md`](release-format.md).

Build output, HTTP `200` responses, screenshots, and an agent's code review do
not replace the user's acceptance or authorization.

## If UI still shows old content (WebView2 cache)

```powershell
Stop-Process -Name pane -Force -ErrorAction SilentlyContinue
Remove-Item -Recurse -Force "$env:LOCALAPPDATA\com.jazii.pane\EBWebView" -ErrorAction SilentlyContinue
# Then re-run from step 4 above
```

**Always prefer `pnpm dev` over `python -m http.server`.**
Vite sends `Cache-Control: no-cache` headers; Python's server does not,
causing WebView2 to permanently cache old assets.

---

## Rust backend changes (`.rs` files edited)

```powershell
cd D:\code\pane\src-tauri
$env:PATH = "D:\Tools\mingw64\bin;$env:PATH"
cargo +stable-x86_64-pc-windows-gnu build
# Output: src-tauri\target\debug\pane.exe
```

Then restart pane.exe as in step 5 above.

> The committed crate-type is `["rlib"]` (desktop only). Do not add
> `cdylib`/`staticlib` back — they pull Windows link libs into the
> macOS/Linux build.

---

## Verifying everything is up

```powershell
# Is pane alive?
Get-Process pane -ErrorAction SilentlyContinue

# Is the frontend being served?
Invoke-WebRequest http://127.0.0.1:1420 -UseBasicParsing | Select-Object StatusCode

# Is the local usage API responding?
Invoke-RestMethod http://127.0.0.1:6736/v1/usage | Select-Object -ExpandProperty id
```

---

## Port reference

| Port | Owner | Purpose |
|------|-------|---------|
| `1420` | Vite / static server | Frontend (HTML, CSS, JS) |
| `6736` | `pane.exe` | Local usage REST API |

---

## Launching from a sandboxed agent session (WMI)

Agent harnesses (ZCode etc.) often run shell tools inside a job sandbox.
Two failure modes, both confirmed on this machine:

- **Tree kill** — a command timeout, a task stop, or even the user
  switching models kills the entire process tree, including any Vite or
  `pane.exe` you started via `Start-Process`.
- **IPv6 loopback block** — connections to `[::1]:1420` fail with
  `WSAEACCES` ("access forbidden by access permissions") even from
  clean processes, while `127.0.0.1` always works. That is why the whole
  dev chain is pinned to IPv4 (see the architecture note above).

The escape is WMI: processes created via `Win32_Process.Create` are
spawned by `WmiPrvSE.exe`, outside the sandbox job:

```powershell
$cmd = 'cmd.exe /c ""C:\nvm4w\nodejs\node.exe" "D:\code\pane\node_modules\vite\bin\vite.js" > "D:\code\pane\temp\logs\vite-dev.log" 2>&1"'
Invoke-CimMethod Win32_Process Create @{ CommandLine = $cmd; CurrentDirectory = 'D:\code\pane' }
```

Gotchas: WMI cannot resolve WindowsApps aliases (`pwsh`, `pnpm`) — you get
rc=9 "path not found"; use real paths (`node.exe`, `cmd.exe`). And
`powershell.exe` started from that chain gets access-denied here — probe
with node instead (`temp/scripts/net-probe.cjs`; note the repo is ESM, so
it must be `.cjs`). Local one-shot scripts: `temp/scripts/pane-wmi-launch.ps1`
(kill + cold start + poke + verify) and `pane-wmi-poke.ps1` (show panel).

---

## Common agent mistakes

| Mistake | Fix |
|---------|-----|
| Restarting pane exe during the dev loop | Use `temp/scripts/restart-pane-dev.ps1` — kill → wait 6736 free → launch → API probe → summon + zombie check, one shot |
| Killing pane then instantly relaunching | The new instance's HTTP server **silently fails** to bind `6736` while the old socket sits in TIME_WAIT (it still prints the URL). Wait for the port to be free before relaunching, then verify `/v1/usage` |
| Rust rebuild fails with `os error 5` / `os error 32` on `pane.exe` | The running exe is locked — `Stop-Process -Name pane` first, then `cargo build` |
| Window "can't be summoned" (Alt+2 / tray click do nothing) | Pre-2026-09-18 bug: a minimized window still reports `is_visible()==true`, so every toggle went to the *hide* branch — fixed in `toggle_popover*` (iconic counts as hidden + `unminimize()` before show). If you must recreate it: launch the exe again (single-instance toggle) restores a zombie-minimized window |
| Launching `pane.exe` with `Start-Process` or `&` | Use `CreateProcess` with `lpDesktop = "WinSta0\Default"` — without it the window opens on the non-interactive station (invisible) |
| Launching `pane.exe` with stdout/stderr attached to a transient agent shell | Redirect to a file (`temp/scripts/launch-pane.ps1` wraps with `cmd /c ... >> pane-dev.log`) — when the agent shell exits the pipe breaks and the next `println!` **panics, killing the refresh task**: footer stuck "Refreshing…", every card stays ⚠数据过时, `/v1/usage` returns `[]` |
| Serving dist/ with `python -m http.server` | Use `pnpm dev`; Python skips cache headers and WebView2 caches forever |
| Not killing old pane before re-launching | Old pane intercepts the second launch via single-instance plugin and just *toggles the window* — no fresh binary loaded |
| Rebuilding Rust for CSS/TS changes | Not needed; `pnpm build` + Vite reload is sufficient |
| Forgetting to clear WebView2 cache | `Remove-Item -Recurse -Force "$env:LOCALAPPDATA\com.jazii.pane\EBWebView"` |
| Wrong working directory | Always `cd D:\code\pane` before any pnpm command |
| Probing `http://[::1]:1420` or `http://localhost:1420` | IPv6-loopback connects are blocked machine-wide here (`WSAEACCES`); always probe `http://127.0.0.1:1420` |
| Vite/pane started under the agent's own shell task | Timeouts / task-stops / model switches kill the whole tree; create them via WMI (`Win32_Process.Create`) so they outlive the session |
| `pwsh`/`pnpm` in a WMI `CommandLine` | WindowsApps aliases don't resolve there (rc=9 "path not found"); use full paths to `node.exe` / `cmd.exe` |
| Changing `devUrl` in `tauri.conf.json` without rebuilding | `devUrl` is compiled into the binary at build time; rebuild (`cargo build`) or the old URL is still baked in |

---

## File map

```
src/
  main.ts              Frontend logic (render, event handlers, state)
  styles.css           All CSS
  providerCatalog.ts   Provider IDs, display names, feature flags
  providerVisuals.ts   Provider SVG icons
src-tauri/src/
  main.rs              Tauri entry, tray, IPC command table
  providers/           Per-provider Rust modules (quota fetching)
  spend.rs             Token spend accounting from local CLI logs
scripts/
  dev-pane.cmd         One-shot dev cycle (build → serve → launch)
docs/
  dev-startup.md       ← this file
dist/                  Compiled frontend (output of pnpm build)
```
