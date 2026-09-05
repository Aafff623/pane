# temp/ — Local Workspace

Everything in this directory is **local-only** and Git-ignored, except this
file and `AGENTS.md`. Use it for anything that must not touch the repo.

| Subdirectory | Purpose |
|--------------|---------|
| `handoff/`   | Cross-session / cross-agent handoff docs. Index + naming rules in [`handoff/README.md`](handoff/README.md). |
| `input/`     | Raw user-provided material (keep provenance; never edit originals). |
| `research/`  | Investigation notes, source studies (e.g. cc-switch, cockpit-tools). |
| `reports/`   | Finished reports awaiting promotion into `docs/` or disposal. |
| `scripts/`   | Throwaway scripts and one-off experiments. |
| `preview/`   | Local HTML/mockups to open in a browser. |
| `secrets/`   | Local credentials and configuration backups. **Never echo their values into replies, logs, code, or tracked files.** |

Subdirectories are created on first use — don't pre-create empty ones.

Agent rules for this directory: [`AGENTS.md`](AGENTS.md). A report or
handoff becomes durable project documentation only when the user explicitly
promotes it into `docs/`.
