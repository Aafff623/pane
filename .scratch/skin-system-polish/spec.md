# 皮肤系统补齐：市场可获取、Comic Pixel 可读性与三主题组件覆盖

**Status:** ready-for-agent  
**Type:** bug + UI polish + integration  
**GitHub:** [#57](https://github.com/Aafff623/pane/issues/57)  
**Feature dir:** `.scratch/skin-system-polish/`  
**Date:** 2026-10-25  
**Owner decisions:** 已确认四项：Rust 拉索引 + CORS 兜底；数据区强制可读字体；Signal/Ink 保持现有方向只补覆盖；GSAP 先覆盖设置与工坊。

## 1. 用户需求

1. 其他用户新装 Pane 后应能正常列出并下载官方皮肤。
2. Comic Pixel 当前有真实可读性问题：三处黄底白字、provider 名和额度信息过花。
3. 组件丰富度不足：侧栏、设置分类、皮肤工坊等图标仍混用原生图标；卡片材质过于同质。
4. 不仅补 Comic Pixel；Signal 与 Ink 的同类组件也要补齐。
5. 点击设置时使用主题专属 GSAP 转场，不能所有皮肤共用同一种动画。
6. 最终由主对话接续实施；本调研对话不改共享工作树。

## 2. 已确认证据

### 2.1 市场

- `https://pane.threetwoa.live/skins/index.json` 返回 200，但无 `Access-Control-Allow-Origin`。
- 开发页来源 `http://127.0.0.1:1420`，浏览器 `fetch` 实测 `TypeError: Failed to fetch`。
- 三个 zip 均 200；`check-skin-packs.mjs --live` 三个包 size/sha 全匹配。
- 本机已有安装 pack，因此索引失败时仍看得见皮肤；全新用户没有这个兜底。

### 2.2 可读性

- 暗色 `--cp-frame:#e6eadf`，是白色。
- 黄色/青色 active 与金额块错误使用 `--cp-frame` 或 `--foreground` 作为前景。
- 黄底白字约 1.09:1；黑字约 17.73:1。
- 本机 `uiFont=MV Boli`，provider 名和额度信息继承了该装饰字体。

### 2.3 安装包漂移

- 仓库 `skins/comic-pixel/skin.css`：57,534 B（20:04 后仍有主对话改动）。
- 本机安装与线上 1.0.0 zip 内：55,096 B。
- 主对话已生成 1.0.1 版本的候选 zip；实施前必须以当时工作树为准重新核对，不能沿用本条的旧数字。

### 2.4 参考站

- AIGC News：纸白/墨蓝/酸绿；卡片本体透明无边框，海报承载个性。
- 页面标题 24/36、650；卡片标题 13/600；元数据 10px。
- 酸绿只用于少量导航与高亮，绝大多数 active 使用墨色反白。
- 参考原则：安静底板、少量强强调、海报承担个性、很少的核心资产。
- 不复制其图片、wordmark 或撕纸素材。

## 3. 设计目标

| 层级 | Comic Pixel | Signal | Ink |
|---|---|---|---|
| Hero | 3px 酸黄框、射线、硬阴影 | 纸卡 + 轻投影 + 邮戳 | 纸面 + 细边 + 笔触 |
| 数据卡 | 1px 墨线/状态侧条，轻装饰 | 圆角纸卡、弱边线 | 纸签行、底部笔触 |
| 控件 | 切角贴纸、按压位移 2px | 圆角票据控件 | 不规则纸签 |
| 状态 | 独立图标场景、告警侧条 | 邮局告警卡 | 墨迹/朱砂状态 |

所有 theme 的 active 配色必须成对定义，不能在不同组件间借用 `--foreground` / `--border` 猜前景。

## 4. 字体边界

保留装饰字体：

- `总花费` / `额度总览` 模块标题。
- 设置分区标题。
- 工坊详情标题。

强制可读 UI 字体：

- Provider 名。
- 额度、百分比、reset、meta。
- 设置 label、description、value。
- 表单、按钮、菜单、告警、错误。

不修改用户全局 `uiFont` / `uiFontCjk` 配置。

## 5. 资产

调研包内已有：

```text
C:\Users\Lenovo\.codex\visualizations\2026\10\25\01a172dc-24f5-7301-a38f-8b44737c1ba0\skin-pixel-20261025\prototype\assets\icons\pixel
```

- 26 个原创 16×16 SVG。
- 供 Comic Pixel 使用。
- Signal / Ink 使用各自框体语言，不直接套同一套像素图标。
- 正式进入 pack 时优先用 CSS mask，资产放在 `skins/comic-pixel/assets/icons/pixel/`。

原型：

```text
C:\Users\Lenovo\.codex\visualizations\2026\10\25\01a172dc-24f5-7301-a38f-8b44737c1ba0\skin-pixel-20261025\prototype\index.html
```

原型用于视觉与交互验收，不是可直接覆盖真实 Pane CSS 的成品。

## 6. 实施范围

### A. 市场读取

- 新增 Rust 索引获取函数/命令，复用 `send_with_direct_fallback`。
- 前端优先 invoke，浏览器 fetch 只留 demo fallback。
- 保留 localStorage last-good 缓存。
- 静态站补 CORS 头，覆盖 index 与 zip。
- 错误态区分网络、HTTP、JSON/结构错误。

### B. Comic Pixel 颜色与字体

- 固定 yellow/cyan/purple 前景 token。
- 修复所有 active、金额块和黄/青背景组合。
- 数据区域使用可读字体栈。
- 不把 display 字体用于 provider/data/forms。

### C. Comic Pixel 图标与材质

- 侧栏主题/刷新/工坊/组件/设置图标。
- 设置分类：常规、密钥额度、快捷键、账号。
- 工坊：返回、模式、页面、使用/下载/删除。
- 状态：空、载、错、警告。
- 把 Hero/数据卡/控件/状态拆成不同材质。

### D. Signal / Ink 覆盖

- 侧栏按钮与图标框体。
- 设置 nav 与二级页。
- 皮肤工坊的列表、预览、状态、动作。
- 空/载/错与弹窗的收口。
- 不重做既有视觉方向，不复制 Comic Pixel 结构。

### E. GSAP

- 抽一个统一转场接口，三套时间线：
  - Comic：22px 像素格擦除。
  - Signal：邮章盖印。
  - Ink：墨团扩散。
- 先覆盖“设置入口”和“皮肤工坊”。
- 支持重播、可中断、`reduceAnimations` 直切。

### F. 打包与验收

- 重新构建 pack zip 与 index。
- 安装新 pack 后清 WebView2 cache。
- 新 profile / 空 skins 目录验证市场。
- 4 主题 × 亮暗 × 380/450 × 关键页面。
- 分享 PNG 验证图标 mask 与装饰字体。

## 7. 非目标

- 不重做信息架构、窗口形态、设置保存语义。
- 不改真实数据、图表几何和 provider logo。
- 不做第三方皮肤市场或任意 CSS 执行模型。
- 不把 AIGC News 资产带入仓库。
- 不在本批扩展到所有页面的动画。

## 8. 验收命令

```powershell
corepack pnpm build
corepack pnpm test
node --test site/site.test.mjs
node site/scripts/check-skin-packs.mjs --live

cd parse-tests
$env:PATH = "D:\Tools\mingw64\bin;$env:PATH"
Remove-Item Env:HTTP_PROXY,Env:HTTPS_PROXY,Env:ALL_PROXY -ErrorAction SilentlyContinue
cargo +stable-x86_64-pc-windows-gnu test -- --test-threads=1
```

UI 验收由 owner 本人完成；自动化通过不等于视觉、转场手感或新装场景已验收。

## 9. 票

1. `issues/01-skin-index-transport.md`
2. `issues/02-comic-contrast-and-fonts.md`
3. `issues/03-comic-icons-and-material-tiers.md`
4. `issues/04-signal-ink-component-coverage.md`
5. `issues/05-theme-gsap-transitions.md`
6. `issues/06-pack-rebuild-and-acceptance.md`

依赖：01 独立；02 独立；03、04 blocked by 02；05 blocked by 03、04；06 blocked by 01–05。

## 10. 调研产物

- `research.md`
- `coverage-matrix.md`
- `implementation-guide.md`
- `prototype/`
- `prototype-*.png`
- `aigc-*` 现场证据
- `skin-market-cors-probe.json`

这些文件不属于应用运行时代码。主对话可只读取，不必把整包纳入仓库。
