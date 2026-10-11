# Update mechanism

How Pane ships and applies its own updates, who verifies what, and what
happens when the network path fails. Release *content* rules live in
[release-format.md](release-format.md); this file is about the channel.

## Channel

- Manifest: `https://github.com/Aafff623/pane/releases/latest/download/latest.json`
  — the latest release's signed manifest, produced by `release.yml` on
  every tag.
- Installer: the manifest's `platforms["windows-x86_64"].url` points at the
  release asset `Pane_<version>_x64-setup.exe` (NSIS), with the detached
  minisign signature beside it (`.sig`).
- Trust: the embedded per-asset signature is verified against the minisign
  public key baked into `tauri.conf.json` (`plugins.updater.pubkey`). The
  private signing key lives in the `TAURI_SIGNING_PRIVATE_KEY` repo secret
  (local backup `temp/updater-keygen.txt`) — losing it kills update signing.
- The detached upstream's trypane.xyz / ItsJazii endpoints were dropped:
  their builds are signed with a key this app no longer trusts, so querying
  them could only produce signature failures.

## Flow (Windows)

1. **Check** — `check_update` (lib.rs) runs on every popover open, from the
   Settings button, and from the 4-hourly background loop
   (`spawn_update_checker`, emits `update-available`). A hit raises the
   footer button and the once-per-version popup (`maybePromptUpdate`).
   A failed check stays silent — offline is a normal state, not an error.
2. **Install** — `install_update` re-runs the check first (the promised
   version may have been yanked; `None` becomes an explicit error so the
   button never sticks), then `download_and_install` streams byte progress
   as `update-progress` (the spend-header ring), verifies the minisign
   signature, runs the NSIS installer and restarts the app.
3. **Failure UX** — the invoking button re-enables and the reason appears
   where the prompt happened (popup body / footer status line). Raw reqwest
   transport errors ("error sending request for url …") are translated to
   the localized network hint (`update.networkHint`) with the original
   string kept for diagnosis; a retry clears the previous error from the
   popup body.

## Network behavior (2026-10-10)

- The updater follows the app-wide outbound proxy chain — config.json
  `proxy.url` first, then the Windows system proxy (Clash-style tools flip
  it in the registry), else direct — the same `providers::outbound_proxy_url`
  every provider client uses. Before this, the plugin's reqwest client
  (built without the `system-proxy` feature) saw only environment vars, so
  system-proxy users were updating over a direct github.com connection and
  flaky direct links surfaced as "error sending request for url" at the
  install-time re-check.
- The manifest check is capped at 20 s, matching the provider clients. The
  installer download intentionally has no total timeout — the plugin's
  download client is unbounded, so a slow download progresses instead of
  dying at an arbitrary cap.
- The plugin tries multiple endpoints in order; today there is exactly one.
  A CN-reachable mirror manifest (e.g. the site's Cloudflare Worker
  proxying `latest.json`) is the evaluated next step (T-0030). Note the
  installer URL inside the manifest would still point at GitHub, so a
  mirror alone only fixes the check leg unless the worker also proxies
  assets.

## Known limits

- macOS has no update feed (dmg is a first published binary; users update
  by reinstalling). Linux ships AppImage/deb the same way; only the
  windows-x86_64 and linux-x86_64 manifest entries exist.
- Proxy changes need an app restart — the chain is read once per run, like
  the provider clients.
