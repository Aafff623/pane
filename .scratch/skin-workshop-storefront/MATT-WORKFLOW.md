# Matt Pocock inspired workflow · execution record

1. **Agree / clarify** — completed in this conversation: original user screenshots, App Store reference, user confirmed two rendered concepts. No new questions needed.
2. **Agree with Docs / .scratch** — `research.md` records decisions, known implementation constraints, and verified sources.
3. **to-spec** — `spec.md` defines problem, scope, business boundaries, contracts and acceptance.
4. **to-tickets** — four dependency-ordered local ticket files under `issues/`.
5. **GitHub** — one consolidated issue [#107](https://github.com/Aafff623/pane/issues/107) created. Not every ticket needs its own remote issue.
6. **implement** — the standalone MVP supplied the information hierarchy; the actual Pane checkout now carries the storefront (Settings page, gallery, detail, real-component previews, state handling). Implementation followed build → tests → real-window checks per ticket; no pass was claimed before running the command.
7. **review / delivery** — two independent review passes replayed earlier findings and found/fixed state, silent-failure, focus and accessibility defects; build, 237 Node tests and 50 site tests pass. Owner accepted the real-window result on 2026-10-27. Remaining closeout is commit/push authorization, not feature work.
