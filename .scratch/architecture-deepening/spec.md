# Pane architecture deepening

Status: accepted
Date: 2026-10-17
Baseline: 97f027d
Branch: codex/architecture-deepening
Accepted: 2026-10-18 (owner confirmed the vault reveal/copy flow and the overview balance display; release v0.8.6 authorized)
Owner request: all, list the complete plan, investigate deeply, implement and finish.

## Problem

The second architecture review covers 27 original candidates and two additional
verification gaps. Repeated provider facts, refresh acceptance, OS calls,
Snapshot composition, meter semantics, vault/login protocols and spend ownership
need stronger locality and tests. Several original claims were overstated;
implementing their proposed abstractions without evidence would add complexity.

## Solution

Complete all rows in the coverage table below in 16 sequential, reviewable
stages. Each stage proves its current consumers and invariants, implements the
justified deepening, runs behavior checks, and records a Standards review, a
Spec review and a simplification result. Merge duplicate candidates at their
actual seam. A rejected abstraction must have a concrete consumer/contract proof;
"lower priority" is not a completion reason.

Readers: implementing and reviewing agents and the owner.
Write trigger: the owner's explicit authorization to pursue all candidates.
This spec and its issue files are the single work-state authority for this
engagement. Existing refresh-ux and trend-units specs retain their product
requirements; link them instead of creating parallel product specs.

## User Stories

1. As a user, newer refresh results cannot be replaced by older results.
2. As a user, hidden windows defer expensive rendering without losing updates.
3. As a maintainer, adding a family does not silently drift across dispatch,
   capability, icon and detection surfaces.
4. As a maintainer, Snapshot, login, vault and spend invariants are testable
   through actual production interfaces on this GNU-only workstation.
5. As a user, displayed usage carries its actual unit and never sums
   incompatible vendor credits or silently loses unknown-version history.
6. As a user, credentials and recovery paths preserve existing security,
   compatibility, deletion and fallback behavior.
7. As a maintainer, CI runs the real regression tests and every review candidate
   has an evidence-backed disposition.

## Implementation Decisions

- Use the current domain glossary in CONTEXT.md and ADRs 0001/0002/0003.
- Keep Rust parse-tests as the existing real-source test adapter; keep the
  existing Node test runner for frontend modules. No second test framework.
- Test seams already established by the reviewed plan: refresh result
  acceptance, rendering requests, provider catalog/dispatch consistency,
  platform operations, Snapshot composition, Metric readings/meter history,
  vault user operations, recovery normalization, protocol-specific login
  lifecycle, secret migration/account ids, state-file I/O, spend ownership,
  pricing resolution and externally observable HTTP/telemetry behavior.
- Preserve wire fields and persisted shapes unless an additive compatible
  representation or a tested migration is provided. Never normalize
  cross-vendor credits into one total.
- Prefer existing dependencies and real variation to new traits, ports,
  callbacks or generic state-file abstractions.
- A protocol-specific adapter is not equivalent to a universal LoginFlow.
  Preserve PKCE, session binding, deadlines and late-result suppression.
- Snapshot decisions may be pure; file writes, publish, telemetry and
  notifications remain effects executed by their real adapters.
- Keep existing keyvault.rs work intact. It belongs to the baseline overlay;
  retest it, but do not claim this engagement originally authored that fix.
- No commits, push, tags, release builds, installed-executable deployment,
  cache clearing or service termination before the separate project gates.

## Complete Plan and Coverage

| Stage | Issue | Scope and original report mapping | Required result |
| --- | --- | --- | --- |
| 01 | issues/01-verification-foundation.md | W1 catalog gate; N1 CI regression coverage; part of A1 | All shared catalog fields and third-surface dispatch checked; all frontend regressions run in a suitable CI environment. |
| 02 | issues/02-refresh-acceptance.md | A4 refresh state | Real refresh race is red before green; one acceptance policy handles independently arriving datasets and queued attempts. |
| 03 | issues/03-render-invalidation.md | A3 render seam; existing refresh-ux | Hidden suspension/frame coalescing is unavoidable; sync navigation/focus consumers retain behavior. |
| 04 | issues/04-platform-seam.md | A2 OS seam | Direct Registry/ACL/RNG/path OS knowledge returns behind the existing platform module; useful guard covers imports and extern calls. |
| 05 | issues/05-snapshot-composition.md | A5 store; A6 account swap; W10 bench/429; S3 cached_usage | Snapshot acceptance, fallback, identity and cooldown policy have a real local test seam; effect ordering stays explicit. |
| 06 | issues/06-provider-registration.md | A1 registry; W2 key-lane adapter | Common provider facts and executable registrations concentrate without erasing special account/composite behavior. |
| 07 | issues/07-usage-units.md | A7 units; existing trend-units | Structured readings replace display-text parsing; meters/trend routing satisfy existing product decisions and ADR 0003. |
| 08 | issues/08-vault-operations.md | A8 vault generations; canonical reveal errors | Operation contracts own generation/error handling while retaining all migration, integrity and recovery invariants. |
| 09 | issues/09-recovery-contract.md | A11 Rust/TS recovery normalization | Shared executable behavior vectors prevent normalization drift; TXT extraction and typed recovery stay compatible. |
| 10 | issues/10-login-protocols.md | A9 login scaffolding | Deepen proven protocol clusters; cancellation/deadline/session binding tested; preserve each vendor protocol. |
| 11 | issues/11-secret-and-identity.md | W3 credentialPolicy; W4 secretstore migration/removal; W5 card ids; W6 plaintext sessions | Correct error classification and vault-first migration/deletion; stable ids retained; plaintext exceptions resolved without reading real secrets. |
| 12 | issues/12-state-and-atomic-io.md | A10 versions; W7 atomic I/O; S1 spend migrations; N2 future credit version | Unknown versions preserved, migrations tested, actual shared state writes atomic; asset installation remains its own transaction. |
| 13 | issues/13-spend-ownership.md | A12 collect_daily; W11 ownership/aliases | Production source composition owns explicit day precedence, routing and alias rules with no double counting. |
| 14 | issues/14-pricing-policy.md | W12 pricing knowledge | Priority/model price policy has one owner and realistic resolution tests. |
| 15 | issues/15-frontend-contracts.md | W8 cross-language i18n; W9 frontend pure modules | Real missing module behavior coverage and Rust-label/TS localization contract checks; remove unsupported zero-test claims. |
| 16 | issues/16-integration-and-review.md | S2 telemetry/HTTP failure paths; all | Fault-focused integration tests, full affected builds/suites, two-axis review and simplification audit; owner UI checklist. |

A = original main candidate (01-12).
W = original Worth exploring row (01-12).
S = original Speculative row (01-03).
N = second-review new verification gap (01-02).
Every original A/W/S id occurs above; merged rows still require explicit evidence.

## Testing Decisions

For each behavior slice: one failing regression at the agreed existing seam,
minimal implementation, then the targeted passing check. Avoid bulk tests
against imagined interfaces and assertions on private implementation details.

- Frontend: Node behavioral tests; pnpm build (tsc + Vite).
- Rust: cargo +stable-x86_64-pc-windows-gnu test in parse-tests, with the
  MinGW bin directory prepended. Use offline mode when cached dependencies
  suffice. Full crate GNU check/build after relevant integration changes.
- Never weaken assertions, skip failing gates or replace a behavioral check
  with a source-text presence check. Cross-surface static catalogs may have
  structural consistency gates in addition to behavioral tests.
- CI runtime/dependency setup must match the tests: several Node tests need
  TypeScript and import .ts files, unlike the dependency-free site checks.
- Isolated adapters and source extraction are acceptable established testing
  techniques, but do not call them target WebView/desktop acceptance.
- Re-run the full relevant suite after review fixes; record exact counts and
  pre-existing warnings/failures at the time, not historical counts.
- Owner personally performs UI acceptance; runtime prerequisites and a short
  checklist must be delivered. This remains open until owner evidence exists.

## Out of Scope

- Installed release deployment, release packaging, tags, remote writes and push.
- Reading/exporting actual user credentials or mutating user runtime data.
- Replacing confirmed vendor protocols with invented endpoints or assumptions.
- Rewriting unrelated site design, provider marketing assets or governance.
- Removing supported compatibility/security behavior to make tests simpler.

## Completion Audit

Each issue must link changed production/test files, checks and results, review
findings and dispositions, and remaining runtime/owner checks. Each row in the
coverage table must have completed implementation or a documented contract proof
that the proposed abstraction should be retained/rejected, with the underlying
real defect addressed. Unresolved evidence, tests not run and owner acceptance
are unfinished work, not implicit success.

## Progress

- 2026-10-17: branch created; original keyvault overlay preserved. Implementation
  not yet verified. Stage 01 claimed.

- 2026-10-17: stages01/02 implementation and local gates resolved; stage03 implemented and under review. Frontend40 passed; full GNU check passed. Local pnpm launcher broken; runtime fallback pnpm11 attempted automatic install and correctly aborted without TTY. Do not use that fallback to purge modules; direct Node equivalents are verified.

- 2026-10-17: stages03-05 implemented and locally verified; stage06 registration implementation complete, W2 transport investigation remains. Combined parse-tests:842 passed/13 ignored; frontend40 passed; GNU check --tests passed. No commit/deployment/runtime-data mutation. Next: finish stage06 disposition, then structured unit/meter migration (stage07), vault/recovery/login, secrets/state, spend/pricing, frontend contracts and final integration.

- 2026-10-18: stages07-09 implemented and locally verified (units/meter ledger + migration; v3 vault entry update; shared recovery normalization). parse-tests 856 passed/13 ignored; frontend 44 passed; tsc/Vite/check pass. Stages10-16 remain active.

- 2026-10-18 restart continuation: reopened premature stage11-14 completion
  markers, completed session migration, state-cache atomic writes, source
  composition and priority cache/locking fixes. Stages10-15 now have local
  evidence. Final parse-tests:873 passed/13 ignored; frontend49 passed;
  TypeScript/Vite and GNU check --tests passed. HTTP request handling and
  telemetry failure/retry/opt-out now execute against loopback fixture servers.
  Owner desktop/live-vendor acceptance remains open; see owner-acceptance.md.
  Debug build and final report delivery are being completed in stage16.

- Final local delivery: GNU debug build passed (1m24s, linker resource-manifest
  warning retained); all agent-owned stage16 checks complete. Overall state is
  ready-for-human until the owner records desktop/live-vendor acceptance.
