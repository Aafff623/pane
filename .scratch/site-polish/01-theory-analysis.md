# 官网打磨 · 理论分析（site-polish）

> 2026-10-12 · worktree `D:\code\pane-worktrees\site-polish`（分支 `codex/site-polish` @ ca92dfa）
> 状态：分析稿，待用户确认后转 spec/tickets。事实均经探查核实，附 文件:行号。

## 0. 需求还原（grilling 两轮结论）

1. 全站打磨：视觉、动效、文案、工程/性能、移动端、内容板块都过一遍（用户：「所有内容都打磨一下」）。
2. **同步的真义 = 保真**：用户原话「面板长什么样子，远端的数据快照就长什么样；皮肤也是，本地长什么样子，远端的界面原型就长什么样」。官网交互演示区是 Pane 的**远端原型**，不是装饰性示意图。
3. 皮肤入口：真机悬浮版不需要专门 icon（尊重克制）；官网演示区**需要**显性入口，因为访客不知道皮肤入口在哪——要展示云端邮局、（未来的）水墨风等。
4. 0.7.0 内容同步：最新进度、情况、UI 界面（截图）、流程，全部跟到 0.7.0；`/en` 同步。
5. 流程：先出清单勾选再动手；收口 = worktree 分支 commit + 用户验收，合并/部署另行授权。

## 1. 事实底座（探查核实）

### 1.1 演示区的本质 —— 这是最重要的发现

**演示区不是手写复刻，是真实 App 前端的 Vite 构建产物**：

- `site/scripts/build-demo.mjs:16-28` 以主项目 `index.html` 为入口直接打包 `src/main.ts`（base `/demo/`），把 `@tauri-apps/api/*` alias 到 mock 桥 `site/demo/bridge.ts`（349 行，注入示例数据），providerCatalog 也用真源。
- demo 专属处理：rAF→setTimeout shim、`document.hidden`→false、桥只放有示例数据的 provider（`site/README.md:17`）。
- 宿主页控制通道：`[data-demo]` 按钮组（overview/spend/settings/refresh，`site/public/index.html:117`）→ postMessage `{type:'pane-demo',action}`（`site/public/site.js:358`）；设置浮层是第二个 iframe `?settings=1`（site.js:338）。桥已支持 `customize` 动作（点 `#customize-btn`），皮肤工坊入口就在自定义抽屉里（`data-skin-open`）。

⇒ **「界面保真」在结构上已成立**：演示区 = 真前端 + 假数据，App 改了 UI，重打包就同步。

### 1.2 已有的同步检测（雏形）与它的失灵

- 构建时对 5 个源文件做 CRLF→LF 归一化 sha256，写入 `site/public/demo/source.json`（`build-demo.mjs:80-85`）。
- 校验：`site/site.test.mjs:67-71` 重算比对，失败提示重跑构建；CI 在 `site-deploy.yml:18`（test job，PR 也会跑）；**`ci.yml` 不含 site 测试**。
- **现状：provenance 测试当前就是红的**——demo 最后重建于 e64e651，其后 6188a6f（内置分组改两类）与 8834c91（CodeBuddy family）改了 src，5 个源哈希仅 index.html 匹配。
- **孤儿资产问题**：`emptyOutDir:false` 导致旧构建残留堆积，`site/public/demo/assets/index-2IUXSWQ7.js` 是 0.6.0 时代孤儿 bundle（内嵌 0.6.0 与旧 changelog），页面不加载但会误导、也被搜索引擎索引。

### 1.3 Worker 与存储

- `site/worker.js` 仅三件事：`/download/windows` 302 到最新安装包、`/en` 注入英文元信息、其余静态透传。
- **无 KV/D1/R2 任何远端存储**（`wrangler.jsonc:8-14`）。⇒ 保真同步不需要新增基建；访客状态云端持久化不在本需求内（用户已澄清）。

### 1.4 皮肤系统（真机）

- 配置键 `pane.skin.id`（localStorage），`SKINS=[{id:"signal"}]` 仅一款「云端邮局」，应用=`root.dataset.skin`（`src/skins.ts:8-19`、`src/main.ts:851-853`）；皮肤=skins.css 令牌 + 5 张 PNG 素材；工坊面板 `renderSkinMarket`（日/夜/并排预览、应用、恢复原生），入口在自定义抽屉（`main.ts:7872-7874`）。
- 无 JSON manifest——唯一清单是 skins.ts 内存数组。主题包导入/在线市场明确不在本期（`docs/skin-market-design.md:19`）。
- **水墨风皮肤在别的 worktree 进行中**（`codex/theme-startup`，`.scratch/ink-wash-theme/`），未合 main——本任务不碰。

### 1.5 内容过期点（0.7.0 视角）

| 位置 | 现状 | 应为 |
|---|---|---|
| demo bundle | 落后 src 2 commit（哈希全失配，测试红） | 重跑 build-demo |
| 批注 02 文案（index.html:104,120） | 「Coding Agent / 生产力工具 / MCP 搜索」三分类 | App 默认已改 **Coding Agent / MCP** 两分组（6188a6f） |
| 服务商数量（index.html:187,201、i18n.js:13） | 66 | 目录实值 **73**（含 CodeBuddy 3 family，0.7.0 后新增） |
| 0.7.0 卖点 | 正文完全没提皮肤工坊/云端邮局；「API 密钥库」仅一句带过（:111）；Agent 自检/Skill、保险箱 v3、更新确认弹窗均未提 | 见 §4 取舍清单 |
| 静态截图（pane-agent*.png 等） | 10-08/09 采集（4274aeb），早于 0.7.0 的 UI 变更（额度环 44/36px、花费归并、皮肤工坊等） | 真机重采 |
| 孤儿资产 | 0.6.0 bundle 残留 assets/ | 构建清理 |
| CHANGELOG 0.7.0 | hero 已是 v0.7.0（index.html:31、i18n.js:10） | 版本号无需改 |

装机版 Pane = **0.7.0**（`D:\OneDrive\Desktop\tools\Pane\pane.exe` VersionInfo 已核实）⇒ computer-use 真机重采可行。

## 2. 同步机制设计（核心理论题）

**目标命题**：本地长什么样，远端就长什么样。分解为三条保真通道 + 一道漂移防线：

### 三条保真通道

- **A. 界面保真**：演示区 = 真前端构建（已有，最强形式）。任何 App UI 改动，重跑一次构建即同步，不存在"手写复刻走样"问题。**永远不手改 demo 产物**。
- **B. 数据保真**：示例数据快照集中在 `site/demo/bridge.ts`。可选增强：把示例数据抽成独立 `site/demo/snapshot.json`（版本号、示例卡、额度环数值、花费行），桥读数据文件——「数据快照」字面化，改数据不动逻辑，也便于对照真机截屏核对口径。
- **C. 皮肤保真**：演示区直接用真 `skins.ts`/`skins.css` + 真素材（构建已带上 5 张皮肤 PNG）。换肤 = 真机同一套 data-skin 令牌生效，所见即真机所见。未来皮肤进 `SKINS` 数组 → 重跑构建 → 演示区自动出现——**「后续皮肤都可集成」天然满足，无需为官网单做接入**。

### 漂移防线（机制 = 让漂移可失败、可修复）

1. **provenance 测试挂进 `ci.yml`**：现在只有 site-deploy.yml 查，且 main 直推也能漏。挂进 ci.yml 后每次 push/PR 必查，漂移即红（机械可失败检查优于文档约定）。
2. **构建清孤儿**：build-demo 改为清理 demo/assets 下未被新 index.html 引用的产物（或 emptyOutDir:true + 保留非构建资产白名单），消灭 0.6.0/0.7.0 版本分裂。
3. **发版清单加一行**：src 前端变更后重跑 `node site/scripts/build-demo.mjs`（写进 docs/release-format.md 或 dev-startup 的发版序列）。
4. **数字一致性断言（可选、便宜）**：site 测试断言官网文案的服务商数量 == `src/providerCatalog.ts` familyId 实数，杜绝 66 vs 73 这类静默漂移。

**结论：不需要新基建、不需要 KV。机制 = 既有构建管线 + 测试闸门 + 清理 + 清单，四件套补强即可。**

## 3. 皮肤入口设计（演示区）

- **原则**：入口为教学而设（访客不知道皮肤入口在哪），形态对齐真机心智。
- **方案**：demo 控制条（现有 overview/spend/settings/refresh 四钮，`index.html:117`）增第五钮「**皮肤工坊**」→ postMessage 新 action `skins` → bridge 点击 `[data-skin-open]` 打开**真·皮肤工坊面板**（日/夜预览、应用云端邮局、恢复原生）。演示区内换装即时生效于整个演示窗——正好把「换肤」变成可玩的卖点。
- 无障碍：新钮走既有 data-demo 体系与 0.7.0 官网刚建的无障碍规范，不另起炉灶。
- `/en` 按钮文案 "Skins"。
- **水墨皮肤**：未合 main，本轮不预埋占位（避免出现点了没反应的死入口）；其分支合入后重跑构建自动出现。

## 4. 0.7.0 内容同步图谱

### 4.1 卖点取舍（待用户勾选）

CHANGELOG 0.7.0 的新增项 → 是否上网页正文：

| 项 | 建议 | 理由 |
|---|---|---|
| 皮肤工坊/云端邮局 | **上** | 有演示区联动（§3），是差异化卖点 |
| 保险箱 v3（两步向导/恢复码/三路找回） | **上** | 安全信任卖点，「密钥，留在本机」四卡可升级表述 |
| Agent 只读自检 + Pane Skill | 小提 | 面向玩家/极客，一句话+链接即可 |
| DeepSeek Harness 花费 | 不上 | 过于细节 |
| 更新确认弹窗 | 不上 | 属修补体验 |

### 4.2 文案与数据修正

- 服务商数 66→**73**（zh/en 两处+滚动区；需用户点头：等于官网把 CodeBuddy CN/WorkBuddy 纳入承诺面）。
- 批注 02/列表同款文案改「Coding Agent / MCP」两分组口径（对齐 6188a6f 后的真机行为）。
- 其余逐节核对（hero 副题、四卡、FAQ、「为工作间隙而生」）以 CHANGELOG 0.7.0 为准，出逐条清单勾选。

### 4.3 截图重采（computer-use 真机，装机版 0.7.0 ✓）

清单（初稿）：Alt+2 悬浮弹窗（日/夜）、主看板、花费面板（新归并口径）、皮肤工坊面板（含换肤前后对照）、保险箱两步向导、设置面板。采完替换 `site/public/` 对应资产并核对裁切尺寸。

### 4.4 demo 与流程

- 重跑 `node site/scripts/build-demo.mjs`（清孤儿、消红测）→ 批注锚点（usage-flow 标注的 01-06 流程）逐条对照新 UI 核对位置与措辞。
- worktree 需先装依赖（vite）才能构建；预览走 8788 静态 + wrangler dev 验证 /en 与 /download 路由。

## 5. 全站打磨方法

1. worktree 起本地预览 → 桌面全页 + 窄视口 + `/en` 全页截图。
2. 自审清单：间距节奏、字号层级、对比度、动效一致性、OG/元信息、窄视口断点——逐条附截图证据。
3. 用户勾选 → 实施 → 复验（同机位重截对照）。
4. 约束：零依赖不破、无障碍体系不破、已验收 v1 的版式骨架不大动（打磨≠重设计）。

## 6. 边界与风险

- **不碰** `codex/theme-startup`（水墨）与 `codex/web-cookie-fallback` 两个在办 worktree 的内容；水墨素材等其合入。
- demo 产物只准由 build-demo 生成（手改会被 provenance 测试抓，也破坏保真命题）。
- ZCode git guard：add/commit 被钩子拦，落 commit 时走 win-shell-mcp git 工具（届时向用户要授权）；push/部署一律另批。
- 截图采集需真机在交互桌面（restart-pane-dev 或直接用装机版），computer-use 由主 agent 亲自执行。
- 部署链：SITE_DEPLOY 仓库变量 + Cloudflare secrets，本轮不动；PR 的 test job 会先验证 provenance。

## 7. 待用户拍板（第三轮）

1. 服务商数字改 73（纳入 CodeBuddy 承诺面）？
2. 水墨皮肤：本轮只上云端邮局、水墨等合入自动出现——确认？
3. §4.1 卖点取舍按建议勾选？
4. 同步机制四件套（ci.yml 闸门/清孤儿/发版清单行/数字断言）——确认后随本票实施？
