# Pane 皮肤系统调研：市场可获取性、Comic Pixel 可读性与三主题补齐

日期：2026-10-25  
工作区：`D:\code\pane`  
分支：`codex/rtk-orcaterm-handoff`，相对 `origin/main` 领先 19 个提交、落后 0 个  
边界：本轮不修改共享工作树；所有调研、资产和原型均放在本输出目录。

## 0. 结论

1. **新装用户拿不到皮肤，最可能是 WebView CORS。** 线上索引与 zip 都是 HTTP 200，索引内容与本地三个官方包一致；但索引没有 `Access-Control-Allow-Origin`。在应用开发页的真实浏览器环境里执行 `fetch("https://pane.threetwoa.live/skins/index.json")`，得到 `TypeError: Failed to fetch`。你本机仍有已安装皮肤，所以看起来正常。
2. **截图里的黄底白字是确定的颜色 token 错误。** 暗色 Comic Pixel 的 `--cp-frame` 是白色；若干黄色/青色选中态把这个“边框色”当文字色。黄底白字最低只有约 **1.09:1**。
3. **Provider 名过花有本机配置因素。** 当前 `%APPDATA%\Pane\config.json` 的 `uiFont` 是 `MV Boli`。皮肤契约原本尊重用户字体，但 provider 名属于数据标签，应该锁定可读 UI 字体；Hero 与模块标题才可以保留装饰字。
4. **当前 repo 源 CSS 已比本机安装包新。** `skins/comic-pixel/skin.css` 为 57,407 B；线上与 `%APPDATA%\Pane\skins\comic-pixel` 安装包里的 `skin.css` 为 55,096 B。安装包缺少新设置骨架的 `.set-section__card`、`.set-row`、`.set-shell__nav` 覆盖；不重新打包，主对话改完源码也看不到。
5. **AIGC News 值得学的不是“每张卡都贴花”。** 它的页面底色很安静，卡片本体甚至透明无边框，个性来自 16:10 预览海报、极少量 wordmark/撕纸资产、明确排版与“强调色只给选中项”。Pane 当前的问题是把同一套粗框/硬阴影复制到大多数组件，导致千篇一律。
6. **原型已完成。** 三套皮肤、亮暗、看板、设置、皮肤工坊、组件矩阵和三种 GSAP 转场都已放在 `prototype/`。Comic Pixel 新增 26 个原创 16px 点阵 SVG。

## 1. 皮肤市场：根因与修法

### 1.1 实测证据

线上索引：

```text
GET https://pane.threetwoa.live/skins/index.json
200 application/json
Cache-Control: public, max-age=0, must-revalidate
没有 Access-Control-Allow-Origin
```

应用开发页来源为 `http://127.0.0.1:1420`，在该页执行：

```js
await fetch("https://pane.threetwoa.live/skins/index.json", { cache: "no-store" })
```

结果：

```text
TypeError: Failed to fetch
```

同一命令在 Node/PowerShell 中可以取到 200，因为命令行请求不受浏览器 CORS 约束。这个差异正好解释“你本机看起来能用，别人新装是空的”。

### 1.2 为什么本机看起来正常

- 本机 `%APPDATA%\Pane\skins\` 已有 `signal`、`ink`、`comic-pixel` 三个安装目录。
- 工坊会先显示已安装包，再异步尝试线上索引。
- 线上索引拉了 200，但被 WebView CORS 拦截；已安装包继续提供内容，所以不会明显报错。
- 对全新用户，已安装列表为空，索引又失败，最终只剩 `skin.indexUnavailable` 空态。

### 1.3 线上包本身一致

`node site/scripts/check-skin-packs.mjs --live` 当前结果：

```text
signal       deployed copy matches
ink          deployed copy matches
comic-pixel  deployed copy matches
OK
```

所以不是 zip 404、不是哈希不一致，也不是索引落后。核心是浏览器读取索引的那一步。

### 1.4 建议修法

首选：

- 新增 Rust 命令 `skin_pack_index`，继续使用 `send_with_direct_fallback`；由 Rust 拉取并解析索引，前端 `refreshSkinIndex()` 优先 `invoke()`。
- 保留浏览器 `fetch` 仅作为网页 demo 的 fallback。
- 保持 `localStorage` 缓存与离线兜底，但错误态应带短原因，区分“无网络”“索引 HTTP 错误”“返回格式错误”。

第二层：

- 给 Cloudflare 静态资源补 CORS。至少覆盖 `/skins/index.json` 和 `/skins/*.zip`；`GET, HEAD, OPTIONS`。
- 部署 `index.json` 与三个 zip 必须同批，继续由 `check-skin-packs.mjs --live` 做发布闸门。

若用户是“能看到卡片，但点下载失败”，那是另一条故障：

- 线上包能下载，且 Rust 下载不经过 WebView CORS；应查网络/代理、SHA 校验、磁盘写权限和包 manifest。
- 不能把这次的新装空市场结论直接套到“下载按钮报错”上。

## 2. Comic Pixel 当前视觉缺陷

### 2.1 三个红圈：同一根因

截图圈出的三处：

1. `按模型`
2. `今天` 下面的 `$64.01`
3. `Coding Agent`

源码对应：

- 暗色 `--cp-frame: #e6eadf`，本质是白色。
- `.spend-group-tabs .tab.active` 使用 `background: var(--cp-yellow); color: var(--cp-frame)`。
- `.overview-cat-tabs .tab.active` 同样使用 `color: var(--cp-frame)`。
- `.spend-col .col-total` 使用 `background: var(--cp-yellow); color: var(--foreground)`；暗色 foreground 又是白色。

对比度：

| 组合 | 对比度 |
|---|---:|
| 黑 `#0b0d0b` on 黄 `#e9ff1f` | 17.45:1 |
| 黄 `#e9ff1f` 上的白 `#e6eadf` | **1.09:1** |
| 青 `#00dfd0` 上的白 | **1.38:1** |
| 紫 `#7b4ff0` 上的白 | 4.65:1 |

正确做法：

- 新增固定的前景 token：`--cp-yellow-ink:#080a0d`、`--cp-cyan-ink:#03231f`、`--cp-purple-ink:#ffffff`。
- 所有 `background: cp-yellow` 强制配 `cp-yellow-ink`，不能再使用 `--foreground` 或 `--cp-frame`。
- 同理，所有 `background: cp-cyan` 强制配 `cp-cyan-ink`。
- 把颜色角色集中成一套 semantic pair，不许组件各自猜。

### 2.2 字体

本机 config：

```text
skinId = comic-pixel
uiFont = MV Boli
uiFontCjk = 空
density = compact
```

截图里的 provider 名非常花，和 `MV Boli` 一致。

建议边界：

- Hero 标题：保留 `Pane Comic Display` / 水墨字样。
- 设置分区头、章节标题：可保留主题 display 字体。
- Provider 名、额度、重置时间、告警、表单标签、按钮、详情、错误：锁定可读 UI 字体。
- 推荐栈：`"Segoe UI Variable Text", "Microsoft YaHei UI", "PingFang SC", sans-serif`。
- 不修改用户的全局 `uiFont` 设置，只在皮肤的数据区域覆盖。

### 2.3 组件千篇一律

当前 Comic Pixel 的重复语法：

- Hero、quota、provider、settings、dialog 大量使用 2–3px 实线边框 + 硬阴影 + 方角。
- 圆角/切角虽有，但每个区域力度相同，视觉上没有“主卡—数据卡—控件—状态”的层级。
- 颜色丰富，却因为重复同一种框体，整体仍像一套模板换色。

建议分成四种材质：

| 层级 | 建议材质 |
|---|---|
| Hero | 3px 酸黄框、放射线、硬阴影、像素大数字 |
| 数据卡 | 1px 墨线/虚线、状态侧条、少量网点；不再复制 Hero 阴影 |
| 交互控件 | 切角贴纸、按压缩进 2px；active 才用强调色 |
| 状态/空态 | 独立图标场景、告警侧条、加载扫描；不与正常卡片同构 |

### 2.4 图标范围不完整

当前 Comic Pixel 没有自己的图标包；`uiIcon()` 仍输出 Phosphor line SVG，皮肤只改了按钮外壳。截图里左上角四个按钮的形状和内部图标语言不一致。

建议：

- 为 Comic Pixel 提供 26 个 16px 点阵 SVG（已包含在 `prototype/assets/icons/pixel/`）。
- 侧栏、页面工具、设置分类、状态、空态、弹窗关闭、分享、折叠、复制、删除全部覆盖。
- 集成时优先使用 pack CSS mask，把 `--pix` 指向包内 SVG；这样不必把视觉资产硬塞进 `src/assets/icons`。
- Signal 用邮戳/票据框，Ink 用纸签/笔触框；不是三套都用同一套像素图标。

## 3. AIGC News 再分析

来源：[UI 灵感库](https://aigcnews.cn/ui-library)。本轮通过 ego lite 读取实时 DOM、CSSOM 与计算样式，并抓取实时 CSS；不是只看旧截图。

### 3.1 具体视觉事实

- 页面基底：纸白 `#fffef9`、墨蓝 `#071735`、酸绿 `#c5ff32`、硬阴影 `#142345`。
- 字体：正文 `DM Sans / PingFang SC / Microsoft YaHei`；页面标题 24/36、650；分区标题 20/30、750。
- 卡片：`.ui-library__card` 本身透明、无边框、无阴影；`.ui-library__poster` 是 16:10 深色海报框，圆角 8px，页脚标题 13/600、元数据 10px。
- 强调：当前“全部”筛选是白色文字压纯墨色；酸绿只出现在当前导航/少量高亮。大多数卡片的强调来自海报图本身。
- 资产：wordmark 是 546×132 独立 webp，并有上层/下层切片；撕纸角 `tear-peel.webp`；页面噪声为内联 SVG turbulence；其他视觉靠 640×400 海报。
- hover：海报里的“预览效果”按钮从透明淡入；没有整卡位移、旋转或彩色阴影。
- 结构：左侧 204px 按用途浏览；上方搜索；下面密集卡片。页面信息层级清晰，不靠统一重边框。

### 3.2 可迁移的原则

1. **安静底板 + 少量强强调。** 不要所有选中态都用荧光色；一个页面只保留 1–2 个主强调。
2. **海报承担个性。** Pane 的 Hero、工坊预览卡、空态插画可“有海报感”；provider 数据卡保持安静。
3. **小字号有严格层级。** 13/600 标题、10–12px 元数据；不把装饰字体用于这些位置。
4. **少量核心资产复利。** wordmark、撕纸、噪声、分隔线即可形成统一气质，不靠几十个互不相干的贴图。
5. **动作克制。** hover 只显露动作或轻微推进；动效集中到关键转场。
6. **角色色有语义。** 统计维度、计量单位、时间范围、危险状态各自固定，不随机换色。

### 3.3 不可直接照搬

- 204px 左侧栏、36px 大标题、3019 卡片密度不适合 380–450px 浮窗。
- 海报图、wordmark、撕纸素材有原站版权；Pane 只借鉴语法，不下载或复用原资产。
- AIGC 的页面 shell 本身也有低对比和 9–10px 信息，Pane 不应复制它的可读性下限。

## 4. 三套皮肤的共同目标

| 维度 | Comic Pixel | Signal / 云端邮局 | Ink / 山水笺 |
|---|---|---|---|
| 主强调 | 酸黄，仅 Hero 与维度 active | 深蓝/暖黄，集中在 active 与显示面 | 墨色，强调靠笔画与留白 |
| 框体 | 切角像素贴纸 + 局部硬阴影 | 圆角纸卡 + 轻投影 + 邮戳边 | 细纸边 + 不规则笔触 |
| 图标 | 16px 点阵 mask | 线性图标 + 邮戳框 | 线性图标 + 纸签/笔触框 |
| Display | Bangers | 稳重 Serif | Ma Shan Zheng |
| 数据字体 | 固定可读 UI 字 | 用户可读 UI 字 | 固定可读 UI 字 |
| 动效 | 像素格擦除 | 邮戳盖印 | 墨迹扩散 |

核心是“同一个语义角色，三套材质表达”，不是三套颜色和同一套组件。

## 5. 原型说明

入口：

```text
prototype/index.html
```

可直接用浏览器打开，也可加查询参数：

```text
?theme=comic-pixel&mode=dark&page=dashboard
?theme=comic-pixel&mode=light&page=dashboard
?theme=signal&mode=light&page=components
?theme=ink&mode=dark&page=dashboard
?page=settings
?page=market
```

交互：

- 左上太阳/月亮切换明暗。
- 左侧刷新按钮有 scan/jitter 反馈。
- 左侧调色盘打开皮肤工坊。
- 左侧扳手播放当前主题的 GSAP 转场。
- 顶部三组按钮切换主题；右侧说明区保持统一。
- `Esc` 返回看板。
- 系统 `prefers-reduced-motion` 下直接切换页面。

GSAP 转场：

| 主题 | 机制 |
|---|---|
| Comic Pixel | 从点击位置生成 22px 像素格，按距离/顺序硬切铺满，页面在两帧之间互换，再反向收起 |
| Signal | 圆章从按钮中心放大，抖三下，页在下一次落章时替换 |
| Ink | 6 个不规则墨团由点击处扩散，页面在墨面覆盖时替换，墨面淡开 |

所有动画都必须有 `reduceAnimations` / `prefers-reduced-motion` 的即时降级路径。

## 6. 建议实施拆票

> 以下只是草稿，尚未在 GitHub 或 `.scratch` 创建。

### 票 01：修复皮肤市场新装空态

- Rust 侧索引读取 + 前端 invoke fallback。
- 静态站补 CORS。
- 新 profile、空 `%APPDATA%\Pane\skins`、离线和代理四种验收。

### 票 02：修复 Comic Pixel 颜色与字体边界

- 新增角色 ink token。
- 修正黄/青/紫 active 与金额块。
- provider 名、额度、时间、表单锁定可读字体。
- 更新已安装包/线上包，否则源码改动不可见。

### 票 03：三主题组件覆盖与图标资产

- Comic Pixel 接入 26 个像素图标。
- Signal / Ink 补侧栏、设置导航、工坊、二级页面图标框体。
- 区分 hero/data/control/state 四种材质，减少重复粗框。

### 票 04：主题专属 GSAP 页面转场

- 抽出统一 `playSkinTransition(skin, source, swap)`。
- 三套时间线，减少动画直切。
- 接入设置、皮肤工坊、可能的二级设置页入口。

### 票 05：验收与打包

- 4 皮肤 × 亮暗 × 380/450 px。
- 重点页：主看板、设置、密钥额度、快捷键、皮肤工坊、空态、错误、加载。
- 市场新装与离线。
- `pnpm build`、Node tests、`parse-tests` 全量、`check-skin-packs --live`。
- owner 目检后才能进入提交/发布门禁。

## 7. 验收口径

| 项目 | 通过条件 |
|---|---|
| 颜色 | 黄/青 active 的前景对比度 ≥ 4.5:1；紫色 ≥ 4.5:1 |
| 字体 | provider 名与数据不再继承 MV Boli/Bangers；字号 ≥ 12px（关键元数据 ≥ 10.5px） |
| 组件 | 侧栏、设置 nav、行、输入、开关、按钮、工坊、状态和弹窗均有主题样式 |
| 图标 | 所有可见与二级页面按钮都有相应主题图标，不留下原生 Phosphor 混搭 |
| 动效 | 三主题转场可中断、可重播；减少动画时直接切页 |
| 市场 | 全新安装、无缓存、无已装包也能列出三款皮肤并可下载 |
| 打包 | 源码 CSS 与实际安装包/线上 zip 哈希一致，不能只在开发环境生效 |
| 回归 | 原生/Signal/Ink 样张不被 Comic Pixel 规则污染 |

## 8. 产物清单

- `research.md`：本文件。
- `coverage-matrix.md`：三主题 selector 覆盖差距。
- `prototype/index.html`、`style.css`、`prototype.js`：可交互三主题原型。
- `prototype/assets/icons/pixel/*.svg`：26 个原创像素图标。
- `prototype/assets/icons/line/*.svg`：原生线图标对照，仅用于 Signal/Ink 原型。
- `prototype/assets/vendor/gsap.min.js`：本地 GSAP 3 运行文件。
- `prototype-settings.png`、`prototype-market.png`、`prototype-components-signal-light.png`、`prototype-dashboard-ink.png`、`prototype-dashboard-comic-light.png`：验收截图。
- `aigc-ui-library-live.json`、`aigc-ui-library-live.css`、`aigc-index-live.css`、`aigc-reference-deep.json`：AIGC 现场证据。
- `skin-market-cors-probe.json`：WebView CORS 复现结果。

## 9. 未验证 / 待确认

- 没有修改真实 Pane 皮肤源码或安装包；原型通过静态浏览器截图验证，不等于 Pane WebView2 实机验收。
- 其他用户的具体失败界面未知；当前证据能确认“新装索引被 CORS 拦截”，不能替代“看到卡片但下载失败”的诊断。
- Signal / Ink 的最终视觉需要 owner 决定是否要同样重的结构性改造，还是只补齐图标/工坊/设置覆盖。
- 图标若进正式 pack，需要决定用 CSS mask 还是 app-level image；当前推荐 pack-local mask，避免扩大全局图标 API。
- 线上部署、重新打包、提交、推送均未执行。
