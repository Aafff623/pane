# Temp Rules (Pane)

- Put temporary material in the matching `temp/` subdirectory — see
  [`README.md`](README.md) for the layout. Create a subdirectory on first
  use; don't pre-create empty ones.
- Keep raw user input unchanged under `input/` when provenance matters.
- Research goes to `research/`, finished reports to `reports/`, and
  cross-session handoffs to `handoff/` as
  `YYYYMMDD-HHMMSS-T-<task-id>-<topic>.md` (task ID from the root
  `TODO.md` board). No index file: the newest handoff is the latest
  filename timestamp; find a task's parcel by searching its `T-XXXX`.
- Experimental scripts and temporary HTML/CSS/JS go to `scripts/` or
  `preview/` — never into the repo tree.
- Local credentials and configuration backups only in `secrets/`; never
  echo their values into replies, logs, code, or tracked files.
- Do not treat a report or handoff as durable project documentation until
  the user explicitly promotes it into `docs/`.
- Do not delete or clean temp material automatically. Ask before any
  destructive cleanup.
- Everything here is Git-ignored except `README.md` and this file.

## Local development startup and delivery gate

- The canonical local debug launcher is `scripts/restart-pane-dev.ps1`.
  Run it from the repository root after `pnpm build`; it starts Vite on
  `127.0.0.1:1420` and the debug `pane.exe` API on `127.0.0.1:6736`.
- The installed release executable is a separate track. Do not use it to
  validate uncommitted local changes, and do not serve `dist/` with a Python
  HTTP server.
- Keep the local debug services available for the user's personal UI
  acceptance. Build/test output and port checks do not count as acceptance.
- Only after the user explicitly confirms acceptance and authorizes delivery
  may an agent create a commit, push, tag, or release. See the repository root
  `AGENTS.md` and `docs/dev-startup.md` for the full sequence.
