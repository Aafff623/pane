# 11 - Secrets and stable identities

Type: task
Status: resolved
Blocked by: 10
Coverage: W3, W4, W5, W6

## What to build

Concentrate error contracts and safe secret migration/removal. Prove existing card ids remain byte-stable. Resolve managed plaintext sessions using synthetic fixtures; do not read real user secrets.

Authority: ../spec.md. Retain existing refresh-ux/trend-units product requirements.

## Acceptance

- [x] Trace all production, persisted and external consumers at this seam.
- [x] Record a behavioral red-before-green check, or contract proof for a justified retained abstraction.
- [x] Implement the complete vertical change; remove replaced duplicate policy.
- [x] Run targeted checks and affected integration/build checks.
- [x] Standards review, Spec review and simplification scan; fix findings and re-test.
- [x] Record exact evidence and any runtime/owner acceptance still open.

## Evidence

- `credentialFailure` now treats numeric status codes as whole tokens, so
  request ids such as `142903` do not become rate-limit evidence. A synthetic
  contract test covers unknown responses, network 5xx, expired sessions and
  real 401/403/429 classifications.
- `secretstore` failure injection proves a failed vault write leaves the
  plaintext fallback intact; healthy migration removes only the secret field
  and deletes key-only files. No real vault or runtime credential was read.
- `identity.rs` owns the two-lane FNV card-id derivation. API-key, OAuth login,
  Antigravity and Cursor stores now call it; existing stable-id tests remain
  byte-compatible.
- Targeted checks: frontend credential tests 6 passed; Rust secretstore tests
  3 passed. Full post-edit suite remains a stage-16 gate.

## Review

- Standards: vault `Missing` and `Unavailable` remain distinct; migration is
  write-first and uses the existing atomic account writer.
- Spec: labels and base-url identity rules remain unchanged; only duplicate
  derivation policy moved behind the shared module.
- Simplification: four hash implementations became one; no credential data or
  persisted card-id format changed.

## Comments


## Restart continuation evidence (2026-10-18)

Resume audit: the earlier resolved marker was premature; it was reopened.
Managed Doubao, StepFun Plan and WorkBuddy session documents now use
secretstore::{load_session,save_session,clear_session}. Successful writes migrate
legacy JSON into the existing OS vault namespace; failed writes preserve a file
fallback, and a fallback rotation removes the old vault value so it cannot
shadow the new token later. Clear checks vault deletion before deleting the
legacy file. StepFun interactive save now reports persistence errors. Tests use
only the harness vault and synthetic temporary files. Session migration,
rotation, fallback, cleanup and the four legacy identity golden vectors are in
the real-source suite. Real Windows vault behavior still needs owner acceptance.

## Review fix (2026-10-18)

Fixed the P1 session rotation case where a failed vault write plus an old
readable vault value could leave the new plaintext fallback shadowed by the old
vault session. `save_session` now detects an existing vault value before the
fallback; if it cannot remove that old value, it removes the fallback and
returns the error instead of publishing two conflicting authorities.
