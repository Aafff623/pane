Status: ready
Issue: https://github.com/Aafff623/pane/issues/60
Priority: P2
Area: UI

# 用量拓扑总览：吉祥物枢纽 → provider 分行 → 模型端点 → 活跃流动线

## 需求（用户原话归纳）

做一个像 magpie「路由」页那样的拓扑界面，但**不搞路由设置**（Pane 只读、不代理请求），只借它的 UI 布局格式与组件，内容换成当前使用的 provider 和模型：

1. **图标与吉祥物**：换掉 magpie 的喜鹊，用 Pane 自己的品牌吉祥物（主形 `src/assets/brand/pane-mark.svg`，pane-brand-icons 管线的唯一源头），并与主题/皮肤绑定；喜鹊飞行动效换成另一种形式（CSS/SVG 优先，Lottie 为可选方案），可用 oil-ui-pro 技能做深度美化。
2. **布局**：同一 provider 排成一行（如 z.ai 的 GLM-5.3 / 5.3-Flash 一行），多个 provider 多行；连线从吉祥物枢纽出发流向对应模型端点；用 MiniMax 就排第二行，依此类推。
3. **展示与"传输"**：显示当前使用的模型（哪个 provider 的哪个模型在用，右侧对应展示）；活跃时画流动线（对应 magpie 的"回答了这个请求"高亮）。

## magpie 实现机制（深挖结论，移植级证据）

`temp/research/20261025-magpie-ui-mechanics.md` §一。核心：三明治结构（SVG 连线层 → DOM 卡片层 z1 → SVG 天空层 z2）；连线=SVG path 手算 d（getBoundingClientRect 换算 stage 本地坐标，S 形三次贝塞尔控制点取两端中点竖直拉平，layout() client.js:1415-1458）；流动=纯 CSS `stroke-dasharray:3 5 + animation dashoffset`（routing.css:12-16）；实时高亮=长轮询增量事件 → li.on/li.now 三态圆点（box-shadow 脉冲）→ LINGER 12s 降级清场；性能=rAF 按需（busy 才续帧）+ 不可见全停 + 测量缓存（WeakMap，getTotalLength/getPointAtLength 极贵）+ FLIP 折叠。

## Pane 版设计（只读替换）

| magpie 概念 | Pane 替换 |
|---|---|
| 左列 agent 卡（Codex） | 会话/工具来源行（来自 spend 数据的 agent 维度；或精简为吉祥物单枢纽，owner 定） |
| 中央枢纽（喜鹊 logo + 网关地址） | **Pane 吉祥物**（pane-mark.svg，主题色绑定）+ 副标「本地只读 · 不代理」 |
| 右列 provider 分组 → 账号行 | **provider 分行 → 模型端点行**（分组=provider 卡片快照 Snapshot；子行=ModelSpend 每模型消耗，main.ts:129-152） |
| "此刻在回答"（网关 trace 事件） | **最近用量刷新**：`listen("usage-updated")` 事件 + 指标变化检测 → 该 provider 的 wire 加 `.live` 流动 + 行 `.now` 脉冲，LINGER 8-12s 过期降级 `.lit`/熄灭 |
| --agent 哈希上色 | provider 品牌色（src/providerVisuals.ts 现成） |
| tries 重试飞行/replay 时间轴/rest 文案 | **不做**（依赖网关语义） |

分层：数据层（快照+事件）→ 布局层（grid 舞台三列：124px 124px 1fr 起步，ResizeObserver 触发 layout）→ 绘制层（每行一条 wire path，d 只在 layout/折叠/FLIP 时写）→ 动画层（CSS dashoffset 流动 + 圆点脉冲，prefers-reduced-motion 降静态高亮）。

## 性能门（照抄 magpie 四招）

rAF 只在 busy（有流动/FLIP）时续帧；面板不可见全停（inSight 门 + visibilitychange）；measure() WeakMap 缓存 path 长度与采样点；renderAll 合并单 rAF、setText 变化才写。

## 验收

见 issues/；UI 视觉与动效由 owner 本人验收；与 #49/#55/magpie-settings-polish 的丝滑基座共用动画令牌。

## 许可

magpie routing 插件 MIT；连线 d 公式与 dashoffset 手法可照搬（带声明）；喜鹊飞鸟层与 tries 叙事不搬。
