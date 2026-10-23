# 07 — 窗口比例、字体与状态预览

Status: done (continuous screen-relative scale and menu regressions) — owner acceptance pending
GitHub: none (follow-up to #41/#44 review)
Priority: P1 for acceptance clarity
Blocked by: owner UI acceptance

## Current contract

The earlier 80–120% percentage-pill design is superseded. Keep the existing state preview and finish the current working-tree UI:

- Default logical size 560×800; scale levels 100/125/150/175%.
- One gear menu contains scale, small/medium/large text, pin, lock and reset.
- General settings in both floating and panel windows expose scale and text size.
- Win32 resize keeps the existing center.
- Overview bar layout uses three columns at 150/175%; two columns at 100/125%.

## Changes in this handoff round

- Removed panel-only early returns that made scale/text settings inert. Both surfaces persist through the same operations; the Rust resize command targets the main window.
- Centralized discrete-level snapping at the scale setter, config fallback and gear slider. Legacy fractional values cannot leak back into saved settings.
- Replaced the physical-width media-query trigger with an explicit window-scale class. CSS body zoom otherwise keeps the content viewport close to base width, so width alone cannot reliably enable three columns. Text size does not change the selected column mode.
- Kept the already-implemented 560×800 baseline, gear menu, centered Win32 SetWindowPos and state preview. No redesign of completed work.

## Comments

2026-10-23: Regression-first tests reproduced three failures: fractional scale accepted, panel scale ignored, panel text ignored. After correction the shared setters pass from both surfaces. The Node suite also covers the four levels, text-size independence and three-column mode.

- `corepack pnpm build`: passed.
- `node --test tests/*.test.mjs`: 104 passed, 0 failed.
- GNU debug build: passed, with existing warnings.
- `git diff --check` for edited sources: passed.
- Standard dev launcher restarted the rebuilt executable. Native geometry reports 560×800; Vite and the local usage API respond. Floating window is hidden, so menu actions, center movement, fonts and borders have not been visually exercised in this round.
- Final serial, proxy-free parse-tests: 1025 passed / 0 failed / 14 ignored, 393.08 s. Log: `temp/logs/account-ui-regression-final.log`.

## Owner acceptance

- Summon with Alt+2. Confirm the gear and other header icons are vertically centered.
- Switch 100/125/150/175% in both gear menu and General settings. Only these four values should appear, the center should stay fixed, and restart should preserve the choice.
- Switch small/medium/large text independently of window scale. Check headings, email labels, tabs and menus for clipping.
- At 150/175% confirm three overview columns and aligned separators; at 100/125% confirm two columns.
- Verify pin, lock and reset, and open the existing synthetic state preview (four states plus five tray colors).

Actual Windows DPI/tray sizing and final visual acceptance remain owner checks. No release authorization is implied.

### Final verification (2026-10-23)

- Final full parse-tests: **1025 passed / 0 failed / 14 ignored**, 393.08 s; doc-tests 0 failed. Command: `cargo +stable-x86_64-pc-windows-gnu test --manifest-path parse-tests/Cargo.toml -- --test-threads=1`, MinGW PATH prepended and proxy environment cleared.
- Multi-source regressions pass 2/2 (parameterized across five families). The Kiro fixture uses the harness's mock secret slot directly, since Kiro does not expose the generic OAuth logout API. Earlier failed fixture runs remain recorded; assertions were retained.
- Final Node suite: 104/104. Frontend build and GNU debug build pass; existing build warnings remain. Edited-source whitespace check passes.
- Review and simplification: checked retirement/suppression ordering, managed re-login, default-id dedupe, source marker compatibility and panel config synchronization; unified all scale snapping through the existing helper and avoided a new general account abstraction.
- Running debug app and Vite remain available. Actual GUI interaction is still unverified and remains owner acceptance. No commit/push/tag/release.

## Reopened: default placement drifts (2026-10-23)

Owner screenshot shows the popover near a screen edge with clipped content. Default summon must center horizontally and vertically in the monitor work area, and resizing must not preserve accidental startup offsets. Existing code unconditionally restores a saved top-left even while locked, remembers automatically positioned windows on dismiss, uses different tray/hotkey fallback placement, and resizes around the old center before startup size restoration is complete.

Regression first: locked default ignores stale saved corners; tray and shortcut share placement; work area excludes taskbar; oversize window fits; unlocked manual position remains clamped and usable; repeated size cycles do not move the default center.

### Position-drift correction and evidence

- Shared physical-pixel geometry now handles standard tray/hotkey/second-instance summon, size restoration, resizing and reset. Locked/default placement ignores stale saved offsets and centers within the monitor work area. Unlocked custom positions remain supported and are clamped to reachable bounds.
- Default dismiss no longer records an automatically chosen offset; minimized parking coordinates are never remembered. Unminimize happens before geometry restoration. The main size is restored before show, so the later WebView startup resize cannot change its center.
- Oversized scale levels are limited to the available work-area dimensions. The window can scroll rather than placing controls outside the screen.
- Frontend lock returns to the centered default and clears its stale anchor; reset uses the same native centering command.
- Regression-first pure geometry: 4 failed / 1 passed before correction, then 5/5 passed. Cases cover saved corners, negative-monitor coordinates, taskbar work area, oversized levels, unlocked custom bounds and repeated scale cycles.
- Real native probe before rebuilding: window `(201,196,840,1200)` inside work area `(0,0,2560,1600)`; horizontal center error **659 px**, vertical error 4 px. This reproduced the owner symptom independently of the synthetic tests.
- After GNU debug rebuild and two standard-script restarts (different process ids): both probes report `(860,200,840,1200)`, horizontal and vertical center error **0 px**, fully inside work area. Probe: `temp/scripts/check-pane-centering.ps1` (read-only native geometry); it returns nonzero on drift or off-screen bounds.
- `corepack pnpm build` passed; `node --test tests/*.test.mjs` passed **106/106**; GNU debug build passed; edited-source `git diff --check` passed. This round ran the five targeted Rust placement regressions, not another full provider/crypto suite. Previous full 1025/0/14 evidence predates this correction.
- Final visual interaction, actual scale-menu cycling, manual dragging and multi-monitor/DPI transitions remain owner acceptance. Computer Use did not obtain a visible Pane target; native coordinate checks do not substitute for visual acceptance.
- No commit/push/tag/release. Unrelated working-tree changes remain in place.

## Owner follow-up: continuous scale and menu clipping

Supersedes fixed levels and fixed 560×800 size: new 100% is about 80% of the former default on the reference screen; expose every integer percentage 80–175 with five guide ticks. Native dimensions derive from monitor work area, not hardcoded pixel dimensions. Existing legacy scale settings migrate once to new 100%. Preserve centered placement.

Fix slider stutter by coalescing native resize requests and persisting on gesture completion, not on every pointer event. Font changes must reflow/reposition the popup inside the window. Scroll must keep it attached to the gear or dismiss it when the gear leaves view.

### Continuous scale, responsive default and menu correction (2026-10-23)

This supersedes the earlier four-level and fixed-native-size behavior. The owner's follow-up is implemented as: new default **100% = approximately 80% of the former default** on the reference screen; the selectable interval is every integer percentage **80–175**. Five labels are guide ticks, not snap targets.

- Native baseline uses **26.25% of work-area width × 60% of work-area height** at 100%, then multiplies by the selected factor. Maximum dimensions stop at 96% of the work area to keep controls reachable. This is separate from Windows DPI: native placement uses physical work-area coordinates, and typography uses logical viewport width. The frontend's 560-unit width is a layout reference only, not a native window dimension.
- One-time `windowScaleVersion=2` migration resets old fixed-pixel scale settings to new 100% and discards old absolute sizes; later percentages survive reload. Font preference remains unchanged. The local pre-migration config backup is `temp/secrets/window-scale-before-v2.json`.
- Gear and General settings both use an 80–175 range with step 1. Resize previews are throttled to 32 ms, serialized with latest-value coalescing, and do not persist settings or repaint the whole dashboard. Gesture completion/closing a dirty popup persists the final setting. Resize events no longer race the slider by writing absolute dimensions.
- Font zoom now reflows body width as well as height. Popup max width/height are constrained, coordinates recomputed on font/viewport changes, and external scroll tracks the gear. When the gear leaves the viewport, dismiss the menu. Listener/timer cleanup happens on close.
- Found and fixed the existing anchor discriminator: DOMRect also has `x`, so checking `x in anchor` incorrectly treated a gear rectangle as a cursor point. The helper now checks rectangle bounds and correctly aligns beneath the gear.

Validation:

- Red first: continuous-percentage tests failed on 80%/147%; the responsive native-size test failed because it still returned fixed 560×800. After correction: `cargo ... test ... popover_geometry -- --test-threads=1` **7 passed**, including migration, two monitor sizes, screen bounds and repeated positioning.
- `node --test tests/*.test.mjs`: **110 passed / 0 failed**. Covers all 96 integer percentage values, serialized/coalesced requests, no preview writes/full repaint, and DOMRect anchoring. `corepack pnpm build`: passed. GNU debug build: passed with existing warnings.
- Ego Lite Chromium fixture imports the actual menu functions and project CSS. DOM checks passed at 320×480, 448×640, 540×720 and 780×960 for small/medium/large text. Popup stayed inside viewport; scrolling moved it by the gear's displacement and dismissed it once the gear left. Body width fills the viewport with both sub-100% and larger zoom. Evidence: `temp/logs/window-menu-browser-evidence.json` and `temp/scripts/window-menu-browser-07.mjs` / `08.mjs`.
- Burst fixture: **96 input events → 1 resize request + 1 settings write**, final readout 175%. This checks event coalescing, not real desktop frame rate. Browser screenshot capture was unavailable while Ego's task window was minimized; this is DOM/layout evidence, not screenshot acceptance.
- Real rebuilt Pane native geometry on 2560×1600 work area: 80% → **538×768**, 100% → **672×960**, 117% → **786×1123**, 175% → **1176×1536**; each inside work area with center error 0–0.5 px. Tested non-default percentages by changing only scale settings while the dev process was stopped, then using the standard launcher. Finally restored new 100%. Logs: `temp/logs/window-scale-{80,117,175}-native.json`.
- Full provider/crypto Rust suite was not rerun for this UI correction. Previous full-suite evidence predates it. No commit/push/tag/release.

Owner check: drag slowly through 80/93/117/146/175 and back; observe frame smoothness and final persistence, change all text sizes without right-edge clipping, scroll while the gear popup is open, and confirm new 100% is visibly smaller. Multi-monitor transitions and actual desktop drag frame rate remain unverified.

## 2026-10-23 close-out review

- Removed the now-unused `set_popover_size` command and its registration: geometry is driven only through `set_popover_scale`, and its 300×400 logical floor contradicted the screen-relative basis. Its doc block had also been spliced between the panel-window comment and `open_panel_window`, so that comment is restored to its own function.
- Dropped a dead `clearTimeout` handle in `bindPopoverScaleSlider`, one redundant in-memory `popoverSize` write, and merged two identical persist guards in the scale queue. Node suite 127 passed; `corepack pnpm build` passed.
- Restarted the dev app for owner acceptance after the rebuild. Native probe on this machine's 2560×1600 work area: `806×1152` at `(877,224)` with center error **0 px**, fully inside the work area. The saved level had moved to **120%** in the owner's own testing between probes, which is exactly what 26.25%×60% of the work area times 1.2 yields — the earlier `914×1306` reading was the 136% level. Both are screen-relative, not fixed pixels.
