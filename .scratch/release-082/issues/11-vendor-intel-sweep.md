# 厂商订阅/高峰/优惠/新模型情报扫描

**What to build:** 用项目级 aigc-news MCP（或 firecrawl 兜底）系统性扫描市面厂商的订阅档位、高峰时段、优惠福利、新模型，产出可维护的情报文档，供 provider 适配参考。

**Blocked by:** owner 提供 aigc-news 凭据并配置项目级 MCP（需新会话生效）

**Status:** needs-info

- [ ] 项目级 MCP 配置落地（.agents/mcp.json，凭据不进库）
- [ ] 至少覆盖：ClinePass / OpenCode Go / Command Code GOAT + 主流大模型厂商
- [ ] 产出情报文档（订阅档位/高峰/优惠/新模型/额度口径），标注来源与抓取日期
- [ ] 与现有 provider 实现逐条比对，列出「需适配」清单
- [ ] 本会话若 MCP 不可用：用 firecrawl 兜底并把缺口如实写清

## Comments

- 2026-10-14：由 spec `.scratch/release-082/spec.md` 拆分（owner 批次需求）。
