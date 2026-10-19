# 01 — ZCode provider 归因

**Blocked by:** None
**Status:** pending

读 `~/.zcode/v2/provider_config.json` 的 `providerRules`，把 `model_usage.provider_id` 映射到真实 provider；按 baseUrl host 路由到 Pane family；映射不到的保留在 ZCode 卡但模型标签加 `providerName`。

**验收**
- [ ] StepFun / 基元律动 / cline / 万象 / 商汤 / MiniMax / DeepSeek 的消耗落到对应卡
- [ ] ZCode 卡 token 总数 = 未映射部分
- [ ] 全局 token 总数不变
- [ ] 文件缺失/损坏时全部归 ZCode，不崩溃
