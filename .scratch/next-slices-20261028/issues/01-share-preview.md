# 01 · Share preview (minimal) for #106

Status: done — implemented 2026-10-28; owner visual acceptance pending | Priority: P1 | Depends: none

## Result (2026-10-28)

- `renderElementSnapshot` now returns a PNG data URL; the preview modal renders it instead of copying silently.
- Provider cards and the total-spend range menu both open the same preview with Copy image / Save PNG / Close, visible success/error text, Esc/backdrop close and focus return.
- `save_share_image` writes to the Downloads folder with a sanitized, non-overwriting file name; `share_image.rs` has three local tests for path stripping, Unicode/empty fallback and unique names.
- Real-app check: total-spend preview (今天) copied to clipboard as 1040×636 PNG and saved to `C:\Users\Lenovo\Downloads\pane-share-总花费-今天-20261028T1420.png`; provider-card preview opened for DeepSeek and copied successfully.
- Gates: `pnpm build`; Node 251/251; site 55/55; parse-tests full run 1202 passed + 1 known proxy flake that passed solo, 18 ignored; the new `share_image` tests passed.
- Not yet done: 125% scale check and owner visual acceptance.

## Deep-review fixes (2026-10-28)

- `wb-rows-head`/`wb-row` no longer share one grid; the header cannot put "credits" over the model column.
- Unknown day buckets are filtered before the bar chart (no "wn" label); WorkBuddy unit header is localized.
- Closing/reopening a preview now dismisses the previous overlay and its document keydown listener; focus returns to the exact share button via an explicit trigger, including the total-spend range path.
- Copy failures use a dedicated `share.copyFailed` key; `card.share` now says "Preview share card", and `share.previewTitle` (dead) is removed.
- Share buttons are visible on keyboard focus; PNG save names avoid Windows reserved names; preview stage uses the available window height at 100%/132% scale.

## Build

- Refactor the existing snapshot renderer so it returns a PNG data URL instead of only writing the clipboard.
- Provider-card share button opens a modal preview (current card, current theme) with: Copy image, Save PNG, Close.
- Total-spend share keeps its range menu; choosing a range opens the same preview.
- Visible success/failure feedback inside the modal and near the button; no more silent copy.
- Fix the total-spend share button's mouse activation; keyboard activation already works.
- Two themes only: follow the current light/dark theme. No 7-theme studio.
- Add `save_share_image` Rust command using the existing dialog plugin and the same PNG size/pixel validation as `copy_share_image`.
- i18n keys in en/zh/ru; `type="button"` on share buttons; Esc/backdrop close; focus returns to the share button.

## Done when

- Clicking a provider card share button shows a preview; Copy puts the same PNG on the clipboard; Save writes the chosen file.
- Total-spend share menu opens with the mouse and the chosen range previews the same numbers as the envelope.
- Failure shows a visible message, not just the bottom status bar.
- Real-app screenshot on Windows WebView2 at 100% and 125% scale; owner acceptance.
- `pnpm build`, Node tests, site tests and parse-tests green.

## Out of scope

- 7 themes, share presets, history, social upload, analytics.
