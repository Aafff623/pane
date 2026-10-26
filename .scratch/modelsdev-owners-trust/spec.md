# Spec: models.dev owners 信任分级

**Status:** ready · **Priority:** P2 · **Area:** Spend
**Issue:** https://github.com/Aafff623/pane/issues/68
**Evidence:** temp/research/20261025-final-sweep-7repos.md §1-2（含 live 碰撞实测与 #56 协调结论）

## 问题

pricing.rs:325-358 `parse_modelsdev` 按 score（cache_read/cache_write 完整性 0-2）仲裁、同 id 先到先得仅高分替换——**完整但加价的转售条目（score 2）必胜不完整的官方条目（score 0）**；现测试 :1157-1171 只防桩条目。live 碰撞是常态：claude-opus-5 有 21 家 provider 声明、kimi-k3 31 家、glm-5.3 32 家。models.dev provider 条目无 owner 字段（实测 232 家），owners 集必须 Pane 侧维护。#56（已落地 f76bbfe）终版零提及 trust——从未立项的真缺口。

## 范围（1 票）

1. supplement.json 新键 `modelsdev_trust{owners[],resellers[]}`：owners=信任3（圈 agent 相关官方一手 ~15-30 家：anthropic/openai/google/moonshotai/zhipuai/deepseek/alibaba/minimax/xai/meta/zai/cohere/stepfun/volcengine/ai21 等，实测存在于 live）；未列出=信任2；resellers=信任1 显式降权（可空投）。provider key 匹配 `to_ascii_lowercase` fold；条目数设上限（仿 MAX_ALIAS_RULES）。
2. 仲裁元组 score→(trust,score)：trust 降序→score 降序→先到先得。
3. 接入 4 点：parse_modelsdev 加参；Store 增 trust 字段；apply_supplement 解析新键；**必填补丁：trust 集变化→从磁盘重读 modelsdev.json 重跑解析→清 memo→bump GENERATION+LAST_INGEST_MS**（否则干等最多 24h 后的 modelsdev 刷新）。
4. parse-tests 6 新+1 守成。

## 验收

- 6 新测：官方不完整（score 0）胜转售完整（score 2）；官方间比完整度；未知 owner 维持现行为；resellers 降权链 owner>unlisted>reseller；kimi-k2.6-nitro 不 fuzzy 命中基价；trust 热更即生效（GENERATION bump）。现测试零改断言通过。
- 本机：注入 owners 后 kimi-k3/glm-5.3/deepseek-v4-pro 解析价切官方条目。
- 时间线最小版：trust 切换后 schedules.json 仅追加新 pin，旧 pin 复放字节不变（不追溯）。
- 移除 owners key 的老 supplement → 解析结果与改前逐字节一致。

## 协调（#56 已落地）

仲裁只影响未来新观察（首见实时解析 spend.rs:510-517 + ingest 后重观察 spend.rs:559-564 都走实时链）；已钉 pin 只追加不可改→历史美元不变，正是「调价不追溯」。**不需 bump CORRECTIONS_REV**（老 supplement 无 key→owners 空→全 trust 2→现行为）。优先级 override>memo>resolve（:751-768）不变；lookup_at 故意不查 override。exact-only 无需新代码（modelsdev 恒 exact :892）；**勿扩展 exactOnly 到 effort peel 层**（Pane 有意 peel——只改 token 燃烧量不改单价）。rollout：app 先发（兼容缺 key）→supplement 后加即生效，无迁移。
