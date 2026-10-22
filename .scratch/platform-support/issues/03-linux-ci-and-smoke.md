# 03 — Linux build and smoke

Type: task
Status: backlog
Blocked by: platform differences inventory

## Environment

- Ubuntu 22.04 CI job (already in release.yml) or WSL2 Ubuntu.
- Packages: `libwebkit2gtk-4.1-dev`, `libayatana-appindicator3-dev`,
  `librsvg2-dev`, `patchelf`.
- Real desktop: WSLg or a Linux VM/desktop; not a headless container alone.

## Acceptance

- AppImage and deb each install and launch once.
- Tray icon appears and opens the popover; single instance and autostart work.
- Secret Service available → secrets persist; unavailable → readable fallback,
  never a fake success.
- Window flags (frameless, skipTaskbar, always-on-top) behave on at least one
  compositor.
- Record the exact distro, desktop environment and compositor.

## Out of scope

Full feature parity, packaging for every distro, signing.
