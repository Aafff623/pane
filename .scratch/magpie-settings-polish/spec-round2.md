# Spec 第二轮扩展：侧栏二级菜单 / 文字字号字体 / pill 推广 / 全局丝滑基座

> 2026-10-25 用户追加（同日晚）。本轮由 magpie UI 机制深挖支撑，证据与移植级细节见 `temp/research/20261025-magpie-ui-mechanics.md`（本地文件）。第一轮 spec（本目录 spec.md）仍然有效；本文件只管 06-09。

## 06 · 全局丝滑基座（CSS 优先，全应用受益）

magpie 丝滑 = 四个习惯：只动 transform/opacity、按下即反馈、页面常驻 DOM+hidden 切页+CSS 重启入场、测量全走 ResizeObserver。Pane 的 `activatePanelPage` 已是 block.hidden 切页（main.ts:17728-17738），天然兼容。

做法（按成本）：
1. 切页入场动画：`.view`/面板页 `animation: page-in .24s cubic-bezier(.2,.8,.2,1)`（opacity 0→1 + translateY 5px→0）；display:none→block 自动重启，JS 零驱动。
2. 全局 `--ease: cubic-bezier(.2,.8,.2,1)` 令牌；hover 统一 background/color .15s；`:active { transform: scale(.96) }`。
3. 折叠一律改 `grid-template-rows: 0fr→1fr` transition + overflow:hidden（WebView2 原生支持），替代 JS 测高/max-height；chevron 用 `[aria-expanded="true"]` 选择器旋转。
4. `prefers-reduced-motion` 全局降级块。
5. `@container` 窄容器查询替代窗口媒体查询（styles.css 相应区块加 container-type）。
6. pointerdown 先行选中：侧栏/分段按钮 `onpointerdown` 先移高亮、click 才执行（消"点了没反应"）。
7. 大列表渲染统一 `replaceChildren` + createElement 助手，弃 innerHTML 拼接（main.ts 渲染路径盘点）。

## 07 · 侧栏二级菜单（设置 → 子页丝滑展开）

参照 magpie：设置入口下挂子页（截图所见 常规/额度/网络与共享/模型/隐私/可观测性/同步与备份/关于 八项）。
- Pane 落点：设置面板侧栏（200px 文字栏，#49 骨架）加树形子菜单——「设置」项展开其子分区；看板侧栏若分区过多可同样收纳。
- 动画：展开用 `grid-template-rows: 0fr→1fr`（06-3），chevron 旋转，子项 `--i` stagger（ magpie app.css:691-696 同款）。
- 状态：aria-expanded + localStorage 记忆；键盘可达（Arrow 上下在子项间）。
- 信息架构：先盘点 Pane 现有设置分区，提出分组方案（常规/外观/花费/更新/关于…）交 owner 拍板再实施。

## 08 · 文字大小四档 + 代码字体 + 色盲模式

参照 magpie 设置/常规页（截图与 index.html:166-178 textRow/uiFontRow/codeFontRow）：
1. **文字大小**：100%/110%/125%/150% 四档 pill（正好示范 pill 推广）；落点=CSS 根字号变量 + textScaleFactor 联动（main.ts:8933 现为 popover 自适应缩放，需拆出用户偏好因子）；持久化进 config.json；三语 i18n。
2. **代码字体**：双下拉（字体+字重）+ 预览行「Aa 0123 ·中文」；Pane 已有 #ui-font/#ui-font-cjk（main.ts:18022-18028）可复用其字体枚举与 datalist 模式；新增代码字体变量供卡片数字/等宽区使用。
3. **色盲模式**：开关（或 pill 关/开）；与燃烧曲线联动（曲线色盲态用 dash 图例区分，见 meter-display-honesty#04 的预留）；magpie 色盲模式仅换 dash/dot 不换色相（app.js:16478）——照此最小实现。

## 09 · pill 分段控件大面积推广

第一轮已完成 6 个静态 select（票 01）。本轮收编遗留：
1. `.st-radio-group`（窗口形态二选）——第一轮 spec 留 owner 拍板，本轮请 owner 定。
2. 新增设置项（08 的文字大小、色盲）直接用 pill。
3. 盘点其余 radio/checkbox 语义实为「互斥多选一」的控件，评估收编；布尔开关保留 switch（owner 既定决策）。
4. thumb 滑动升级：第一版用背景切换（票 01 既定），本轮按 magpie 方案补真滑动（WAAPI transform+width 280ms + ResizeObserver 重定位 + `.still`+void offsetWidth 免动画，约 60 行 JS；跨重绘续滑 Map 可省）。

## 验收

- 06：切页/折叠/悬停动画统一走 --ease；reduced-motion 降级生效；`pnpm build`+`pnpm test` 绿。
- 07：展开收起丝滑、键盘可达、刷新后记忆；owner 视觉验收。
- 08：四档字号即时生效且与 popover 缩放不冲突；代码字体预览正确；色盲模式下曲线可辨；三语 key parity。
- 09：owner 拍板 .st-radio-group 后实施；thumb 滑动无布局抖动（性能面板无 layout 突刺）。

## 不做

- View Transitions 布局形变、rAF 弹簧积分器、弹窗 clipPath 长出（结构性改动，CSS 方案已够；magpie 也只用于个别场景）。
- 虚拟滚动（Pane 列表规模未到；用分页/replaceChildren 即可）。
