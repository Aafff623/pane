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
- Owner must click close/minimize in the real panel window.
