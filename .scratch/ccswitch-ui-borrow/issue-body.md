Status: ready
Priority: P2
Area: UI

# cc-switch UI 借鉴：图标嵌入 / 更新绿点与关于页 / 授权中心头像 / 问号悬浮 / 用量统计页移植

## 需求（用户 2026-10-25 点名五项）

1. **图标嵌入与尺寸**：cc-switch 的 provider 图标大而自然（32px 圆角框、tile/透明分档、内描边），Pane 现在固定 16px 偏小——调大并抄嵌入细节。
2. **更新提示与关于页**：设置入口绿点 Pane 已有；缺的是关于页更新态——有新版时主按钮变「更新到 vX」高亮、「检测到新版本」行、更新日志直达 release、可关闭的 GitHub Star 横幅。
3. **授权中心完善**：授权后展示真实头像（OAuth avatar_url 透出 + 外链 img + 字母降级）与基本信息；空态拆「手动添加型/自动发现型」两种文案。
4. **问号悬浮**：小问号（14px，圆形命中区不占布局）+ hover 弹出「标题+正文」卡（300ms 开/200ms 关/点击固定/键盘可达），替换现有 5 处 title 式 ？。
5. **用量统计页**：整体移植（时间范围/趋势图/53 周热力图/Hero 四格/供应商与模型 tab）——Pane 已只读 cc-switch.db，成本在富聚合（请求数+缓存四分量）不在接入。

**明确排除**：绘画、提示词功能（超出 Pane 职责边界）；RequestDetailPanel 明细全套（无 per-request 数据源，可选票 08 直读 cc-switch.db）。

## 方案要点（详见 spec.md）

- 图标：24px 图标 + 30px 圆角容器（radius 8px、border var(--card)/var(--border)），行首化；auth 分组头复用同参数。
- 关于页：复用现有 updateVersion/update-available，无新后端；横幅关闭存 localStorage。
- 头像：auth_center.rs 加 avatar_url: Option<String>（codex/copilot/kimi 装配点），img+no-referrer+onError 降级；外链头像可接受（URL 无凭据）。
- helpTip：新组件 helpTip({title,body})，复用 placeMenuAt 定位；.help-card 全 CSS 变量（三皮肤自动适配）。
- 用量页：不引 recharts，内联 SVG 自绘（Pane 先例充分）；趋势/热力/Hero/供应商/模型五块按数据就绪度分 4 票递进；口径必抄（fresh-input 归一已有、命中率公式、速度/首字门槛、首字中位数）。

## 验收

每票独立验收（见 issues/）；票 01 与 04（图标与问号都动行布局）合并验收；三皮肤+明暗全覆盖；UI 视觉由 owner 本人验收。

## 不移植清单

绘画/提示词；跨源去重 SQL（Pane 已按天去重）；UsageSettingsSheet 同步开关（扫描管线不同）；PricingConfigPanel/ModelsDevAutoSyncPanel（Pane 已有 pricing.rs，双入口打架）；claude-desktop 折叠等 cc-switch 特有记账概念。
