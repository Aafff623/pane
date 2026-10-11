# Claude Code Project Entry

Read and follow the project rules in [`AGENTS.md`](AGENTS.md) — it is the
shared source for this project; do not create a second competing rule system
here. The canonical startup guide it points to is
[`docs/dev-startup.md`](docs/dev-startup.md); domain vocabulary lives in
[`CONTEXT.md`](CONTEXT.md) and verified engineering facts in
[`docs/project-facts.md`](docs/project-facts.md).

## Agent skills

### Issue tracker

Issues and specs live as local markdown files under `.scratch/<feature>/`
(spec.md + issues/NN-<slug>.md with `Status:` lines). See
`docs/agents/issue-tracker.md`.

### Triage labels

The five canonical roles; label string = role name. See
`docs/agents/triage-labels.md`.

### Domain docs

Single-context: `CONTEXT.md` + `docs/adr/` at the repo root. See
`docs/agents/domain.md`.
