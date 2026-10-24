# OrcaTerm quota card

Status: implemented (2026-10-24) - automated checks green; owner visual acceptance pending

## Goal

Show Tencent Cloud OrcaTerm's locally signed-in account and the package
quota returned by its own billing API. This is a read-only usage card; it is
not a chat client, an SSH quota viewer or a claim automation surface.

## Source and privacy

- Read `%APPDATA%\com.orcaterm-desktop.app\data.bin` only. Never write it.
- Read `oauth_access_token`, `oauth_expires_at` and the optional display name
  from `auth_identity`.
- Call only `api.orcaterm.com` with the `orcaterm` product header.
- Never log, persist or return the access token. Refreshed tokens stay in
  memory.

## Behavior

- Show the returned `roleType` without guessing a marketed plan.
- Show `hunyuan_ai` requests with its monthly reset, `space` in GB,
  `secret` count and `session_share`; a limit of `-1` means Unlimited.
- Missing or malformed usage is an error, not zero usage.
- Refresh only when the token expires within 60 seconds. The in-memory cache
  is bound to the source disk token so switching accounts invalidates it.
- HTTP 401/403 and rejected refresh are `no_credentials`; transport and parse
  failures are `error`.
- Use one fetch mutex so concurrent expired-token reads cannot race refresh.

## Acceptance

- Automated: catalog mirrors, dispatch, parsing, token-cache invalidation,
  reset window, missing usage, i18n and site provider count.
- Owner: inspect the live card and compare it with the OrcaTerm client.
- Pending owner experiment: determine whether refreshing a still-valid token
  invalidates the client's own token. Do not claim this was verified.
