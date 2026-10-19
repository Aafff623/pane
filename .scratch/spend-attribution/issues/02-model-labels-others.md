# 02 — 模型标签与 Others 展开

**Blocked by:** 01
**Status:** pending

- 模型标签：目标卡用裸 `model_id`；ZCode 卡用 `providerName · model_id`。
- `Others` 悬浮列出每个被折叠 provider 的模型明细。
- 新增 `spendShowAll` 设置（默认 false），成本模式可关闭折叠。

**验收**
- [ ] `Others` 里能看到每个 provider 的模型与金额
- [ ] 开启「显示全部」后不再折叠
- [ ] token 视图行为不变
