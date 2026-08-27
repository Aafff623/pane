# Roadmap — full Mac parity (and beyond)

**Status (v0.4.44, 2026-08-27): every wave below is shipped, and the
post-launch releases keep going.** Pane has full feature parity with the
macOS original plus 21 providers, signed auto-updates published by CI,
Codex reset-credit redemption, update-check-on-open with a one-click
footer button, live model pricing that re-prices within the hour when
new models appear, modern Cursor plan support, and a Mac-parity design
pass (inset card composition, wedge spend donut with dollars ⇄ tokens,
in-popover drag reorder, curated share cards). What comes next is
demand-driven — open an issue for the provider or feature you're
missing. Candidates on deck: Windsurf and JetBrains AI providers,
long-context pricing tiers, a Re-detect Tools button, tray "Bars" icon
style, code signing.

---

Original plan below, kept for history. Feature audit vs
robinebers/openusage @ 4d75562 (2026-07-06). Waves are ordered by
dependency: the pace engine (Wave 1) feeds notifications (Wave 2);
structured metric data from Wave 1 also unlocks Wave 4's toggles. Each
wave ends with a shipped, installed build.

## Wave 1 — Pace engine (the brain)
- [x] Structured metric data: add `resets_at` (ms) + `period_ms` to Metric,
      emitted by every provider (replaces the preformatted reset string).
- [x] Burn-rate math: used% vs elapsed% of window → projection at reset.
- [x] Verdicts: blue (≥10% spare projected), yellow (<10% spare, "~3% spare"
      note), red (projected to run out — flame + "Limit in 3h 5m"),
      "Limit reached" when actually spent. No-reset metrics color by level
      (yellow ≥80% used, red ≤10% left).
- [x] Even-pace tick on the bar; hover shows projection at reset.

## Wave 2 — Notifications (Windows toasts)
- [x] tauri-plugin-notification + permission flow.
- [x] Alerts: Almost Out (<10% left), Cutting It Close (projected thin),
      Will Run Out (projected over) — once per metric per reset period,
      only when a quota *worsens* while running. State in config dir.
- [x] Three Settings switches, default off (like the Mac).

## Wave 3 — Usage Trend + spend depth
- [x] 30-day per-day token bar chart per provider (spend engine already
      buckets by day — expose the series, draw SVG bars).
- [x] Hover: peak day, date range, source note.
- [x] Per-model spend breakdown (extend spend engine to aggregate by model;
      hover a spend row → ranked model list with share bars, "Other" tail).
- [x] Total Spend polish: persist tab choice, hover exact total, ⓘ listing
      contributors, minimum visible sliver for tiny spenders.

## Wave 4 — Interaction polish
- [x] Click headline → flip Used ⇄ Left everywhere (persisted).
- [x] Click reset label → flip countdown ⇄ exact time (persisted).
- [x] 30-second local tick: countdowns/pace re-render between refreshes.
- [x] Time format setting (Auto / 12h / 24h).

## Wave 5 — Customize
- [x] Per-metric show/hide.
- [x] Always Visible vs On Demand (caret expander on cards, state persists).
- [x] Drag-to-reorder providers and metrics (HTML5 drag & drop).
- [x] Stars (≤2 per provider) drive the tray strip, replacing the plain
      trayProviders picker; strip follows customize order.
- [x] Per-provider Reset and Reset All (with confirm).
- [x] Stretch: Ctrl+Z undo for customization steps.

## Wave 6 — Platform features
- [x] Provider quick links (Status / Console / Dashboard buttons per card).
- [x] First-launch detection: fresh installs enable only providers whose
      local credentials exist.
- [x] Share screenshot: render a branded PNG of a card (canvas) → clipboard.
- [x] Local HTTP API on 127.0.0.1:6736 (GET /v1/usage, /v1/usage/:id).
- [x] Theme setting: System / Light / Dark (light palette via CSS vars).
- [x] Density: Default / Compact.
- [x] Global shortcut to toggle the popover (tauri-plugin-global-shortcut).
- [x] Proxy setting for all provider requests.

## Wave 7 — Antigravity (research project)
- [x] Discover Antigravity's local language-server process on Windows
      (process scan → ports + CSRF token from its command line).
- [x] Call RetrieveUserQuotaSummary / GetUserStatus locally (method names
      and parsing ported from the Mac source).
- [x] Fallback chain like the Mac: quota summary → legacy endpoints.

## Wave 8 — Ship v0.2.0
- [x] Version bump, changelog, rebuild installers, silent reinstall.
- [x] GitHub release, auto-updater, winget submission (in review).

All waves shipped 2026-07-07 as v0.2.0; public launch followed as v0.4.0.
Remaining backlog: OpenCode official *balance* API when it ships
(anomalyco/opencode#10448 — the account-wide *usage* API landed in Pane
0.4.34; balance would add dollar amounts to the account-wide view).
Long-context pricing tiers shipped in 0.4.9.

## Deliberately not ported
- PostHog SDK telemetry — reconsidered 2026-07-27: Pane ships its own
  minimal, SDK-free, opt-out daily statistic instead
  (src-tauri/src/telemetry.rs; see docs/privacy.md), keeping the same
  daily-rollup restraint as upstream without the analytics dependency.
