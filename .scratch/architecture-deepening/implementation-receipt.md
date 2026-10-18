# Implementation receipt

Baseline: 97f027d plus the pre-existing keyvault.rs fix.
Branch: codex/architecture-deepening. No commits, pushes, tags or deployment.

## Completed implementation stages01-06

- Verification foundation: catalog fields and CI runner.
- Refresh request/acceptance state, including a reproduced stale-result race and queued interactive permission.
- Render scheduling: hidden suspension, frame coalescing and explicit visible synchronous flush.
- Platform: Registry/ACL/replace/RNG/path seam; actual private-file tests.
- Snapshot: locally testable combination/identity/cooldown behavior; repeated failed-prune ordering regression.
- Provider registration: native/global/manual single source, defaults/tray derived from catalog, key/account dispatch consolidated. Universal vendor-transport adapter rejected on concrete protocol variation; shared HTTP primitives already exist.

## Verification

- node --test tests/*.test.mjs:40 passed.
- Node tsc and Vite production build passed; existing chunk-size warning.
- GNU parse-tests full suite:842 passed,13 ignored;34.60s (before the3 final provider_dispatch tests).
- GNU cargo check --offline --tests: passed with existing warnings. This is compilation, not execution of the MSVC/Tauri tests.
- Platform directed tests:7; relay storage29; ids2; proxy1;home2 all passed.
- Snapshot policy tests:17 passed (11 existing tests moved plus6 new).
- Red regressions were observed before fixes: dispatch name mismatch; missing stepfun-plan manual adapter; creditTrend100 vs200; queued clearBenches false; hidden render writes1 vs0; repeated failed prune advancing stamp.

## Review

Standards and Spec review performed directly against the baseline and stage spec. Parallel agents unavailable due tool routing validation. Targeted findings fixed: Node strip-only incompatibility in parameter properties; account VM fixture needed real RefreshState; repeated disk-failure stamp progression; obsolete provider/account comments.
Simplification removed duplicate frontend flags, render entrypoints, native matches and default/tray lists. New modules expose behavior; no new dependencies or schema changes in stages01-06.

## Runtime and recovery

- No user credentials read; no runtime files, permissions, services or WebView cache changed. Permission tests use temporary fixtures only.
- UI acceptance, GNU debug build and target desktop runtime not yet completed; stage16 owns these gates.
- Broken user pnpm shim targets a missing package. Bundled fallback pnpm11 tries automatic installation and aborted before module purge; do not repeat it. Verified direct Node equivalents use the existing dependencies. Global installation/configuration unchanged.
- Rollback is source-only: reverse the engagement diff selectively while retaining the initial keyvault overlay. No installed-state restoration is required.

## Remaining scope

Stages07-16 remain active requirements: structured units/meter history, vault operations, recovery normalization, protocol clusters, secrets/stable identities, state/atomic I/O, spend source ownership, pricing, frontend/i18n contracts, integration/review and owner acceptance. This receipt does not claim the full goal is complete.

## Stage07-09 addendum

- Units: typed UsageUnit/UsageReading/UsageDays; per-unit/scope/shape meter ledger; legacy migration; unknown-version refusal; GIPHY daily flow; Tavily key-scoped readings; frontend typed trend + unit tooltips.
- Vault: one generation-owned v3 entry update (label/note) with fresh nonce + manifest re-MAC; unlocked note display; duplicate v3 row maps removed; canonical reveal fix retained.
- Recovery: one shared normalization contract executed by both languages; TXT extraction moved into crypto; obsolete TS copies deleted.
- Rejected abstractions (with contract proof): a universal open/read/write Vault trait, and a universal key-lane transport adapter.
- Current totals: parse-tests 856 passed / 13 ignored; frontend 44 passed; tsc + Vite build pass; GNU check --tests passes with only pre-existing warnings.
- keyvault.rs was accidentally rustfmt'ed once; it was restored from HEAD and the semantic edits re-applied, so the diff is now edits only (141/63 including the pre-existing reveal fix).
- Still open: stages 10-16 and owner UI acceptance.

## Restart continuation (2026-10-18; supersedes earlier completion claims)

- Earlier stage11-14 resolved markers were reopened because their evidence did
  not cover the complete acceptance list; implementations and tests now cover
  the missing managed sessions, cache writes, spend composition and priority
  locking/cache dependencies. The old receipt sections are historical.
- Stage11: shared session-document vault migration with bounded legacy reads,
  safe fallback rotation and checked deletion; four stable-id golden vectors.
- Stage12: actual snapshot/identity writes now atomic, unique staging names,
  failed-write cleanup; permanent history migrations/future-version refusal
  retained; no invented envelopes for config or reconstructable cache maps.
- Stage13: production pi composition fixture and mcode backfill reuse.
- Stage14: no catalog mutex re-entry; spend price probes preserved; baked price
  corrections revision14; realistic held-lock priority-resolution regression.
- Stage15: peak-hour and actual reset-wake badge behavior; Rust metric-label to
  frontend mapping gate. Removed the shallow resetWakeState test-only module.
- Stage16: real loopback HTTP host/method/CORS/JSON handling; telemetry 500->200
  daily retry, same anonymous identity and opt-out state deletion with fixture
  paths only. No real telemetry was sent and no runtime credentials were read.
- Verification: parse-tests873 passed/13 ignored, Node49 passed, TypeScript/Vite
  passed, GNU check --offline --tests passed, git diff --check passed.
- One full-suite run failed the existing scheduled display-unit backfill's
  3-second persistence deadline (867 passed/1 failed/13 ignored). Its targeted
  rerun passed, followed by full suites870,872 and873 passed. No assertion or
  deadline was weakened; the intermittent cause remains unconfirmed.
- Standards/Spec review was performed directly after subagent routing rejected
  the configured model's inherited reasoning effort. This is not independent
  parallel review. Fixed findings: nested pricing lock, lost price probes,
  harness-only failure injection in production check, direct cache overwrites,
  shallow testing extraction and overstated completion records.
- No commits, push, tag, release, service termination, WebView cleanup or app
  launch occurred. Owner desktop/live-vendor acceptance remains open.

Final build: cargo GNU debug build --offline passed in1m24s. The linker emitted
` .rsrc merge failure: multiple non-default manifests`; target desktop execution
remains unverified. Stages01-15 are locally resolved; stage16 is ready-for-human.
See owner-acceptance.md for the remaining desktop/live-vendor checks.

Report delivery: report-location.txt points to a fresh OS-temp HTML containing
16 before/after cards and links to the previous report. Static HTML parsing
confirmed16 cards/20 links; tone_check passed for this receipt. The app open
request returned queued, and browser inspection rejected the file: protocol.
No rendered-browser verification is claimed and no alternate transport was used.

## Review fixes (2026-10-18)

- Session fallback authority is now single-valued: an old readable vault entry
  must be removed before a fallback file is published; failed removal cleans up
  the new file and returns an error.
- Daily meter samples now use the same 35-day retention window as cumulative
  samples, preventing unbounded daily-history growth.
- Verification after fixes: parse-tests874 passed/13 ignored, Node49 passed,
  TypeScript/Vite passed, and the GNU `check --offline --tests` run is in
  progress/completed separately in the local log. No commit or push.
