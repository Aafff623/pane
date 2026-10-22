# 05 — Android minimal exploration

Type: task
Status: backlog
Blocked by: owner starting the exploration

## Goal

Prove the Tauri 2 Android chain with a read-only/display surface. No desktop
feature parity.

## Steps

- Verify Android SDK/adb and whether Android Studio is installed.
- `tauri android init` and build a minimal WebView shell.
- Read one existing snapshot or a fixed fixture; no account writes.
- Check storage, network, background behavior and package signing on an emulator
  and, if available, a real device.

## Explicitly out

- Tray, popover, always-on-top, skipTaskbar, single instance, autostart.
- Mobile UI redesign.
- Entering the 0.8.x release line.
