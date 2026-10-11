# Update channel

Pane's updater checks this project's own GitHub Releases `latest.json`, verifies a
minisign signature, and shows an in-app popup (once per version per run) beside
the footer update button.

## Proxy chain

Since 2026-10-10 the updater follows the app-wide proxy chain:
`config.json.proxy.url` → Windows system proxy. The manifest check is capped at
**20 s**; a transport failure renders the localized `update.networkHint`.

**Most common failure:** a system-proxy user whose updater previously went over a
direct `github.com` link (the plugin's reqwest was built without the
system-proxy feature). If the user has a proxy but Pane reports a network error,
check whether the proxy is actually applied to the update check.

## Diagnosing

1. Confirm the installed version (About / tray tooltip) vs the latest release.
2. If the check fails: is a proxy configured in Settings? Does the OS system proxy
   work for `github.com`?
3. If the check succeeds but the download stalls: a large asset over a flaky path;
   retry (resumable) rather than reinstalling.
4. Never suggest uninstalling to update — the in-app path is the supported one.

## Do not

- Do not create tags, releases, or push anything. Update diagnosis is read-only.
