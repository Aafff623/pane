# Update mechanism

How Pane ships and applies its own updates, who verifies what, and what
happens when the network path fails. Release *content* rules live in
[release-format.md](release-format.md); this file is about the channel.

## Channel

- Endpoints, tried in order (the plugin only advances on a failed or non-2xx
  request, so the fast source must come first):
  1. `https://pane.threetwoa.live/update/latest.json` — the site Worker
     serving the same GitHub manifest, except CN clients
     (`request.cf.country`) get the Windows URL rewritten to the Gitee
     mirror (measured 2026-10-14: GitHub asset ~40 KB/s vs Gitee ~315 KB/s
     from a Chinese network). The mirror is probed before it is offered; a
     missing Gitee copy falls back to the GitHub URL, so the mirror can cost
     speed but never availability. `?source=gitee|github` overrides for
     debugging.
  2. `https://github.com/Aafff623/pane/releases/latest/download/latest.json`
     — GitHub's own manifest, the reachable-everywhere fallback.
- Installer: the manifest's `platforms["windows-x86_64"].url` points at the
  release asset `Pane-Aafff623_<version>_x64-setup.exe` (NSIS) — on GitHub, or on the
  Gitee mirror for CN clients — with the detached minisign signature beside
  it (`.sig`). The mirror is uploaded by the `gitee-mirror` job in
  `release.yml` (needs the `GITEE_TOKEN` secret and a Gitee repo that exists
  with at least one commit) and holds byte-identical files.
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
- The CN mirror (2026-10-14) fixes the check leg and the asset leg at once:
  the edge route rewrites the installer URL, not just the manifest source —
  a manifest mirror alone would still hand out a GitHub asset URL. Design
  borrowed from cc-switch (`sync-r2.yml`: custom-domain manifest first,
  GitHub second, hard-fail when the mirror secrets are missing); the
  difference is geo-routing via `request.cf.country` instead of a single
  global mirror, so non-CN clients keep GitHub's speed. macOS/Linux entries
  stay on GitHub — the AppImage exceeds Gitee's 100 MB per-file limit.

## Known limits

- macOS has no update feed (dmg is a first published binary; users update
  by reinstalling). Linux ships AppImage/deb the same way; only the
  windows-x86_64 and linux-x86_64 manifest entries exist.
- Proxy changes need an app restart — the chain is read once per run, like
  the provider clients.
