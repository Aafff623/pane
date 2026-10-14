# 厂商订阅/高峰/优惠/新模型情报扫描

**What to build:** 用项目级 aigc-news MCP（或 firecrawl 兜底）系统性扫描市面厂商的订阅档位、高峰时段、优惠福利、新模型，产出可维护的情报文档，供 provider 适配参考。

**Blocked by:** 无运行时阻塞；2026-10-14 当前会话可发现并成功调用 aigc-news。部分官方动态定价页不可读，具体字段保留待确认。

**Status:** resolved

- [x] aigc-news 当前会话真实调用成功；本轮不读写 `.agents/mcp.json` 或凭据，不宣称重新验证了配置来源
- [x] 文档覆盖 ClinePass / OpenCode Go / Command Code GOAT 与主流厂商，区分已复核与原稿待确认线索
- [x] 更新 `docs/provider-intel.md` 的五类情报、官方来源与 2026-10-14 复核范围
- [x] 静态比对现有 provider 路径，保留需适配清单；纠正 GLM 倍率、MiniMax 窗口与本地模型缺价的过度结论
- [x] MCP 已可用，无需配置兜底；不可读的动态官网与剩余字段已在文档第 5 节明确列出

## Comments

- 2026-10-14：由 spec `.scratch/release-082/spec.md` 拆分（owner 批次需求）。
- 2026-10-14：通过 AIGC Radar 检索一次，再回溯官方正文。核实 Cursor 新 X Subscription 与存量双池月套餐并存；MiniMax 非视频仍 5h + weekly；Z.ai 当前个人档低谷为标准 credits 费率 0.5×，现有峰谷提示无需按原稿 3×/1× 修改。补齐 Claude Team 与 Kiro 档位，更新 Warp/Devin 口径，移除 Poe 等未证实价目作为已确认事实的表达。未改配置、未调用账户 API、未修改 provider 代码、未提交或推送。

## Answer

情报交付见 [`docs/provider-intel.md`](../../../docs/provider-intel.md)，第 5 节记录本轮官方复核范围、不可读取的页面和原稿未复核字段；第 4 节为基于真实源码的适配建议。此票 resolved 表示研究文档与缺口记录已交付，不表示所有厂商当前价目全部确认，也不表示 provider 适配或 UI 验收完成。
