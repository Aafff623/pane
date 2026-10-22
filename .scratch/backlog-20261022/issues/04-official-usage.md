# 04 — Official WorkBuddy usage (#31)

Status: S1 done; S0 blocked on account
Blocked by: verifiable WorkBuddy account

## S0 — probe

- One script, redacted output only: HTTP status, business code, field names,
  row counts, min/max request time, continuation advance, boundary duplicates.
- CN/AI; personal/enterprise header matrix; pageSize 1/3000; one continuation.
- Never print tokens, cookies or row values.

## S1 — pure module

- Sanitize, parse, sweep, merge, aggregate; fixtures only; no network, UI or
  ledger write.
- Adopt the sweep rules: full page -> advance `startTime` to newest timestamp;
  short page -> done; no advance -> `Stalled`; 100-round cap; dedupe on
  `(requestId, requestTs)`.
- Keep official rows out of `record_meter_samples`; never sum with
  `credit_history` snapshots.

## S2+ — gated

- Absolute-host POST, cache, command and UI only after S0 passes.

## Acceptance

- Probe exists and is safe to share.
- S1 tests cover pagination stall, page tail overlap, malformed timestamps,
  negative credit, dedupe and region isolation.
- No UI or ledger change before S0.
