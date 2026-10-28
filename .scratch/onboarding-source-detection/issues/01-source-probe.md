# 01 · Read-only source probe
Status: done — implemented 2026-10-28; owner acceptance pending | Priority: P1 | Depends: none

Build: backend `source_probe` command; PATH/PATHEXT and known-directory install evidence; process + HKCU/HKLM env snapshot; credential presence/source; common unsupported env keys; unit tests for path/env parsing. No command execution or network.
Done when: probe returns installed/missing/unknown and evidence for the main harness families; env names never values; rescan reads registry after `setx`.
