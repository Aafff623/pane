## 问题

### 1. 新安装无法从皮肤市场取得皮肤（P1）

线上 `https://pane.threetwoa.live/skins/index.json` 与三个 zip 均返回 HTTP 200，索引中的 size/sha256 也通过 `site/scripts/check-skin-packs.mjs --live`。但响应没有 `Access-Control-Allow-Origin`：

```text
HTTP/1.1 200 OK
Content-Type: application/json
(no Access-Control-Allow-Origin)
```

在 Pane 开发页（来源 `http://127.0.0.1:1420`）执行：

```js
await fetch("https://pane.threetwoa.live/skins/index.json", { cache: "no-store" })
```

实测为 `TypeError: Failed to fetch`。已安装用户因为本地 `%APPDATA%\Pane\skins` 还有 pack，看起来正常；全新安装没有本地 pack 时，市场只剩空态。

### 2. Comic Pixel 可读性

截图确认三处黄底白字不可读：

- 总花费 → 按模型
- 时间范围的今日金额
- 额度总览 → Coding Agent

暗色 `--cp-frame` 为白色，部分选中态把它或 `--foreground` 当成黄色/青色背景上的文字色。黄底白字约 **1.09:1**；正确深色前景约 **17.73:1**。

同时，数据名称受用户字体 `MV Boli` 影响，provider 名和额度信息过花。主题装饰字体应只用于 Hero/章节标题；provider 名、额度、重置时间、告警、表单标签必须使用可读 UI 字体。

### 3. 三套皮肤组件覆盖不完整

- Comic Pixel 仍复用原生 Phosphor 图标，侧栏、设置分类、工坊等按钮内部没有自己的图标语言。
- Signal / Ink 的侧栏按钮、皮肤工坊、部分二级导航与工作坊控件缺少专属样式。
- 当前多数组件共用 2–3px 边框 + 硬阴影，Hero、数据卡、控件和状态缺少材质层级。

### 4. 源码安装包漂移

当前仓库 `skins/comic-pixel/skin.css` 为 57.5KB；本机安装包与线上 zip 内为 55.1KB，缺少新设置骨架选择器。只改源码不重打 zip，用户端看不到。

## 参考结论

AIGC News 的可迁移原则是：安静底色、透明卡片、海报承担个性、少量强强调、清楚的标题/元信息层级。不要复制其版权资产，也不要把满屏贴纸当成设计。

## 实施范围

1. 皮肤索引改为 Rust 拉取并解析，保留浏览器 fetch 作为网页 demo fallback；静态站补 CORS 头作为第二层。
2. Comic Pixel 增加固定的 yellow/cyan/purple 前景 token，修正所有选中态和金额块。
3. 数据区域使用皮肤内可读字体，不改用户全局字体；装饰字体只留标题。
4. Comic Pixel 新增完整 16px 像素图标集；Signal / Ink 补齐各自图标框体与工坊/设置覆盖。
5. 把 Hero、数据卡、控件、状态拆成不同材质，减少重复粗框。
6. 设置入口与皮肤工坊增加主题专属 GSAP 转场；支持打断与减少动画直切。
7. 重打皮肤包并同步 index；新装、离线、亮暗、380/450px、分享 PNG 全部验收。

## 验收

- 全新 profile、空 `%APPDATA%\Pane\skins` 能列出并下载三款皮肤。
- 黄/青/紫 active 与金额块对比度 ≥ 4.5:1。
- provider 名、额度、时间和表单不被装饰字体污染。
- 四个主题 × 亮暗、380/450px、看板/设置/密钥额度/工坊/空态/错误/加载。
- 三主题转场可播放、可中断；减少动画直切。
- `pnpm build`、Node tests、parse-tests、site tests、`check-skin-packs --live` 全绿。
- owner 亲自完成 UI 验收后才能进入提交/发布门禁。

## 非目标

- 不重做 Pane 信息架构。
- 不改真实数据、额度与图表几何。
- 不复制 AIGC News 图片素材。
- 不在本 issue 内扩成第三方皮肤平台或任意 CSS 包安全模型。
