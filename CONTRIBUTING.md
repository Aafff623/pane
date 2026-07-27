# Contributing

Thanks for caring about Pane! A few ground rules keep this easy for
everyone.

## Issues first

Open an issue before writing code — especially for new providers or
features. It saves you from building something that won't be merged.
Bug reports with reproduction steps are always welcome.

## Security issues

Never in public issues — use [SECURITY.md](SECURITY.md) (GitHub private
vulnerability reporting).

## Pull requests

- Keep PRs focused: one change per PR.
- Describe **what was happening** and **what this changes** in plain
  English; screenshots for anything visual.
- Provider PRs must follow the house rules: credentials are read only
  from where the official tool already stores them, and are sent only to
  that vendor's own API. Every new provider gets a section in
  [docs/providers.md](docs/providers.md) documenting exactly what it
  reads and calls.
- No telemetry, analytics SDKs, or "phone home" code — PRs adding any
  will be declined regardless of intent. Exactly two deliberate,
  maintainer-shipped exceptions exist, both documented field-by-field in
  [docs/privacy.md](docs/privacy.md): the update check (anonymous daily
  install counting, country-level, no IPs stored) and the opt-out daily
  usage statistic (`src-tauri/src/telemetry.rs` — random install ID,
  daily rollups, error categories only, hard-stop toggle). Those
  exceptions are not a precedent: users' quotas, usage amounts, spend,
  and error text never leave their PC, and PRs widening what either
  channel carries — or adding any third-party analytics SDK — will be
  declined.
- No new dependencies without a stated reason.

## Building

```
npm install
npm run tauri dev     # run with hot reload (frontend)
npm run tauri build   # produce the installer
```

Rust changes need a rebuild + relaunch of the app — it's a long-lived
tray process.
