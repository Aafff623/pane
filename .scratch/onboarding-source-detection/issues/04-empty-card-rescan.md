# 04 · Card state, empty-board continuation and rescan
Status: done — implemented 2026-10-28; owner acceptance pending | Priority: P1 | Depends: 01–03

Build: no-credential cards distinguish installed-not-signed-in vs not-installed/unknown; credential source appears in the help/status line; empty board offers "Continue setup"; Settings offers "Rescan environment"; rescan updates probes without restart.
Done when: the three acceptance scenarios in the issue are visible on the real first-run window; rescan after `setx` changes the row without a restart.
