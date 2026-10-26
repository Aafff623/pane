Status: ready
Priority: P2
Area: UI

# ZCode 借鉴：首启引导 / 快捷键体系 / 使用统计页

## 需求（用户 2026-10-25 点名三项，明确不抄其他）

1. **引导流程**：ZCode 全屏三步首启向导的骨架与交互（可跳过/可返回/ESC 关不再弹/判定失败保守不弹）；Pane 三步设计=欢迎+托盘说明 → 快捷键设置 → provider 扫描完成页；config.json 双布尔记状态，settings 加重看入口。
2. **键盘快捷键**：ZCode 的 shared 命令表 + 单点 capture 分发 + 纯函数内核（IME 过滤/物理等价归一/保留键黑名单/抢绑确认/多绑定）+ 按组合键搜索的设置 UI；Pane 现状分发散在 ~15 处 keydown、冲突检测只是字符串比较。Pane 的系统热键（Alt+2/Shift+1）独立不动。
3. **使用统计页**：用户方案已定——每日详情移出悬浮窗、使用统计成为设置左栏独立项、页面对齐 ZCode（KPI 五格 + 52 周热力 + 范围 tabs + 每日按模型折线 + 模型 donut）。Pane 数据底子更强（365 天持久台账 vs ZCode 30 天滚动），Rust 几乎不动；**不引 Recharts**，自绘 SVG。

## 许可与署名

ZCode 是 **Apache-2.0**，可正式抄码：LICENSE 追加 ZCode 署名（照现有 OpenUsage 段落先例），整函数级照抄（快捷键内核 bindings/conflicts）保留版权头。

## 关键落点

- 引导：renderOnboarding() 模板 + 单 keydown capture，复用 tour overlay 定位（main.ts:8205 tourSeen 先例）。
- 快捷键：命令表+纯函数内核移植 + ~15 处 keydown 收拢单监听；设置 UI 按键搜索/抢绑确认/行级多绑定。
- 统计页：PANEL_PAGES 新增 stats 项（**usage id 已被密钥页占用，必须用新 id**，main.ts:17868）；KPI/热力/选日明细（main.ts:4537-4623 内容迁入）；删除悬浮窗 overlay 与 data-spend-details 按钮（:4775/4973）；热力 CSS 扩展 .spend-heat-*（styles.css:2981-3067）；图表照抄 ZCode 四规则（Y 轴单点峰值/0 值过滤/稀疏刻度/紧凑单位）。

## 验收

见 issues/ 各票；三项 UI 全部 owner 本人验收；三皮肤+明暗全覆盖。
