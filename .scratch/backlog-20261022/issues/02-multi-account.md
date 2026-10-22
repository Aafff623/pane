# 02 — Multi-account center residuals (#29)

Status: ready-for-agent
Blocked by: None

## Slices

1. Identity labels: Pane-managed, official-client local login, official-client
   current login; current is a derived fact and never written back.
2. Last used: persist a non-secret timestamp per stable card id; show last
   used and last checked separately.
3. Batch remove: stable card ids resolved once, one mutation lock, per-item
   result; frontend selection mode with explicit confirm.
4. Cross-account pack ordering: nearest expiry + remaining, stable sort.
5. Batch relogin: sequential reuse of the existing login flow.

## Acceptance

- Two regions, same-name accounts and multiple sources remain distinguishable.
- Re-login does not create a second card.
- Last used / last checked / health are visible; unknown is not shown as normal.
- Batch remove touches only the selected accounts; vendor stores are untouched.
- Pack ordering is stable and unknown expiry is labelled unknown.
- No plaintext token in the UI.
- `pnpm build`, `node --test tests/*.test.mjs`, parse-tests pass.
