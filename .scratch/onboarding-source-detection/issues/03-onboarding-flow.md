# 03 · One first-run flow
Status: done — implemented 2026-10-28; owner acceptance pending | Priority: P1 | Depends: 01

Build: remove the first-run picker before the guide; make onboarding four steps (welcome → detected sources → connect/verify → done); detection rows show evidence, install/credential state and a rescan; selected families are applied on finish/skip; completion is driven by a real `status == "ok"` source.
Done when: first run has one full-screen flow; no double modal; selection persists; "continue later" remains possible.
