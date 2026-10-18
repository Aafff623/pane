# Structured usage sources

Read by implementation/review agents during stage07. Updated because typed readings replaced UI-text parsing. This is the evidence/disposition map, not a second provider catalog.

| Source | Observed counter | Unit / scope | Policy |
| --- | --- | --- | --- |
| qodercn credit_metrics | aggregate used + package used | credits / primary | Do not additionally sample package rows. |
| traecn usage_summary | consumed_amount | credits / primary | Pack rows remain quota displays, no second samples. |
| codebuddy CreditsFace | total - remain through used() | credits / primary | Same producer covers CodeBuddy, CN and WorkBuddy web route. |
| shandianshuo speech | usedMinutes | minutes / primary | Separate from assistant credits; no conversion. |
| shandianshuo assistant | usedCredits | credits / primary | A second unit on the same card. |
| tavily usage | key.usage | credits / stable credential fingerprint | Never sum repeated account.plan_usage from possibly shared accounts. Label changes do not alter real meter scope. |
| firecrawl | remaining_credits + plan_credits | no cumulative reading | Grants/topups make plan-minus-remaining ambiguous; no fabricated trend. |
| bocha | balance response | no cumulative reading | Currency balance is not consumed credits. |
| brave | monthly limit/remaining quota headers | no new meter yet | Need prove key vs shared-account ownership before summing. Existing quota display retained. |
| giphy | last completed day + rolling7d | no cumulative meter | Native daily buckets require flow observations, not cumulative differencing; retain real quota rows until flow ingestion is separately validated. |
| jetbrains | tariffQuota/current | credits / primary | Top-up balance is not added to consumed meter. |
| kiro | plan_used | credits / primary | Overage remains separate rather than silently double-counting. |
| clawsgo | cycleSpentMilli /1000 | credits / primary | 30-day tokens/requests are rolling windows, not cumulative meter sources. |
| codebuff | usage or used | credits / primary | Missing total still permits explicit consumed amount. |
| sub2api | usage.total.total_tokens and requests | tokens + requests / primary | Only All time is cumulative; today figures do not enter this meter. |
| apigoto | subscription.used/window_sec | not sampled here | Anchor rolling window is not proven cumulative; no implicit migration from text. |
| local CLI spend | timestamped events | tokens, cost view | Existing flow and dedup stay authoritative. |

Remaining facts to finish stage07: integrate real GIPHY dated flow and Brave ownership only when its source contract permits it; decide honest missing-data behavior from the confirmed no-fabrication rule. These are unfinished evidence/implementation, not marked completed by passing unrelated meter tests.
