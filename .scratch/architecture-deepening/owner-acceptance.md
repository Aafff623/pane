# Owner desktop acceptance

Readers: the owner who performs desktop acceptance.
Write trigger: the architecture-deepening implementation is ready for local review.

Agent checks do not establish desktop or live-vendor acceptance. Use the debug
checkout through `temp/scripts/restart-pane-dev.ps1`; startup, process termination
and WebView cache cleanup remain separate deliberate operations. This engagement
has not launched the app, stopped services or touched real credential stores.

- [ ] Show the popover with the configured shortcut; check the tray and settings.
- [ ] Refresh repeatedly, switch accounts during refresh, and verify newer
  readings and account identities remain attached to the correct cards.
- [ ] Hide the popover during refresh, reopen it, and verify pending data appears;
  navigation/focus should continue to work without duplicate painting.
- [ ] Check usage units and trends for credits, requests and speech minutes.
  Independent vendor credits must remain separate.
- [ ] Exercise a chosen browser login: completion, cancellation, expiry and retry.
  Synthetic tests do not prove current vendor endpoints or browser callbacks.
- [ ] Check vault reveal, label/note editing, lock/unlock and recovery import.
- [ ] If testing session migration, use a deliberate test account. Confirm the
  selected session remains usable, can rotate and can be cleared. Real Windows
  Credential Manager availability and size limits were not exercised by tests.
- [ ] Verify ordinary cards, spend detail and localization in the desired locale.

Record observations here or in the conversation. UI acceptance does not by itself
authorize a commit, push, tag or release; those project gates remain separate.
