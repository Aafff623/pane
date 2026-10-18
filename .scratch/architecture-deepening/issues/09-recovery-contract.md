# 09 - Recovery normalization

Type: task
Status: resolved
Blocked by: 08
Coverage: A11

## What to build

Give Rust and TS shared executable input/output vectors. Resolve ASCII/Unicode whitespace and uppercase differences. Cover TXT extraction and keep checksum/recovery validation strict.

Authority: ../spec.md. Retain existing refresh-ux/trend-units product requirements.

## Acceptance

- [x] Trace all production, persisted and external consumers at this seam.
- [x] Record a behavioral red-before-green check, or contract proof for a justified retained abstraction.
- [x] Implement the complete vertical change; remove replaced duplicate policy.
- [x] Run targeted checks and affected integration/build checks.
- [x] Standards review, Spec review and simplification scan; fix findings and re-test.
- [x] Record exact evidence and any runtime/owner acceptance still open.

## Evidence

RED: the hand-copied TS normalizer kept U+0085 ("0\u008511" instead of "011") and used Unicode uppercase, while Rust uses char::is_whitespace plus ASCII uppercase. GREEN: src/recoveryCode.ts follows the Rust rules; tests/fixtures/recovery-normalization.json is executed by BOTH the Node test and a parse-tests gate over crypto::recovery_code_normalize, so the two languages cannot drift again. TXT extraction moved to crypto::recovery_code_extract (whole string, then per line) and is shared by Rust reset and TS file upload; checksum validation stays in recovery_code_decode. The obsolete main.ts copies were deleted rather than kept in parallel.

Verified: frontend 44 passed (2 recovery tests) and parse-tests 856 passed including recovery_consistency.

## Comments


This leaf has no dependency on the unfinished Vault operation refactor; it does not claim stage08 completion.
