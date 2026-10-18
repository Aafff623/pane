# 16 - Integration and final review

Type: task
Status: resolved
Blocked by: 15
Coverage: S2, all

## What to build

Test telemetry and HTTP failure/accept paths through real observable interfaces. Run appropriate full builds/suites, Standards and Spec review plus simplification scan, reconcile all rows, and deliver the owner acceptance checklist.

Authority: ../spec.md. Retain existing refresh-ux/trend-units product requirements.

## Acceptance

- [x] Trace all production, persisted and external consumers at this seam.
- [x] Record a behavioral red-before-green check, or contract proof for a justified retained abstraction.
- [x] Implement the complete vertical change; remove replaced duplicate policy.
- [x] Run targeted checks and affected integration/build checks.
- [x] Standards review, Spec review and simplification scan; fix findings and re-test.
- [x] Record exact evidence and any runtime/owner acceptance still open.

## Evidence

Completed 2026-10-18; see the final local evidence below.

## Comments


## Final local evidence (2026-10-18)

- Real HTTP requests exercise the production request responder over an ephemeral
  loopback listener: rebound Host403, mutating method405, usage GET200, JSON
  bodies and no permissive CORS header. No fixed app port is bound by tests.
- Telemetry record_at uses the production counting/send/persist path with a
  synthetic file and local500/200 server. Failed daily delivery retries, success
  persists the same anonymous identity, opt-out deletes the fixture state.
- Final GNU parse-tests:874 passed,13 ignored. Full GNU check --offline --tests
  passed. GNU debug build --offline passed; it reports the linker warning
  `.rsrc merge failure: multiple non-default manifests`. Successful linking does
  not prove desktop execution. No app launch occurred.
- Dev-server blocker (2026-10-18): Vite was crawling `.mimosa` local scanner
  state (65,913 files, not in the watch ignore list), which starved every HTTP
  request. `vite.config.ts` now ignores `**/.mimosa/**`; verified `/` 51ms,
  `styles.css` and `main.ts` 200. This was an environment/state issue, not
  caused by the architecture changes.
- Frontend49 passed; tsc and Vite build passed (existing chunk-size warning).
- Standards and Spec review ran directly because subagent routing was unavailable.
  Fixed nested pricing mutex acquisition, price-probe bypass, production test-cfg
  mismatch, non-atomic cache writes, shallow test extraction and premature ticket
  closure. No further actionable finding was identified within this review.
- Simplification consolidated hash/priority/pi/backfill knowledge and removed
  the resetWakeState module. Distinct vendor login protocols, historical schemas
  and resource installation rollback remain justified by their real contracts.
- Owner confirmed the vault reveal/copy flow and the overview balance display
  on 2026-10-18, and authorized the v0.8.6 release. Owner desktop/live-vendor
  acceptance is otherwise as recorded above.
