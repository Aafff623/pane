# 01 - Verification foundation

Type: task
Status: resolved
Blocked by: None
Coverage: W1, N1, A1

## What to build

Check seven shared catalog fields and independent dispatch metadata. Run all existing frontend tests in dependency-prepared CI. Prove a real current mismatch before fixing it.

Authority: ../spec.md. Retain existing refresh-ux/trend-units product requirements.

## Acceptance

- [x] Trace all production, persisted and external consumers at this seam.
- [x] Record a behavioral red-before-green check, or contract proof for a justified retained abstraction.
- [x] Implement the complete vertical change; remove replaced duplicate policy.
- [x] Run targeted checks and affected integration/build checks.
- [x] Standards review, Spec review and simplification scan; fix findings and re-test.
- [x] Record exact evidence and any runtime/owner acceptance still open.

## Evidence

RED: snapshot_dispatch_uses_catalog_names failed on relaybalance Custom Balance vs Custom Relay. GREEN: catalog_consistency 4 passed, including seven shared fields and duplicate-family checks. Frontend CI now uses Node24, installed dependencies and pnpm test. Local 30-test baseline passed; later combined suite 40 passed. tsc/Vite passed. GNU full crate check passed with existing warnings. Standards/Spec direct review: no remaining finding in this slice; independent agents unavailable due route validation. Simplification: replaced two TS row parsers with one structural parser; no new dependency. Remote CI not executed.

## Comments
