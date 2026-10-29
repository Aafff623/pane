---
description: Apply the user's Design Mode edits and resolve their comments
---

# Design Mode

Turn the visual edits and comments the user made in the Design Mode browser
extension into real code changes. Design Mode exposes these over its MCP
server — drive them live; do not look for task files on disk.

## Tools (Design Mode MCP server)
- `get_session_summary` — health check; confirm the extension is connected.
- `get_changes` — design-token / style / text / DOM changes plus pinned
  comments. The `items` array gives every entry a stable `id` for status
  updates. Each comment has an `id`, a `selector`, and — for region
  comments — a `region` box ({x,y,w,h} in document pixels) instead of an
  element. `tokenChanges` lists CSS custom properties the user redefined,
  each with the `scopeSelector` it's declared on. A `handoff` field means
  the user explicitly pressed "Send to Agent".
- `get_screenshot` — capture a `selector`, `elementId`, or region for
  visual context.
- `export_changes` — emit the edits as CSS / Tailwind / SCSS / JSX.
- `apply_changes` — push a CSS tweak back to the live page for preview.
- `set_change_status` — flip change `id`s to `in_progress` / `resolved`
  as you work; the user's Changes tab updates live.
- `mark_comment_resolved` — mark a comment done (pass its `id`).
- `wait_for_handoff` — Local MCP only. Bounded wait (max 20s) for the next
  user "Send to Agent" round. First call passes `pageUrl` and omits
  `sessionId` to opt in; resume only with the returned `sessionId` and
  `after` cursor. JSON `status` is `waiting` | `feedback` |
  `stopped` | `busy`. On `feedback`, `report` is an immutable
  snapshot — implement those ids only. Cloud / Self-hosted do not expose
  this tool; if it is missing or unknown, do a single `get_changes` and
  do not loop.

## Workflow
1. Call `get_session_summary`. If the extension isn't connected, ask the
   user to open the Design Mode side panel, then stop.
2. Ask whether the user wants one pass or live feedback rounds. Live rounds
   require the Local `wait_for_handoff` tool. For live rounds, select the
   exact `pageUrl` from the session summary (ask if ambiguous), call
   `wait_for_handoff` without `sessionId`, and wait for an explicit Send.
   Otherwise call `get_changes` once. Build a task list from unresolved
   token/style/text/DOM changes and comments; skip already-resolved items.
3. For each item:
   - Call `set_change_status` with `{ status: 'in_progress', ids: [id] }`
     so the user sees a WIP badge on the row you're working.
   - Map the `selector` to its source in the codebase. Reference files as
     `path:line`.
   - For a comment, read its `text`; for a region comment, call
     `get_screenshot` with the region first so you can see the area.
   - For a `tokenChanges` entry, find where that token is defined in the
     codebase (SCSS variable, theme file, Tailwind `@theme` block, or the
     CSS rule matching its `scopeSelector`) and change the definition —
     never inline the new value on components.
   - Implement the change. Prefer existing design tokens / CSS variables and
     `rem` over hardcoded `px`. A style change whose `newValue` is
     `var(--x)` means the element should reference that token, not the
     literal it resolves to. A `tokenChanges` entry means the token's own
     definition should change at the source. Match the surrounding code's
     conventions.
   - Once shipped, call `set_change_status` with `{ status: 'resolved',
     ids: [id] }` — the row gets struck through in the user's Changes tab.
     For a comment, call `mark_comment_resolved` with its `id` instead.
     Never auto-resolve edits that were not in the snapshot you are working;
     ids only.
4. Only if the user chose live rounds, call `wait_for_handoff` with the same
   `sessionId` and `after` set to the last consumed cursor (timeout
   ≤ 20s). Repeat:
   - `waiting` — poll again; do not invent work.
   - `feedback` — implement that snapshot's ids, then wait again.
   - `stopped` — the user ended the live session; summarise and stop.
   - `busy` — another session owns the page; stop.
   If the tool errors, is missing, or the client is Cloud /
   Self-hosted, skip this loop.
5. Summarise what you changed and which comments you resolved.

## Processing modes (ask the user if unspecified)
- **step** — one item at a time; confirm with the user between each.
- **batch** — group related items; one confirmation per group.
- **yolo** — apply everything autonomously (use with care).
