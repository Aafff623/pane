# Temp Rules (Pane)

- Put temporary material in the matching `temp/` subdirectory — see
  [`README.md`](README.md) for the layout. Create a subdirectory on first
  use; don't pre-create empty ones.
- Keep raw user input unchanged under `input/` when provenance matters.
- Research goes to `research/`, finished reports to `reports/`, and
  cross-session handoffs to `handoff/` (naming rules in
  [`handoff/README.md`](handoff/README.md)).
- Experimental scripts and temporary HTML/CSS/JS go to `scripts/` or
  `preview/` — never into the repo tree.
- Local credentials and configuration backups only in `secrets/`; never
  echo their values into replies, logs, code, or tracked files.
- Do not treat a report or handoff as durable project documentation until
  the user explicitly promotes it into `docs/`.
- Do not delete or clean temp material automatically. Ask before any
  destructive cleanup.
- Everything here is Git-ignored except `README.md` and this file.
