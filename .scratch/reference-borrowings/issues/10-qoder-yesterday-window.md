# 10 — Audit Qoder yesterday DeepSeek estimate

Status: implemented; owner display acceptance pending
Priority: P1
Blocked by: None

## Owner report

Yesterday (2026-10-22 Asia/Shanghai) DeepSeek input-token estimate looked about 1.2–1.3B near midnight; Pane now shows about 2.2B. Verify date windows, request deduplication and model attribution against local usage metadata. Do not rewrite source/vendor data or force the estimate to the remembered value. Credits are measured; tokens remain labeled estimates.

## Regression first

Construct yesterday/today/local-midnight events and compare the yesterday window against the exact previous-day row, including tokens, credits, trips, sessions and per-model totals. Run a read-only live metadata audit before and after correcting any demonstrated bug.

## Root cause and correction (2026-10-23)

`build_report` used yesterday as a lower bound but today as the upper bound, in both day/model accumulation and session membership. Consequently "yesterday" included today. Share explicit inclusive (from, to) ranges; yesterday is now exactly the previous local calendar day. Existing parser/deduplication and estimate derivation remain intact; source/vendor files were not rewritten.

- Regression `yesterday_is_one_local_calendar_day_not_yesterday_plus_today`: before **1,100,000 vs expected 300,000**; after passed, including local-midnight edges, credits, trips, sessions, model totals, today and 7-day totals.
- Qoder suite: **15 passed / 0 failed / 2 ignored**. Explicit read-only `real_machine_yesterday_audit` also passed.
- Final complete harness: **1033 passed / 0 failed / 15 ignored**, 434.94 seconds, doc-tests passed (`temp/logs/navigation-rust-all.log`). Ignored real-machine audits remain opt-in; the yesterday audit was explicitly run separately.
- Before audit: buggy yesterday total **2,257,298,490**, DeepSeek **2,156,268,402**. These equal Oct 22 + Oct 23 at the audit time.
- After audit: yesterday total **1,019,286,328**, DeepSeek **1,018,286,974**, Qwen **999,354**; trips **3,517**, nominal credits **1,994.789**. Values exactly match the Oct 22 day row. Today's ongoing usage changes independently.
- Oct 21 total was **1,218,897,633** (DeepSeek **1,154,723,353**). This is context only; it does not prove which screen/date the owner saw near midnight. No historical screenshot or midnight snapshot was available to establish the remembered 1.2–1.3B value.
- Tokens remain estimates from context ratio × model context window, not authoritative vendor token billing. Credits retain their existing measured metadata semantics.
- Audit logs: `temp/logs/qoder-yesterday-before.log`, `temp/logs/qoder-yesterday-after.log`. GNU debug build passed and Pane restarted using the standard launcher. Final displayed chart remains owner acceptance.

## Owner acceptance

After refresh/restart, select Yesterday / tokens / by model: Qoder CN DeepSeek should be about **1.018B**, all Qoder CN models about **1.019B** for Oct 22. Confirm today's work no longer increases yesterday's amount. The overall chart can additionally include other tools; compare the Qoder source breakdown, not an unrelated combined total.
