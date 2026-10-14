# 豆包卡换成豆包自己的头标

**What to build:** 卡片上的豆包图标换成豆包应用头标（owner 指认的二次元形象）。侦察结论：现有 `src/assets/providers/doubao.svg` 是豆包「种子/花」品牌标、并非火山方舟标；owner 要的是应用头标。**需要 owner 提供素材或授权取图**。

**Blocked by:** None（owner 2026-10-14 已授权从官方公开来源获取）

**Status:** in-review（官方素材已替换，owner 视觉终验待做）

- [x] 核实卡片实际渲染的图标来源（providerVisual → iconKey → doubao.svg）并留证
- [x] 拿到 owner 素材（或授权的公开来源）后替换 doubao.svg，保持 1em/24×24 viewBox 约定
- [ ] 卡片、总览小卡、临期行三处显示一致；深浅主题下都可辨
- [ ] 若素材未到位：保持现状并在票内记录「待素材」，不得用火山方舟标充数

## Comments

- 2026-10-14：由 spec `.scratch/release-082/spec.md` 拆分（owner 批次需求）。

- 2026-10-14：owner 在当前对话明确授权官方公开取图。读取 https://www.doubao.com/chat/ 的icon标签，使用官方192px头像 https://lf-flow-web-cdn.doubao.com/obj/flow-doubao/favicon/new-doubao/192x192.png 。SHA256 `dabb2abd94e3a6c11e7f1a42c9343839b6ef643c086751b71afbc68f615aec38`。保留原始PNG字节，内嵌至24×24 SVG包装，供providerVisual共享；未重新绘制或改像素。该素材为豆包官方品牌资产，不声明为本项目MIT原创。
