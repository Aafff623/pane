# 08 - Panel settings window close/minimize controls

Status: implemented (2026-10-25) - automated green; owner visual acceptance pending

## Problem

The red close button in the panel-form settings window appeared inert. The
frontend called `getCurrentWebviewWindow().close()` and swallowed the rejection
with an empty `.catch()`. Minimize used the same direct WebView path.

## Fix

- Close calls backend `close_panel_window`.
- Minimize calls new backend `minimize_panel_window`.
- Both errors are surfaced instead of swallowed.
- A static test locks those command paths and audits every rendered
  `data-*-close|cancel|dismiss` control for a delegated handler.

## Verification

- Frontend build and Node suite.
- Debug Pane via CDP: clicking the rendered close control removed the `panel`
  page target; clicking minimize made
  `plugin:window|is_minimized {label:"panel"}` return `true`; reopening the
  settings panel returned it to `false`.
- The installed `0.9.1` executable still carries the pre-fix bundle; it cannot
  demonstrate this change until a new build is installed.
- Owner must click close/minimize in the real panel window.
