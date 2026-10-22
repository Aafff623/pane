# 05 — Four-platform plan (#36)

Status: in-progress
Blocked by: None

## Deliverable

`docs/platform-support.md` plus child tickets under
`.scratch/platform-support/issues/`.

## Must cover

- Windows: keep local development and NSIS release.
- Linux: WSL2/Docker build, Ubuntu CI, WebKitGTK/GTK/AppIndicator deps, one real
  desktop smoke, AppImage/deb naming.
- macOS: Actions runner, arm64 dmg, screenshot acceptance, signing and
  notarization as a separate gate.
- Android: Tauri 2 minimal proof; read-only/display first; tray/popover out.
- Platform difference matrix: Windows-only APIs, tray, windows, paths, single
  instance, autostart, signing.
- Prerequisites: WSL Ubuntu absent, Android Studio unverified, Actions quota,
  macOS signing, release asset naming.

## Acceptance

- Each platform has an executable child ticket with environment and acceptance.
- No version or schedule promise is written.
- Owner confirms the route before implementation starts.
