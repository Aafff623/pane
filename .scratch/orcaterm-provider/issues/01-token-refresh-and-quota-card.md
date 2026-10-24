# 01 - OrcaTerm token refresh and quota card

Status: implemented (2026-10-24) - automated green; owner visual acceptance and valid-token refresh test pending

## Scope

- Parse the local OrcaTerm session without modifying `data.bin`.
- Refresh an expired token through the vendor endpoint and bind the memory
  cache to the source token.
- Parse package, AI request, storage, credential and shared-session rows.
- Wire Rust catalog and dispatch, frontend catalog, icon, mechanism, onboarding
  hint and en/zh/ru labels.
- Update the public provider count and generated website demo.

## Evidence

- `cargo test --manifest-path parse-tests/Cargo.toml providers::orcaterm -- --test-threads=1`
- Full `parse-tests`, `pnpm build`, `pnpm test`, `site/site.test.mjs`

## Open owner checks

- Compare the card with the live OrcaTerm client.
- Test whether a refresh using a still-valid token revokes it.
- Confirm that switching accounts in OrcaTerm changes the Pane card on the
  next refresh.

## Comments

- 2026-10-24: local session file exists and its token is expired, which
  exercises the refresh path. No credential values were recorded.
