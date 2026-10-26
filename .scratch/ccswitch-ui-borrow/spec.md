# Spec: cc-switch UI 借鉴批次（图标/更新提示/授权中心/问号 + 用量统计页移植）

**Issue:** https://github.com/Aafff623/pane/issues/61

> 用户 2026-10-25 点名五项 + 工程项另立 `ccswitch-eng-borrow/`。证据：temp/research/20261025-ccswitch-ui-port.md（移植级细节，本地）。参照物 temp/cc-switch（v4.0.20，MIT）。
> 明确排除：绘画、提示词功能（超出 Pane 职责边界）；RequestDetailPanel 明细全套（Pane 无 per-request 数据源，除非票 08）。

## A. Provider 图标嵌入与尺寸（票 01）

cc-switch：32px 圆角框（h-8 w-8 rounded-8px）；tile 型（自带实心底）铺满 object-cover + 内描边 ring-black/10；透明 glyph 缩 22px 居中；列表卡恒白底不随主题；侧栏同资源两档（16/20px）；首字母 fallback `max(size*0.5,12)`。
Pane 现状：`.provider-icon` 固定 16px（styles.css:362），卡片头行尾，无底色框。
移植：图标移到 name 左侧、**24px 图标 + 30px 容器**（radius 8px、border var(--border)）；Pane 的 SVG 多为品牌色 glyph，先全走「透明型居中」，后续按是否自带 rect 底分档；容器底 var(--card)；hover 不单动图标。auth 分组头 24px 框可复用同参数（styles.css:9453）。

## B. 更新提示绿点 + 关于页更新态（票 02）

Pane 绿点已有（#panel-update-dot 7px，updateVersion 驱动，main.ts:17919）。cc-switch 增量在**关于页**：
1. 主按钮四态：检查更新 / 检查中 spinner / **更新到 vX（solid 高亮）** / 更新中（AboutSection.tsx:225-255）——Pane 有新版时按钮应切换高亮态（现恒「检查更新」，main.ts:18416-18460）。
2. 「检测到新版本 vX」独立行 + release notes 摘要（line-clamp-3）。
3. 更新日志按钮直达 GitHub release tag URL。
4. **Star 邀请横幅**：bg-action-soft 一行 + amber 星 + 关闭存 localStorage（pane:about:starDismissed）；关闭后 GitHub 链接补回链接行。
全部复用现有 updateVersion/update-available 事件，无新后端。

## C. 授权中心头像与账号信息（票 03 + 空态并入）

cc-switch：28px 圆头像；头像来源=OAuth 用户信息接口（GitHub /user avatar_url、Kimi identity.avatar、xAI 恒 None）随账号持久化；前端 `<img loading=lazy referrerPolicy=no-referrer>` + onError 降级字母（无本地缓存）；字母 fallback bg-subtle+首字母大写；行结构=名字+「默认」描边胶囊/「需要重新登录」红胶囊 + 第二行「域名 · 登录日期 · N 个供应商在用」；**空态两型**：手动添加型「还没有登录 X 账号」vs 自动发现型「还没有 X 账号。X 里已登录的号会自动出现在这里」。
Pane 现状：authRowHtml 结构已对齐（main.ts:19175）但 AuthAccountRow 无 avatar_url 字段（auth_center.rs:24-83），头像恒字母；空态只有一种（main.ts:19417）。
移植：① auth_center.rs 在 codex/copilot/kimi 装配处透出 `avatar_url: Option<String>`；② 前端 img+no-referrer+onError 字母降级；③ i18n 拆 auth.empty / auth.emptyAuto（agy/kimi 类自动发现族）。隐私判断：外链头像可接受（cc-switch 同款，URL 无凭据+no-referrer）。

## D. 问号悬浮提示组件（票 04）

cc-switch：lucide CircleHelp 14px（strokeWidth 1.5）放 20px 圆形命中区（-my-0.5 不占布局）；Radix Popover sideOffset 6/alignOffset -8/collisionPadding 8，max-w 300px 圆角卡=**标题 font-semibold + 正文 caption**；交互 hover 300ms 开/移开 200ms 关（移进卡片不关）、focus-visible 立即开、点击固定 pin、Escape/点外关、modal=false；与普通 tooltip 分工（HoverTip 报名字 / HelpTip 讲规则）。
Pane 现状：5 处 ？ 全靠 title→#hover-tip 单行气泡（main.ts:8794/4203-4245）。
移植：新建 helpTip({title,body}) 返回 span（icon 13px、--muted-foreground、hover --accent）；弹层复用 placeMenuAt（gap 8 防溢出已有）；类 .help-tip/.help-card（全 CSS 变量，三皮肤自动适配）；#hover-tip 加 data-rich 变体承载标题+正文；300/200ms+pin+focus-visible 同款。首批替换 5 处 ？（main.ts:5258/6054/11206/11552/16272）+ overview-peak-help。

## E. 用量统计页移植（票 05-08）

cc-switch 页=页头+筛选行（时间范围/应用 chip/供应商模型级联）+Hero 四格（成本/请求数/真实Tokens/缓存命中率，点格切换图表）+趋势图或 53 周热力图（range=all）+三 tab 明细（logs/providers/models，带计数徽标）。后端全是 cc-switch.db SQL 聚合；**Pane 已只读该库**（spend.rs:3145），移植成本在富聚合不在接入。

| 区块 | Pane 数据 | 难度 |
|---|---|---|
| 趋势图（天/小时桶） | fetch_spend_daily（365 天）内联 SVG 自绘 | 低 |
| 53 周热力图 | 同上，纯 CSS grid 四分位 5 色直抄（UsageHeatmap.tsx:357） | 低-中 |
| Hero 成本/Tokens | collect() 汇总求和 | 低 |
| Hero 请求数 | spend.rs DayMap 无此字段——聚合时新增 request_count | 中 |
| Hero 命中率+缓存四分量 | DayMap 压扁了——扩成 fresh_input/output/cache_read/cache_write 四分量 | 中 |
| 供应商 tab | ProviderSpend 直用 | 低 |
| 模型 tab | ModelSpend / daily models[] 前端聚合 | 低 |
| 请求明细表 | 不支持（无 per-request 行）——票 08 直读 cc-switch.db 可选 | 高 |

口径必抄：fresh-input 归一（Pane 已有 ccswitch_fresh_input）、命中率=cache_read÷(fresh+write+read)、速度/首字门槛（输出≥100 且 耗时−首字≥100ms 才计速；首字用中位数）。
明确不移植：跨源去重 SQL（Pane 已按天去重）、UsageSettingsSheet 同步开关、PricingConfigPanel（Pane 已有 pricing.rs 双入口会打架）、绘画/提示词、RequestDetailPanel（除票 08）。
实现方式：renderXxx(): string 模板 + replaceChildren + 事件委托（Pane 现有风格）；localStorage 记忆 tab/指标/范围；i18n usage.* 关键 key 搬运（zh 文案 MIT 可直用）；**不引 recharts**（Pane 内联 SVG 先例充分，main.ts:2779/4875）。

## 验收总则

每票独立可验收（见各票）；涉及行布局的票 01/04 合并验收；UI 视觉由 owner 本人验收；三皮肤+明暗全覆盖。
