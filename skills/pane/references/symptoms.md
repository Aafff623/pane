# Symptoms → cause → fix

Each entry: what the user sees, how to confirm it from evidence, and the fix.
Fixes that restart or delete things are performed by the agent **in the user's
visible shell**, after telling the user what will happen.

---

## S1. "A card shows ⚠ stale / no data"

**Confirm (API first — it now carries the diagnostics):**
1. `curl -s --noproxy '*' http://127.0.0.1:6736/v1/doctor` — the `provider_errors` /
   `provider_stale` / `providers_without_credentials` checks name the cards and
   the remediation.
2. `curl -s --noproxy '*' http://127.0.0.1:6736/v1/providers` — that card's
   `status` / `stale` / `error` / `warning`, plus the `disabled` and `removed` id lists.
3. If absent from `providers`: `disabled` contains it (turned off) or `removed`
   contains it (deleted). Off ≠ broken — say which.
4. Compare the age of `fetchedAt` with `/v1/status` `refresh.minutes` (default 5).
5. Fallback only when the API is unreachable: `%APPDATA%\Pane\last_snapshots.json`
   (a map keyed by card id; same fields).

**Causes:**
- `status:"no_credentials"` → the user never configured this provider. Guide them to
  the gear → provider row, or the Customize drawer.
- `status:"error"` with an auth/401 message → key revoked or session expired.
  For OAuth providers (codex/cursor/kimi/qoder/…) the login may have rotated.
- `status:"error"` with a network/timeout message → check proxy (see `references/update.md`);
  the user may have a system proxy the app is not using.
- Every card stale and `lastRefreshAt` far in the past → the background refresh loop died
  (the `refresh_loop_alive` doctor check says so explicitly). Restart Pane.

---

## S2. "Alt+2 / tray click will not open the popover"

**Confirm (Windows):** the minimized-window zombie state. A window iconified by
Win+D / minimize still reports `IsVisible() == true`, so the toggle takes the
hide branch and the popover never comes back.

- Diagnostic fingerprint: `IsIconic == True` **and** `GetWindowRect ≈ (-21333,-21333)`.
- This was fixed on 2026-09-18 (toggle treats iconic as hidden and un-minimizes
  before showing). On an older build, the workaround is to restore the window from
  the taskbar, or restart Pane.

**Other causes for the same symptom:**
- The window opened on a **non-interactive desktop station** — this happens when
  Pane is launched via a plain `Start-Process`/`&` instead of
  `CreateProcess(WinSta0\Default)`. It is running but invisible. Relaunch properly.
- Global hotkey conflict — Settings shows the availability/conflict state.

---

## S3. "UI shows old content after an update / change"

WebView2 caches assets aggressively.

**Fix:** close Pane, delete `%LOCALAPPDATA%\com.aafff623.pane\EBWebView`, restart.

---

## S4. "The local API / a script can't reach 6736 right after a restart"

The new instance **silently fails to bind** while the old socket sits in
TIME_WAIT — it still prints the URL.

**Confirm:** `netstat`/port check shows the port owned by a dead/old PID; the API
returns connection-refused.

**Fix:** wait for the port to free, then restart Pane. Do not assume the API is
"broken".

---

## S5. "One/New API (中转站) card says OK but shows 0 / wrong"

A wrong access token still returns **HTTP 200 with `success:false`**. Never treat
200 alone as success. Verify the site's access token + `New-Api-User` id, and that
at least one relay key is present (a token-only site can only show the subscription
endpoint — billing fallback needs a key).

---

## S6. "Update fails / the update button does nothing"

See `references/update.md`. The usual cause is a network path the app's proxy
chain does not cover (the updater follows `config.proxy.url` → Windows system
proxy; the manifest check is capped at 20 s and renders a localized network hint).

---

## S7. "Spend panel looks wrong / a day is 0"

See `references/spend.md`. Tokens are facts even when no price is known; unknown
prices are marked `unpriced` rather than erased.

---

## S8. "The whole app won't start / crashes at launch"

- Dev builds that depend on Vite will show an error page when Vite is absent —
  that is a development artifact, not a shipped bug.
- On a machine without MSVC, a MinGW-linked full build dies at process start with
  `STATUS_ENTRYPOINT_NOT_FOUND` — expected; not a user-facing issue for released builds.
