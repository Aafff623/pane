@echo off
REM Run from repo root: scripts\dev-pane.cmd
REM
REM One-shot dev cycle: build the frontend and launch Pane. Pane's
REM debug build talks to whatever is on http://localhost:1420 - we
REM serve dist/ there via Python, so changes to src/ are picked up
REM after a build without the user having to start vite manually.
REM
REM Why this exists: Pane's debug binary reads devUrl (localhost:1420),
REM not ../dist. Forgetting to start vite used to leave the UI frozen on
REM whatever the user saw before. This script always shows the latest code.

setlocal EnableExtensions

REM Resolve repo root from this script's location.
pushd "%~dp0.." 2>nul
if errorlevel 1 (
  echo [dev-pane] cannot resolve repo root from %~dp0
  exit /b 1
)
set "REPO=%CD%"
popd

if exist "D:\Tools\mingw64\bin" set "PATH=D:\Tools\mingw64\bin;%PATH%"
if exist "%USERPROFILE%\.cargo\bin" set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
set "PATH=%REPO%\src-tauri\target\debug;%REPO%\src-tauri\target\debug\deps;%PATH%"

echo [dev-pane] repo: %REPO%

REM Verify the toolchain pieces exist before we touch anything destructive.
set "TOOLS_MISSING="
where /q pnpm
if errorlevel 1 set "TOOLS_MISSING=%TOOLS_MISSING% pnpm"
where /q cargo
if errorlevel 1 set "TOOLS_MISSING=%TOOLS_MISSING% cargo"
where /q python
if errorlevel 1 (
  where /q python3
  if errorlevel 1 set "TOOLS_MISSING=%TOOLS_MISSING% python"
)
if not "%TOOLS_MISSING%"=="" (
  echo [dev-pane] missing on PATH:%TOOLS_MISSING%
  exit /b 1
)

REM Stop any leftover dev server on 1420 + any running Pane.
echo [dev-pane] stopping existing pane + 1420 listener
for /f "tokens=5" %%a in ('netstat -ano ^| findstr ":1420 " ^| findstr LISTENING') do (
  taskkill /F /PID %%a >nul 2>&1
)
taskkill /F /IM pane.exe >nul 2>&1

REM 1. Build the frontend so dist/ is up to date.
echo [dev-pane] pnpm build
pushd "%REPO%"
call pnpm build
if errorlevel 1 (
  echo [dev-pane] frontend build failed
  popd
  exit /b 1
)
popd

REM 2. Start a static HTTP server on 1420 that serves dist/. Python's built-in
REM    http.server blocks under Pane's webview concurrency and stops serving
REM    requests while the socket stays LISTENING - the window then renders
REM    as bare HTML with no JS/CSS. Try npx serve (Node keep-alive, ok on
REM    Windows), fall back to python with explicit IPv4 bind.
echo [dev-pane] starting static server on 1420 -> %REPO%\dist
where /q npx
if not errorlevel 1 (
  start /B "pane-dev-1420" npx --yes serve -l tcp://127.0.0.1:1420 -s "%REPO%\dist"
) else (
  start /B "pane-dev-1420" python -m http.server 1420 --bind 127.0.0.1 --directory "%REPO%\dist"
)
for /L %%i in (1,1,10) do (
  netstat -ano | findstr ":1420 " | findstr LISTENING >nul
  if not errorlevel 1 goto bound1420
  ping -n 2 127.0.0.1 >nul
)
echo [dev-pane] failed to bind 1420
exit /b 1
:bound1420

REM 3. Build + launch the Pane binary.
echo [dev-pane] cargo build
pushd "%REPO%\src-tauri"
cargo +stable-x86_64-pc-windows-gnu build %CARGO_FLAGS%
if errorlevel 1 (
  echo [dev-pane] cargo build failed
  popd
  exit /b 1
)
popd

echo [dev-pane] launching target\debug\pane.exe
start "" "%REPO%\src-tauri\target\debug\pane.exe"
endlocal
