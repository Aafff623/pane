# 官网打磨 · 实施计划（site-polish）

> 2026-10-12 · 前置：[01-theory-analysis.md](01-theory-analysis.md) 已确认，四项拍板全批
> （服务商数 73 / 本轮只上云端邮局 / 卖点=皮肤工坊+保险箱+Agent 一句话 / 同步机制四件套随票做）。
> 分支 `codex/site-polish` @ ca92dfa，收口 = commit + 用户验收；合并/部署另批。

## 执行顺序与工作包

### WP1 · 同步机制四件套（地基，先行）
1. `site.test.mjs` 的 provenance 校验挂进 `.github/workflows/ci.yml`——每次 push/PR 必查，漂移即红。
2. `build-demo.mjs` 构建后清理 `demo/assets` 里未被新 index.html 引用的孤儿产物（消灭 0.6.0 残骸与版本分裂）。
3. 发版文档加一行：src 前端变更后重跑 `node site/scripts/build-demo.mjs`（落 `docs/release-format.md` 发版序列）。
4. site 测试新增数字断言：官网文案服务商数 == `src/providerCatalog.ts` familyId 实数（当前应=73）。

### WP2 · demo 重建 + 皮肤工坊入口
5. worktree 装依赖（vite；注意 agent shell 里 pnpm 解析坏，用 win-shell-mcp/直调 node）。
6. `site/demo/bridge.ts` 新增 action `skins` → 点击 `[data-skin-open]` 打开真·皮肤工坊。
7. 宿主页 `index.html` demo 控制条加第五钮「皮肤工坊 / Skins」（走 data-demo postMessage 体系，带无障碍属性）；`site.js` send() 放行新 action。
8. 重跑 `node site/scripts/build-demo.mjs`：重建 0.7.0+ bundle、消红测、孤儿清零。
9. 预览验证：按钮开真工坊面板、应用云端邮局即时换装、恢复原生、日/夜预览，`/en` 文案。

### WP3 · 0.7.0 内容同步
10. 服务商数字 66→73：`index.html:187`、`:201` + `i18n.js:13`（英文同步）。
11. 批注口径修正：「Coding Agent / 生产力工具 / MCP 搜索」→「Coding Agent / MCP」两分组（`index.html:104`、`:120` 同款文案 + 相关注解位置随新 demo 核对）。
12. 卖点上新（勾选口径）：
    - 皮肤工坊/云端邮局——与 WP2 联动成「可玩」段落；
    - 保险箱 v3（两步向导/恢复码/三路找回）——升级「密钥，留在本机」卡与设置批注表述；
    - Agent 只读自检 + Pane Skill——一句话+文档链接；
    - 明确不上：DeepSeek Harness 花费、更新确认弹窗。
13. `/en`（i18n.js）全量同步所有文案改动；FAQ 按 CHANGELOG 0.7.0 复核。

### WP4 · 截图重采（computer-use 真机，装机版 0.7.0 ✓）
14. 采集清单：Alt+2 弹窗（日/夜）、主看板、花费面板（产品归并新口径）、皮肤工坊面板（换肤前后对照）、保险箱两步向导、设置面板。
15. 替换 `site/public/` 资产：裁切尺寸、压缩、明暗版本对应位核对。
16. 可与 WP2/WP3 并行（采的是装机版，不依赖分支代码）。

### WP5 · 全站打磨（内容稳定后做，避免白磨）
17. worktree 起本地预览（8788 + wrangler dev 验 /en、/download）→ 桌面全页 + 窄视口 + `/en` 截图。
18. 自审清单附证据：间距节奏、字号层级、对比度、动效一致性、OG/元信息、断点表现。
19. 用户勾选 → 实施 → 同机位复验对照。
20. 约束：零依赖不破、0.7.0 无障碍体系不破、已验收 v1 版式骨架不大动。

### WP6 · 收口
21. 代码评审 pass + 冗余/简化扫描（仓库规矩），修完复测。
22. 验证门：`node --test site/*.test.mjs` 全绿（provenance + 新断言）；主项目 `pnpm build` 不受影响（本票不改 src/，唯一例外=无）。
23. commit 按主题分组（mechanism / demo-skin / content / screenshots / polish / docs），落 commit 走 win-shell-mcp git（届时向用户要授权）。
24. 用户验收清单 → 合并 main 与线上部署**另行授权**。

## 依赖与风险提示

- WP2 依赖 WP1-2（先有清理逻辑再重建，避免再次堆孤儿）；WP5 依赖 WP3/WP4 内容落定。
- demo 产物只准由 build-demo 生成；手改会被 provenance 抓。
- 不碰 `codex/theme-startup`（水墨）与 `codex/web-cookie-fallback` 在办内容；水墨合入后重跑构建自动进演示区。
- ZCode git guard 拦 add/commit：到 WP6 落 commit 时按对话授权走 win-shell-mcp。
