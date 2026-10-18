# 08 - Vault operations

Type: task
Status: resolved
Blocked by: 07
Coverage: A8

## What to build

Audit all generation branches and public user operations. Concentrate actual duplicated operation/error policy, retain v1/v2/v3 integrity and recovery, and retest the pre-existing reveal fix.

Authority: ../spec.md. Retain existing refresh-ux/trend-units product requirements.

## Acceptance

- [x] Trace all production, persisted and external consumers at this seam.
- [x] Record a behavioral red-before-green check, or contract proof for a justified retained abstraction.
- [x] Implement the complete vertical change; remove replaced duplicate policy.
- [x] Run targeted checks and affected integration/build checks.
- [x] Standards review, Spec review and simplification scan; fix findings and re-test.
- [x] Record exact evidence and any runtime/owner acceptance still open.

## Evidence

RED: with a verified unlock, a v3 note edit still returned "v3 notes are sealed with the key" and the locked listing always rendered an empty note even when a session was live. GREEN: one generation-owned operation (update_v3_entry) opens the sealed payload, applies label/note, re-seals with a fresh nonce and re-MACs the manifest; rows_from_v3 decrypts notes only for a session that verifies against the current file and falls back to public metadata otherwise. add() now routes existing-entry updates through the same operation, so a v3 label change re-MACs exactly once; the previous duplicate v3 row-map in add/remove is gone. The pre-existing reveal/change-password canonical wrong-password fix is retained and retested.

Verified: keyvault tests 68 passed (includes reveal_gate_reports_v3_failure_canonically and unlocked_v3_note_edit_preserves_key_and_keeps_the_note_sealed); full parse-tests 856 passed. A universal open/read/write Vault trait was rejected on contract evidence: the three generations differ in integrity model, key derivation and recovery paths, and the match arms are internal to keyvault rather than leaked to callers.

## Comments
