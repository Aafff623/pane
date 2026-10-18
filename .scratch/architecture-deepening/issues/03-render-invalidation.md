# 03 - Render invalidation

Type: task
Status: resolved
Blocked by: 02
Coverage: A3

## What to build

Inventory synchronous DOM consumers. Unify hidden suspension and frame coalescing, preserve navigation/focus, and verify deferred updates flush when visible. Link refresh-ux instead of duplicating its product scope.

Authority: ../spec.md. Retain existing refresh-ux/trend-units product requirements.

## Acceptance

- [x] Trace all production, persisted and external consumers at this seam.
- [x] Record a behavioral red-before-green check, or contract proof for a justified retained abstraction.
- [x] Implement the complete vertical change; remove replaced duplicate policy.
- [x] Run targeted checks and affected integration/build checks.
- [x] Standards review, Spec review and simplification scan; fix findings and re-test.
- [x] Record exact evidence and any runtime/owner acceptance still open.

## Evidence

RED: production direct renderer rebuilt a hidden window (1 write vs expected0). GREEN: five render scheduling tests pass; combined frontend40 passed; tsc/Vite passed. One RenderScheduler owns dirty/frame/visibility. Removed direct renderer calls and duplicate renderIfVisible/scheduleRender policy. Four synchronous flush consumers retained for expiring scroll, expand animation, popover reveal and first-tour anchors. Navigation and drawer timings undergoing further review.

## Comments


Standards/Spec direct review completed: all ordinary callers use requestRender; renderDashboard has a single scheduler-owned caller. Removed renderIfVisible and scheduleRender aliases. Explicit flushRender preserves four synchronous consumers; hidden flush still defers. Five scheduler behavior tests plus all existing tests pass (40 total); tsc/Vite pass. Owner UI focus/scroll/animation acceptance remains open in stage16.
