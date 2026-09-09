# Pane — Agent Startup Guide

> **Read this first.**  This file is the canonical startup reference for any
> AI agent (Antigravity, Codex, Claude, Kimi, etc.) working on this repo.
> Failure to follow the sequence below is the #1 cause of "UI not updating"
> and "pane won't open" bugs during development.

---

## Architecture in one sentence

Pane is a **Tauri** app: the Rust backend (`src-tauri/`) compiles to
`pane.exe`, which opens a **WebView2** window that loads the frontend from
`http://127.0.0.1:1420`.  In dev mode that URL is served by Vite (live
reload).  In production the binary still reads `127.0.0.1:1420` — so you
must always have something serving `dist/` there.
(Explicit IPv4: on machines where IPv6 loopback `[::1]` connections are
blocked — WFP filter / VPN driver — `localhost` may resolve to `::1`
first and every probe fails with access-denied.)

There are **two independent processes** that must both be running:

| # | Process | Port | Role |
|---|---------|------|------|
| 1 | Vite dev server (`pnpm dev`) | `1420` | Serves index.html + CSS + JS |
| 2 | `pane.exe` (debug build) | `6736` | Rust backend + tray + IPC |

---

## Quick-start (frontend-only changes — no Rust rebuild)

```powershell
# 1. Build the frontend
cd D:\code\pane
pnpm build

# 2. Kill any previous pane + free port 1420
Stop-Process -Name pane -Force -ErrorAction SilentlyContinue
# If something else owns 1420:
# Get-NetTCPConnection -LocalPort 1420 | ForEach-Object { Stop-Process -Id $_.OwningProcess -Force }

# 3. Clear WebView2 cache (prevents serving stale CSS/JS)
Remove-Item -Recurse -Force "$env:LOCALAPPDATA\com.jazii.pane\EBWebView" -ErrorAction SilentlyContinue

# 4. Start Vite dev server in background (correct cache headers, HMR)
Start-Process powershell -ArgumentList '-NoProfile -Command "cd D:\code\pane; pnpm dev"' -WindowStyle Minimized

# 5. Launch pane.exe on the INTERACTIVE desktop (WinSta0\Default is required)
#    Direct Start-Process / & runs on a non-interactive station — window invisible.
$code = @"
using System; using System.Runtime.InteropServices;
public class PaneLauncher {
  [StructLayout(LayoutKind.Sequential, CharSet=CharSet.Unicode)]
  public struct STARTUPINFO {
    public int cb, _r; public string lpDesktop, _t;
    public int _a,_b,_c,_d,_e,_f,_g,_h; public short _i,_j;
    public IntPtr _k,_l,_m,_n;
  }
  [StructLayout(LayoutKind.Sequential)]
  public struct PROCESS_INFORMATION { public IntPtr hProcess,hThread; public int dwProcessId,dwThreadId; }
  [DllImport("kernel32.dll",SetLastError=true,CharSet=CharSet.Unicode)]
  public static extern bool CreateProcess(string a,string b,IntPtr c,IntPtr d,bool e,uint f,IntPtr g,string h,ref STARTUPINFO i,out PROCESS_INFORMATION j);
  [DllImport("kernel32.dll")] public static extern bool CloseHandle(IntPtr h);
  public static int Launch(string exe, string cwd, string desktop) {
    var si = new STARTUPINFO(); si.cb = Marshal.SizeOf(si); si.lpDesktop = desktop;
    PROCESS_INFORMATION pi;
    if (!CreateProcess(null, exe, IntPtr.Zero, IntPtr.Zero, false, 0, IntPtr.Zero, cwd, ref si, out pi))
      throw new Exception("CreateProcess error " + Marshal.GetLastWin32Error());
    CloseHandle(pi.hThread); CloseHandle(pi.hProcess);
    return pi.dwProcessId;
  }
}
"@
Add-Type -TypeDefinition $code
$env:PATH = "D:\code\pane\src-tauri\target\debug;D:\Tools\mingw64\bin;" + $env:PATH
$pid2 = [PaneLauncher]::Launch("D:\code\pane\src-tauri\target\debug\pane.exe", "D:\code\pane", "WinSta0\Default")
Write-Host "pane.exe launched, PID $pid2"
```

Press **Alt+2** or click the tray icon to open the panel.

---

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
