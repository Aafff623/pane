# 10 - Login protocol clusters

Type: task
Status: resolved
Blocked by: 09
Coverage: A9

## What to build

Map actual protocol adapters and pending lifecycles. Share real repeated machinery within compatible clusters, verify callback/session binding and late result cancellation, and retain distinct remote polling and RSA semantics.

Authority: ../spec.md. Retain existing refresh-ux/trend-units product requirements.

## Acceptance

- [x] Trace all production, persisted and external consumers at this seam.
- [x] Record a behavioral red-before-green check, or contract proof for a justified retained abstraction.
- [x] Implement the complete vertical change; remove replaced duplicate policy.
- [x] Run targeted checks and affected integration/build checks.
- [x] Standards review, Spec review and simplification scan; fix findings and re-test.
- [x] Record exact evidence and any runtime/owner acceptance still open.

## Evidence

- Added `src-tauri/src/oauth_util.rs` for the identical PKCE, URL coding,
  random token, callback page and request parsing machinery. Vendor adapters
  retain their distinct endpoint, polling, RSA and device-signature flows.
- Trae callbacks now read the expected state from the matching pending login;
  a mismatched present state is rejected and a missing vendor state remains
  tolerated only when a code/authCodeInfo is present. Cancelled and late
  callbacks cannot complete a different login.
- `cargo +stable-x86_64-pc-windows-gnu test --offline --quiet`:
  864 passed, 13 ignored. `cargo ... check --offline --tests` passed with
  existing warnings. The final post-edit full suite is rerun at stage 16.

## Review

- Standards: shared arithmetic is behind a small interface; protocol-specific
  adapters remain local because their external contracts genuinely vary.
- Spec: session binding, deadline and cancellation guards remain in each
  adapter; no universal `LoginFlow` was invented without a second compatible
  adapter contract.
- Simplification: removed seven repeated helper implementations and Trae's
  unused `login_host`; no persisted wire fields changed.

## Comments
