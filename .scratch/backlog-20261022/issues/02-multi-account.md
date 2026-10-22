# 02 — Multi-account center residuals (#29)

Status: implemented; owner acceptance pending
Blocked by: None

## Slices

1. Identity labels: Pane-managed, official-client local login, official-client
   current login; current is a derived fact and never written back.
2. Last used: persist a non-secret timestamp per stable card id; show last
   used and last checked separately.
3. Batch remove: frontend resolves stable card ids and calls the existing
   single-delete command per item, reporting per-item results; no cross-account
   atomicity claim.
4. Cross-account pack ordering: earliest valid resource-pack expiry
   (`Snapshot.auth_pack_expiry`) first, nearest reset as fallback, stable
   label tie-break.
5. Batch relogin: intentionally manual — OAuth cannot target which account the
   browser signs into.

## Acceptance

- Two regions, same-name accounts and multiple sources remain distinguishable.
- Re-login does not create a second card.
- Last used / last checked / health are visible; unknown is not shown as normal.
- Batch remove touches only the selected accounts; vendor stores are untouched.
- Pack ordering is stable; accounts without a known pack expiry fall back to
  the nearest reset and sort at the end when both are unknown.
- No plaintext token in the UI.
- `pnpm build`, `node --test tests/*.test.mjs`, parse-tests pass.

## Comments

- 2026-10-23: cross-account ordering now reads `Snapshot.auth_pack_expiry`
  (earliest live credits-pack expiry) before falling back to the nearest
  reset; 33 codebuddy tests, `pnpm build`, `cargo check` pass. Batch relogin
  remains manual by design.
