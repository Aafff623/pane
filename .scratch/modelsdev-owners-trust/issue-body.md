Status: ready
Issue: https://github.com/Aafff623/pane/issues/68
Priority: P2
Area: Spend

# models.dev owners 信任分级：官方条目胜出仲裁

## 问题

`parse_modelsdev`（pricing.rs:325-358）按 cache 字段完整度仲裁——**完整但加价的转售条目（score 2）必胜不完整的官方条目（score 0）**；现测试只防桩条目。live 碰撞是常态（claude-opus-5 有 21 家声明、kimi-k3 31 家、glm-5.3 32 家）。ccusage 有三级信任分级（owners>platforms>resellers），#56 终版未收录——真缺口。

## 方案（详见 spec.md，证据 temp/research/20261025-final-sweep-7repos.md §1-2）

1. supplement.json 新键 `modelsdev_trust{owners,resellers}`（owners=信任3 圈 agent 相关官方 ~15-30 家；未列出=信任2；resellers=降权）。
2. 仲裁元组 (trust,score)：trust 降序→score 降序→先到先得。
3. 接入 4 点含**必填补丁**：trust 集变化→磁盘重读 modelsdev 重解析+清 memo+GENERATION bump（否则干等 24h）。
4. parse-tests 6 新+1 守成。

## 验收

6 新测全绿+现测试零改通过；本机注入 owners 后 kimi-k3/glm-5.3/deepseek 切官方价；schedules.json 仅追加新 pin 历史美元不变（#56 已落地，协调结论见 spec）；老 supplement 无 key 时逐字节一致。

## 不做

不改造 litellm（单文档唯一无碰撞）；不扩展 exact-only 到 effort peel 层；不动 override 优先级。
