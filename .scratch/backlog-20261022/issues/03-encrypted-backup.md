# 03 — Encrypted backup export/import (#25)

Status: implemented; owner acceptance pending
Blocked by: None

## Shape

- `pane-backup-v1`: strict JSON inside one AEAD envelope.
- Argon2id -> HKDF -> AES-256-GCM; header only version, KDF params, timestamp,
  section counts.
- Selection is a value type: accounts / keys / settings.
- Preview after decrypt: added / conflict / unchanged counts and masked labels,
  no secrets.
- Apply aborts if the OS vault is unavailable; atomic writes; rollback
  snapshots; per-section result.

## Acceptance

- Only selected sections enter the snapshot.
- The file contains no plaintext token; leaving Pane cannot read tokens.
- Import conflicts are previewed and require explicit confirmation.
- Cancel or failure leaves existing data unchanged and reports rollback status.
- Machine migration works after reinstall.
- Tests cover roundtrip, wrong password, tamper, duplicate ids, oversized,
  unknown version, no-plaintext, conflict and injected write failure.
