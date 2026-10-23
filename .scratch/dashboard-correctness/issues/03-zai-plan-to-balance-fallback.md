# 03 — 智谱「没有该套餐」回落余额，不再亮红点

**What to build:** 没有 Coding Plan 的智谱账号显示余额（或明确的「无此套餐」黄色态），而不是
`unexpected quota response shape` 错误 → 概览与托盘红点。
**Blocked by:** 无（但真实账号验收受 owner 材料限制，见末节）
**Status:** needs-info（需要先找到余额端点）
**GitHub:** [#48](https://github.com/Aafff623/pane/issues/48) §3 · **Priority:** P1 · **量:** 中

## 要点

1. **根因链已确认。** `src-tauri/src/providers/zai.rs` 全文没有余额/钱包路径。所有端点都返回
   `DomainErr::Empty`（`:211-213`）时，`fetch_with_key`（`:149-166`）的处理是：

   ```
   Err(DomainErr::Auth)  => rejected += 1
   Err(DomainErr::Empty) => {}          // 吞掉，不计数
   ...
   if rejected == ENDPOINTS.len() { return Err("API key was rejected on both ...") }
   Err("unexpected quota response shape (endpoint is undocumented)")   // ← 无套餐账号落到这里
   ```

   → `Snapshot::error` → 概览红点（`src/main.ts:5297`）。用户看到的是「出事了」，实情是「这家没这个套餐」。

2. **照本仓已有范式改，不新增状态。** 现成参照：
   - `providers/stepfun_plan.rs:146-157`：无套餐 → 回落 `wallet_snapshot()` →
     `Snapshot::needs_setup(...)`；
   - `providers/kimi.rs:236-237`、`:314-319`：套餐与 Moonshot 余额合并显示；
   - `providers/mod.rs:280`：`needs_setup` 第四态，渲染黄色而非红色。

   落点：`zai.rs` 的 `fetch_with_key` / `fetch_from`，在「有 key、非拒绝、但套餐行为空」这条分支上
   增加余额端点尝试；拿不到余额也给黄色 `needs_setup` + 明确文案，**不回落到红点**。

3. **前置研究：余额端点未知。** `zai.rs` 自己的错误串写着 endpoint is undocumented，本仓没有智谱
   余额路径。上游 v4.0.16 #8465 证明这类端点存在（它改成无套餐时显示账户余额），但 Pane 要自己找：
   用现有 key 探 `api.z.ai` / `open.bigmin.cn` 两侧账户类端点，或从厂商控制台抓。**探到之前不要动代码**，
   探不到就退到「黄色态 + 无此套餐文案」，那已经解决了误报。

4. **同类小尾巴一并处理**：`providers/volcengine.rs:59-68` 把错类型凭据（套餐额度要 AccessKey
   ID + SecretAccessKey，不是推理 API Key）当成 error 串返回；应给 `needs_setup` + 「需要 AccessKey」
   提示，参照上游 v4.0.16 #8494 的同一形态。

## 取证材料 / 复现步骤

- 需要一类账号：**只有按量付费、没有 Coding Plan 的智谱 key**。有则当场可复现（卡片直接红点）。
- 没有真实账号时：在 `parse-tests` 里用夹具走「套餐响应 200 但行集为空」这条路径，断言当前产出
  `Snapshot::error`、修复后产出 `needs_setup`。
- 凭据一律走既有 vault，夹具里不得出现真实 key。

## 验收

- [ ] 余额端点取证结论写进 Comments（找到 / 未找到；未找到时采用的兜底文案）
- [ ] 「有 key + 无套餐行」不再产生红点，改为黄色 `needs_setup` 或余额行
- [ ] 「key 被两家都拒绝」仍然是错误态（不要把真失败也变黄）—— 这条要有测试守住
- [ ] 火山凭据类型错误给 `needs_setup` 提示，不再是 error 串
- [ ] 现有智谱套餐账号显示不回归
- [ ] `parse-tests` 全量绿；`pnpm build` 通过
- [ ] owner 目检（**仅在手上确有按量付费-only 账号时**）：卡片不亮红点
- [ ] 若上一条无法执行，明确记「未做真实账号验收，仅夹具通过」，不虚报

## Comments

- 2026-10-23：立项，证据来自 #48 与 `temp/research/20261023-ccswitch-usage-gap.md` §9.1-3。
