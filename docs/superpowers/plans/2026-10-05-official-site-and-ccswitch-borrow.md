# 整合执行计划：Pane 官网 + cc-switch 借鉴（2026-10-05）

> 输入：两份交接件（`temp/handoff/20261005-131900-pane-official-site-kimi.md`、`temp/handoff/20261005-122411-ccswitch-borrow-to-pane.md`）+ 独立核实的仓库现状（main @ 5d9c5fe，工作区干净，site/ 不存在，Rust 侧 provider 定义 42 个）。

## 0. 关键冲突裁决

两份交接件都要求「在 `codex/<feature>` 分支做、不合 main」，但用户随后明确指示：**只保留 main 分支，本地远端都不要其余分支，main 保持最新**。以用户最新指令为准——**两个任务都直接在 main 上做、做完即提交推送**。交接件中的分支条款视为过期。（仓库 AGENTS.md 仍写着 codex/<feature> 分支规则，与用户新指示不符，本次按用户指示执行；是否改 AGENTS.md 由用户定。）

防踩踏依然成立：官网任务**只新增 `site/` + `.github/workflows/site-deploy.yml`**；cc-switch 任务动 `src/`、`src-tauri/`。两者不相交，但同走 main，每个任务内部小步提交、随时可回退单 commit。

## 1. 工作流一（WS1）：Pane 官网 Phase 1 —— 先做完

选它先做的理由：零产品代码风险、零外部依赖（Cloudflare 账号是 Phase 2 的事）、规格已定稿，可以一口气交付闭环。

1. **核实产品事实**（写文案前）：provider 清单以 `src-tauri/src/provider_catalog.rs` 为准（42 个定义，含前端独有的 mcode）；本地 API 形状以 `src-tauri/src/httpapi.rs` 实测为准；下载直链 `https://github.com/Aafff623/pane/releases/latest`；卖点以 `CONTEXT.md` 为准（本地优先、密钥不出本机、6736 回环 API、签名自动更新、主密码密钥库）。
2. **`site/public/index.html`**：Magpie 式单文件零框架（内联 style+script、无依赖、无构建）。区块顺序：Hero（一句话定位 + GitHub/下载双按钮）→ 支持对象墙 → 工作原理三步（添加凭据 → 本机查询 → 总览面板）→ 截图占位区（灰底 + 固定 width/height + lazy，文件名约定写死，用户后续换图零代码）→ 本地 API curl 段 → 尾部 CTA。视觉套 DEEP-DIVE 的 Token 表与排版规则（深色默认、九档字重、负字距、tabular-nums、1080px 容器），克制无特效。
3. **i18n 双轨（zh 源 + en 辅）**：静态文案 `data-i18n`，Worker 端 HTMLRewriter 重写出 `/en/`；动态文案 `window.T = (zh, en) => ...`。
4. **`site/worker.js`**：语言协商（Cookie/Accept-Language + Vary + 删 ETag）+ 静态资产经 `env.ASSETS.fetch` 转发（`run_worker_first` 的坑）；`/api/latest` 只留 TODO 占位。
5. **`site/wrangler.jsonc`**：`name: "pane"`、compatibility_date、assets 配置；无 routes。
6. **测试锁 `site/site.test.mjs`**（node --test 零依赖）：i18n parity 双向、T() 参数个数断言、本地资产存在性、下载链接格式。
7. **CI `site-deploy.yml`**：test 全跑 + deploy 仅 main 且 `vars.SITE_DEPLOY == 'true'`（Phase 1 合并也绝不触发部署）。
8. **验收证据**：`node --test site/*.test.mjs` 全绿输出；`wrangler dev` 本地预览截图（dark/light/窄屏三张）；`pnpm build` 无回归；给用户 5 条人工验收清单。
9. Phase 2（用户注册 Cloudflare 后）：配 secrets + SITE_DEPLOY=true → 部署验证 → 线上截图验收。给用户的三步傻瓜指引写在交付里。

## 2. 工作流二（WS2）：cc-switch 借鉴 —— WS1 交付后启动

主文档：`temp/reports/2026-10-05-ccswitch-v4.0.0-borrow-plan.md`；只读参考克隆 `temp/cc-switch`（不动它）。用户已拍板方向不再问。

按依赖关系排序的执行序（每个 milestone 独立可验收、独立提交）：

- **M1（P1-C 前置 PoC）双窗口技术验证**：Tauri 第二窗口（~960×640 可调）+ 事件总线在 Pane/WebView2 环境跑通。borrow-plan 标注此方案未做技术验证，且 P0-A 大面板的载体就是它——PoC 失败则大面板退化为「悬浮窗内全屏设置页」的降级形态。最小验证：第二窗口打开、前后端事件互通、关闭回收。
- **M2（P0-A）设置大面板 + 托盘重构**：纯 HTML/CSS 复刻 cc-switch 四原语（SettingsBlock/Card/Row/SwitchRow）+ 分区导航 + Lucide 图标（Pane 已有）；迁移现有全部设置项进分区；新增开机自启（先查 lib.rs autostart 现状）、静默启动、窗口形态切换；托盘菜单加「设置…」「开机自启」check 项、额度就地 set_text。
- **M3（P0-B）授权中心**：大面板授权中心页（antigravity/codex/copilot/cursor/xai 分组卡片，账号行版式抄 ManagedAccountsGroup），添加账号走现有 OAuth 流，与悬浮窗卡片数据同源。
- **M4（P1-D）Provider 扩容 Tier A**（逐项独立交付）：GLM coding plan remains 端点（补 zai 个人订阅口径）→ Codex wham/usage + reset-credits → Kimi 5h 窗口 → Gemini 订阅 → 火山方舟（V4 签名最重，放最后）。每项：端点细节先回 `temp/cc-switch` 源码核对 → parse-tests 驱动开发 → 验证后提交。
- **M5（P2，本期不做，写入 Roadmap）**：预设模板库、QuickJS 脚本沙箱、Tier D 大厂逆向。

每个 milestone 交付门禁：`pnpm build` + `cargo +stable-x86_64-pc-windows-gnu check` + parse-tests 全绿 + reviewer/simplifier 过一遍 + 用户 UI 验收清单。UI 最终验收永远归用户本人。

## 3. 风险与注意

- main 直提模式下，每个 commit 必须独立可回退、构建常绿——提交前必跑三件套。
- WS2 动 main.ts（1 万行）和 lib.rs（4900+ 行）时只加不改无关逻辑；托盘/窗口改动重启验证走 `temp/scripts/pane-wmi-start.ps1`。
- cc-switch 端点结论动手前必须回 `temp/cc-switch` 核对原文（独立思考契约）。
- temp/ 永不入 git；密钥/token 零入代码与 git。
