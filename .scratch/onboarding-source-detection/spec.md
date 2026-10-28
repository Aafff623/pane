# Spec · First-run onboarding and source detection (#109)

**Status:** implemented 2026-10-28; owner acceptance pending · **Branch:** `codex/onboarding-source-detection` · **Issue:** https://github.com/Aafff623/pane/issues/109

## Problem

The first run presents a provider picker and a second static three-page guide, and the guide counts enabled cards rather than real detections. The UI cannot distinguish "installed but not signed in" from "not installed", and environment credentials are invisible unless Pane already knows how to query that provider.

## Scope

**A. One first-run flow.** Replace the separate picker + guide at first run with one modal flow: welcome → detected sources → connect/verify → done. The detected-source step is driven by read-only local probes plus the current snapshots, shows evidence (executable/directory/env name), and carries a rescan action. The empty board keeps a "continue setup" entry until a source is usable. Manual re-run from settings stays available.

**B. Read-only source detection.** Add a local probe that reports, per family: `installState` (`installed`/`missing`/`unknown`), install evidence, credential presence/source (`env:NAME`, `keyvault`, `local`), and present env names. It reads PATH/PATHEXT, known install directories, process environment plus HKCU/HKLM environment; it never executes a CLI, reads a secret value into UI, or opens a network connection. Common keys (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `GEMINI_API_KEY`, `GOOGLE_API_KEY`, `AZURE_OPENAI_*`, …) are reported as found-but-unsupported when Pane has no quota reader. The same probe feeds `/v1/providers` and a new `/v1/doctor` check.

**Connect step:** existing setup sheets (Customize key editor / provider help / local sign-in instructions) are reopened from the checklist; Pane does not add a second credential form. Completion is driven by a real `status == "ok"` source, not by clicking Next.

## Contracts

`source_probe` returns:

```ts
type SourceProbe = {
  family: string;
  displayName: string;
  installState: "installed" | "missing" | "unknown";
  installEvidence: string[];
  credentialState: "present" | "absent";
  credentialSource: "env:<NAME>" | "keyvault" | "local" | "none";
  envVars: string[];
  localHint: string | null;
};
type SourceProbeReport = {
  families: SourceProbe[];
  envKeys: { name: string; family: string | null; supported: boolean }[];
};
```

`/v1/providers` rows gain `installState`, `installEvidence`, `credentialState`, `credentialSource`, `credentialEnvs` (additive). `/v1/doctor` gains `installed_but_not_signed_in` with remediation.

## Non-goals

- No new provider quota endpoints and no execution of `--version` or any CLI.
- No network requests in the probe.
- No new credential storage format, no changes to provider account stores.
- No automatic install of CLIs/apps.

## Acceptance

- [ ] Fresh install with Claude Code installed but signed out shows "installed, not signed in" with a setup action, not the old "sign-in not found" text alone.
- [ ] Fresh install with `ANTHROPIC_API_KEY`/`OPENAI_API_KEY` set shows the key evidence, even when quota reading is unsupported.
- [x] `setx` a new variable while Pane runs → Rescan sources updates without restart. (verified in the debug build with a temporary `AZURE_OPENAI_API_KEY`; removed afterwards)
- [x] First run is one full-screen flow; no separate picker before the guide. The detection step shows evidence and a per-source action. (real-window walkthrough of the four steps)
- [ ] Completion requires a real `status == ok` source; the empty board retains a continue-setup entry.
- [x] `/v1/doctor` reports `installed_but_not_signed_in` with remediation; `/v1/providers` additive fields are documented. (live `127.0.0.1:6736` check)
- [x] zh/en/ru i18n parity and docs pass. (`tests/i18n.test.mjs`, docs updated)

## Results

- Backend: `src-tauri/src/source_probe.rs` (read-only PATH/dir/registry probe, env names only), `source_probe` command, `get_credential_status` additions, `/v1/providers` fields, `/v1/doctor` advisory check. `provider_env_vars` moved to `provider_catalog.rs` so the `parse-tests` harness compiles the real probe.
- Frontend: four-step onboarding, detection/connect/done rendering, empty-board continuation, card installed/missing split, Settings rescan row, cross-window-safe replay (replay never writes provider config).
- Gates: `pnpm build` pass; `pnpm test` 239/239; `parse-tests` 1195 passed / 0 failed / 18 ignored (serial, proxy env stripped); `cargo check` pass.
- Pending: owner visual acceptance of the fresh-install card states and empty-board continuation; winget PR #463652 is unrelated to this ticket.

## Related

#42 (source probe/doctor), #64 (closed first-run guide), #1 (closed new-user overload).
