# 12 - State files and atomic I/O

Type: task
Status: resolved
Blocked by: 11
Coverage: A10, W7, S1, N2

## What to build

Preserve unsupported future history instead of rewriting it empty. Prove migration/atomic-write and failure semantics for each actual state owner. Reject unjustified generic envelopes and distinguish resource install transactions.

Authority: ../spec.md. Retain existing refresh-ux/trend-units product requirements.

## Acceptance

- [x] Trace all production, persisted and external consumers at this seam.
- [x] Record a behavioral red-before-green check, or contract proof for a justified retained abstraction.
- [x] Implement the complete vertical change; remove replaced duplicate policy.
- [x] Run targeted checks and affected integration/build checks.
- [x] Standards review, Spec review and simplification scan; fix findings and re-test.
- [x] Record exact evidence and any runtime/owner acceptance still open.

## Evidence

- `keyvault.rs` rejects unsupported versions before any write; `spend_history`
  has explicit v1→v2→v3 migrations and rejects future versions without
  rewriting them empty. Existing tests cover unknown versions, migration
  retention and atomic write failure behavior.
- Actual JSON state owners use sibling temp-file plus rename (`accounts`, key
  vault, spend history, StepFun credentials); resource installation has a
  separate staged backup/restore transaction in `font_packs.rs`.
- Review found no justified project-wide state envelope: the persisted shapes
  and migrations differ by owner, so a generic wrapper would be a shallow
  adapter with no common consumer.

## Comments


## Restart continuation evidence (2026-10-18)

Resume audit: the earlier statement that all relevant JSON owners were
already atomic was too broad. persist_last_ok_at and cache_identities.json
still wrote directly; both now use accounts::write_json_atomic. That writer
uses per-process/sequence temporary names and cleans up on write/replace
failure; its failure test preserves a destination marker and verifies that no
secret staging file remains. Config keeps its existing backup and unique-temp
transaction. The permanent spend v1->v2->v3 ladder and meter future-version
refusal remain covered by existing behavioral tests. Config is an extensible
unversioned object, while snapshot/identity maps are reconstructable caches:
adding an invented version envelope would change those contracts without a
migration consumer. Resource pack staging/backup is retained as a distinct
multi-file transaction, not folded into a JSON writer.
