# 04 — macOS build, acceptance and signing

Type: task
Status: backlog
Blocked by: platform differences inventory

## Build

- GitHub Actions `macos-latest`, arm64 dmg (already produced).
- Do not attempt macOS virtualization on the Windows dev box.

## Acceptance

- One real Mac or trusted remote session: menu-bar tray, popover window level,
  keychain read, single instance, autostart, Retina scaling.
- Check `.app.tar.gz` updater artifact separately from the dmg.
- Unsigned dmg is labelled unsigned; do not claim notarization.

## Signing decision

- Decide whether public distribution needs an Apple Developer ID.
- If yes: certificate, notarization and CI secrets are a separate ticket.
- If no: keep the unsigned label and document the Gatekeeper workaround.
