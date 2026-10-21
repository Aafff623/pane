# Spec — 账号生命周期：删除 / 归档 / 置顶

**Status:** ready-for-agent
**Feature dir:** `.scratch/account-lifecycle/`
**Date:** 2026-10-21
**Source:** owner 2026-10-21 MiniMax 多账号 bug 报告 + 本地数据只读核对（凭据、card id 已脱敏）

## Problem Statement

MiniMax 的一个账号密钥被重置后，owner 想先归档、再删除它。实际结果是：

- 账号删不干净：归档/删除之后它仍以旧账号身份出现在卡片上，占着「置顶」位置；
- 归档与删除互相打架：同一个 `card id` 可以同时存在于 active 和 archived 两个 store；
- 重新添加一个新账号后，卡片上出现两个账号胶囊，明明 active 里只应该有一个可用账号；
- 删除当前置顶账号时，置顶没有交给下一个账号，旧账号还会从归档/快照/布局里回来；
- 删除后卡片上的实时用量没有立即消失，仍然显示旧账号的错误（如 `cookie is missing, log in again (code 1004)`）；
- 添加账号时保存按钮被测试结果卡住；owner 期望先保存、先显示，再由后台监测，而不是测试不通过就不能保存。

本地只读核对确认了数据层的现状：active store 里有两行；其中旧的 `card id` 同时也在 `archived_accounts.json` 里；布局顺序包含 bare family 和新账号，旧 active 仍占 index 0（默认/置顶）；快照缓存只属于新账号，旧账号的卡片面继续显示旧凭据错误。

核心不变量（本次要建立并保持）：

1. **active 与 archived 不相交**：同一个 `card id` 不能同时在两个 store。
2. **删除是身份删除**：被删除的 `card id` 不得出现在 active、archived、快照、布局、置顶/默认引用中。
3. **默认/置顶是位置语义**：active 列表第一个账号是默认账号（index 0）；删除它后，下一个 active 账号自动成为默认；删除非默认账号不改变默认。
4. **归档是墓碑**：归档账号停止轮询、从 active 卡片和胶囊消失，但保留凭据与历史，可恢复；归档账号不参与默认/置顶。
5. **保存先于监测**：账号添加保存后立即成为 active 账号并显示，首轮探测失败只影响错误状态，不影响账号存在。

## Solution

把账号生命周期改成按稳定 `card id` 驱动，而不是按 UI 行号：

- 新增一个统一的账号删除操作：按 `card id` 删除 active、archived（含内部/外部归档）以及所有派生引用；删除默认账号时由下一个 active 账号自动接班。
- 归档与恢复做成一致操作：任何一步失败都不能留下 active + archived 的双份记录；归档后账号立即从卡片/胶囊消失，恢复后回到原 `card id` 且不产生重复。
- 设置里的「已归档账号」增加删除入口；删除归档账号与删除 active 账号走同一条按 `card id` 的清理路径。
- 账号卡片的胶囊、默认/置顶标记、快照和布局在删除/归档后立即重建；不再等待一次成功刷新才消失。
- 添加账号改为「填完必填字段即可保存」；测试连接是可选动作。保存成功后立即显示新账号并开始监测，测试失败只显示可修复的错误状态。

## User Stories

1. As a Pane owner, I want to delete an active account and have its card/pill disappear immediately, so that I do not keep seeing a credential I already removed.
2. As a Pane owner, I want deleting an account to also remove its archived twin, so that the account cannot come back from the archive.
3. As a Pane owner, I want to delete an account from the archived list, so that a tombstone I no longer need does not stay forever.
4. As a Pane owner, I want deleting the current default/pinned account to promote the next active account automatically, so that the card always has a valid default.
5. As a Pane owner, I want deleting a non-default account to leave the current default unchanged, so that account order remains predictable.
6. As a Pane owner, I want the deleted account to never reappear after a refresh or restart, so that deletion is final.
7. As a Pane owner, I want archiving an account to remove it from the active card and pills immediately, so that archived accounts do not look active.
8. As a Pane owner, I want archive to keep the credential and usage history for later restore, so that I can recover an account I archived by mistake.
9. As a Pane owner, I want restore to bring back the same stable card id without creating a duplicate, so that usage/layout reattach correctly.
10. As a Pane owner, I want a failure during archive/restore/delete to leave either the old state or the new state, never a half-state, so that I never have to fix JSON by hand.
11. As a Pane owner, I want the current MiniMax duplicate (active + archived same card id) to be cleanable with one delete action, so that I can return to a single account.
12. As a Pane owner, I want adding an account to save even when the optional connectivity test fails, so that I can repair the credential later.
13. As a Pane owner, I want a newly saved account to appear immediately, even before its first successful usage probe, so that I can confirm it was saved.
14. As a Pane owner, I want a saved account's note/label to persist and be visible, so that I can distinguish multiple accounts.
15. As a Pane owner, I want deleting the last account to leave an explicit no-account/needs-setup card instead of a stale old card, so that I know the provider has no account.
16. As a Pane owner, I want the active account order to remain stable across archive/delete/default changes, so that index 0 is always the account shown on the bare family card.
17. As a Pane owner, I want deleting/archiving an account to show a confirmation that states exactly what happens to credentials and history, so that I do not accidentally lose or keep data.
18. As a Pane owner, I want the operation to be disabled/busy while in flight, so that double-clicks cannot create duplicate requests.

## Implementation Decisions

- **Identity over index.** All lifecycle operations (delete, archive, restore, set-default) resolve the target by stable `card id` (`family@fingerprint`). Row index is display/order only and may be stale. Existing index-based commands can remain for compatibility, but the UI must use the identity-based path.
- **One lifecycle module.** Account store operations stay in the account store module; the command layer composes store cleanup with snapshot/layout/history cleanup. The store exposes pure helpers that can be tested from the parse-tests harness without Tauri.
- **Delete.** New command `account_delete(provider, card_id, keep_usage)`:
  - remove the matching active entry (if present);
  - remove every archived record with the same `card id` (internal and external archives);
  - clear the card's snapshot/cache entry;
  - remove the card id from provider order and per-provider layout; if it was the default, the next active account becomes index 0 and the bare family card uses it;
  - if no active accounts remain, keep the provider card but render the explicit no-account/needs-setup state;
  - when keep-usage is false, also remove the card's usage history; when true, keep history but never render a live card for the deleted identity;
  - return the removed card id and former index so the UI can reconcile immediately.
- **Archive.** Resolve by card id, remove from active, append the tombstone, and write both stores consistently. If the active save fails after the archive write, roll the archive back; if the archive write fails, leave active unchanged. Archive also clears the card's live snapshot/order references so the card/pill disappears immediately; history/credential are retained.
- **Restore.** Resolve the archived tombstone by card id; if the same card id is already active, reject with a clear "already exists" error. Add to active, then remove the archive record; if the archive removal fails, roll back the active addition so there is no duplicate. Restored accounts keep the original card id and history.
- **Default/pinned.** The default is the first active account (index 0). There is no independent pinned id for account-store families. Set-default moves the selected account to index 0 and preserves relative order of the rest. Deleting index 0 promotes the next account; deleting a non-default account does not change index 0. Archived accounts are never default.
- **Card/pill rendering.** Build account pills from active accounts only; archived accounts appear only in the archived section. After a successful lifecycle mutation, refresh the account list and immediately remove the affected card/pill/snapshot from the UI; do not wait for a successful provider fetch. A failed first probe shows an error/needs-setup state on the account card but does not hide it.
- **Add flow.** The save action is enabled when required fields are present (key, label; base URL where the provider requires it). The connectivity test is optional. Saving stores the account and displays it immediately; monitoring starts from the normal refresh loop. Failed probes mark the account as needing repair, not reject/delete it. Duplicate keys still produce a clear error.
- **Usage/history semantics.** The live card and snapshot are always removed on delete. Historical usage/spend may be kept only when the user explicitly chooses the keep-history option (default checked, matching the current dialog intent); kept history must not render a live account card. Unchecking removes it. Archive always keeps history for restore. The confirmation copy must say this explicitly.
- **Data repair.** Existing duplicate active+archived records are not silently auto-deleted. The new delete-by-card-id path must clean both when the owner deletes the old account; the archived list delete must also be available. A load-time invariant check may surface a warning, but must not choose a side automatically.
- **Concurrency.** Lifecycle mutations are serialized by the command layer where possible; double-clicks are blocked in the UI. The store writes use the existing atomic-write helper. Failure injection seams (`FAIL_WRITE_PATH`, secret-store failure) are used in tests.

## Testing Decisions

- **Good tests** exercise public behavior through the store/command seams, use stable card ids and temp data roots, and assert the user-visible invariants (active/archived disjoint, default promotion, no ghost card). They must not depend on private field layout or real credentials.
- **Store seam (parse-tests, fast loop):** test delete-by-card-id, archive/restore, default order, and duplicate cleanup on temp roots. These are pure Rust tests compiled by the parse-tests harness; they are the tight feedback loop for the data-integrity bugs.
- **Command seam (src-tauri tests, CI):** test that delete/archive also clears snapshots, provider order, per-provider layout and usage history (when requested), and that default promotion is reflected in the bare-family snapshot identity. These run in the existing `src-tauri` test job on CI (MSVC); the local GNU box cannot link that test binary.
- **UI seam:** the add-account save-first flow and the immediate disappearance of pills/cards are verified in the demo and by owner acceptance on the real MiniMax state; there is no DOM unit-test harness for the drawer today. The UI must expose the same card ids in the DOM/test hooks where practical.
- **Failure injection:** archive/restore/delete atomicity tests use the existing write-failure seams; assert that no operation leaves the same card id in both active and archived.
- **Prior art:** the account-store round-trip tests and the command-level snapshot/layout tests already in the repo; extend those patterns rather than add a new runner.

## Out of Scope

- Redesigning the account center or the whole Customize drawer.
- Changing provider fetch protocols, credential formats, or usage accounting rules.
- Login/OAuth account stores that do not use the generic account store (Codex/Cursor/Antigravity etc.) beyond what is needed for the archived-list delete path; their default/pin semantics remain unchanged.
- Auto-switching accounts, deleting provider records, or merging accounts by label.
- Automatic repair of the owner's existing duplicate without an explicit delete action.

## Further Notes

- The owner's current MiniMax state is the acceptance fixture: an old account (index 0, archived twin) plus a new account. After the fix, deleting the old account must remove the active row, the archived twin, the snapshot/order references and the pinned/default position, leaving the new account as index 0.
- “置顶” in the owner's vocabulary maps to the positional default (index 0), not a separate account flag. The UI label can stay “置顶”, but the storage semantics are order-based.
- The owner expects deletion to remove the account from display immediately; hidden historical usage is acceptable only if the confirmation says it is retained and the card is gone.
- The fix touches the same account paths as the earlier identity/auth-center work; keep the card-id vocabulary consistent with `family@fingerprint` and the existing account store.
