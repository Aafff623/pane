# 02 — Platform differences inventory

Type: task
Status: ready-for-agent
Blocked by: None

## Scope

Walk the code and record every Windows-shaped assumption in one list:

- platform seam calls (`src-tauri/src/platform/`), DPAPI / Credential Manager;
- tray menu, popover window flags, skipTaskbar, always-on-top, drag/reset;
- autostart, single-instance, path resolution, console-window suppression;
- updater signing vs OS code signing;
- any `cfg(windows)` branch outside `platform/`.

For each item: current behavior, cross-platform risk, and whether the other
platform needs the same responsibility or a different one.

## Acceptance

- Matrix in `docs/platform-support.md` matches the code (no invented rows).
- Every Windows-only call has either a Unix implementation or an explicit
  "Windows-only, unsupported elsewhere" note.
- No platform code changes in this ticket.
