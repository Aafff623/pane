# 01 — ZCode provider 归因

**Blocked by:** None
**Status:** done（实现/测试完成，UI 验收待 owner）(2026-10-20)

读 `~/.zcode/v2/provider_config.json` 的 `providerRules`，把 `model_usage.provider_id` 映射到真实 provider；按 baseUrl host 路由到 Pane family；映射不到的保留在 ZCode 卡但模型标签加 `providerName`。

**验收**
- [x] StepFun / 基元律动 / cline / 万象 / 商汤 / MiniMax / DeepSeek 的消耗落到对应卡
- [x] ZCode 卡 token 总数 = 未映射部分
- [x] 全局 token 总数不变
- [x] 文件缺失/损坏时全部归 ZCode，不崩溃

**实现** `src-tauri/src/spend.rs`：`ZCODE_HOST_FAMILIES` 单表驱动 host → family 映射（baseUrl 存在时只用 host，未映射 host 不因 providerId 同名而迁卡；仅无 baseUrl 的模板规则按 providerId 兜底）、`zcode_place_row` 按行拆分、DB 缓存以 db/wal/rules 三处 mtime 失效；MiniMax 合并进既有卡，其余家庭建卡。历史库 `spend_history.rs` 升到 v4：清掉扫描窗（32 天内）的旧 zcode 单元格，避免归因后的 family 行与旧 zcode 行重复计费；窗外历史保留。测试 `parse-tests` 全量通过；真机 30 天数据 ZCode residual 7.18B tokens，各 family 卡均有量。
