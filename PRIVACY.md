# Privacy

**The authoritative privacy document is [`docs/privacy.md`](docs/privacy.md).**
This file is a pointer plus the short version; if the two ever disagree,
`docs/privacy.md` wins.

## The short version

- **No Pane account, no Pane backend.** Your quotas, spend, keys and provider
  data are never sent to us.
- **No telemetry, no analytics SDK, no crash reporter.** This build sends no
  usage statistics to anyone.
- **Requests that leave your machine**: the provider's own API (only for
  providers you enabled, with that account's own credential), public price
  tables, this project's update manifest, an optional public status-page probe
  when a Claude card is stale or failed, and your own local Ollama if enabled.
  The complete table is in [`docs/privacy.md`](docs/privacy.md).
- **Never sent**: credentials, quotas, usage amounts, spend, model names from
  your logs, prompts, file paths, account identities, window titles or
  clipboard contents.
- **Local storage**: `%APPDATA%\PaneAafff623` (config, accounts, snapshots,
  spend ledger, skins, font packs). Secrets go to the OS credential store
  first. Deleting that folder erases Pane's local memory.
- **Local HTTP API**: `127.0.0.1:6736`, loopback-only, Host-checked, no CORS,
  never serves credentials — see [`docs/local-http-api.md`](docs/local-http-api.md).

Website versions: [中文](https://pane.threetwoa.live/privacy.html) ·
[English](https://pane.threetwoa.live/en/privacy).
