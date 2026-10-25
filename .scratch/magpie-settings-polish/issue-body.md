## 问题

Pane 的设置面板骨架已按 QuickClipboard 重构完成（8 张票已实现、未做用户验收），但 owner 反馈「有些地方显示的不是很好」。对照参照物 Magpie 的设置界面，需做一轮细节微调：抄其精炼的精髓点，不大改骨架。

与骨架重构的关系：这是对既有骨架的「内容层打磨」，不重做外壳、不引入第二套 UI。

## 已核实

- 参照物源码：`yetone/magpie` @ `defbb1a`（本地只读副本 `temp/magpie-yetone/`），设置 UI 为 `internal/gui/assets/` 的 `index.html`（`#view-settings`）+ `app.css` + `app.js`；已用 headless Edge 按真实 CSS 静态渲染常规/额度/隐私页亮暗原型图，并与 Pane 10-24 实拍（Signal 暗）双栏对比，看板在 `temp/ui-diff-20261025/index.html`。
- Magpie 精髓：全设置页零 toggle switch，38 处控件全是「灰轨道 + 白色滑动 thumb」的分段 pill；说明文字全部行内副标题（默认单行省略、个别特例换行）；行间分隔线比卡片边框浅一档；危险按钮 hover 才变红；侧栏 hover 只有底色无位移。
- Pane 现状问题（毒舌评审，全部 file:line 取证）：
  - 迁移行仍是原生 select/checkbox/number 混排（`src/settings/controls.ts`），与骨架精致感不匹配；
  - 说明文字双写：行内副标题 + 控件 `title=` 重复同一句（`src/main.ts` `panelSwitchRow`/`adoptControl`）；
  - 副标题全换行导致单行约 70px（Magpie 约 40px）；
  - 行分隔线与卡片边框同档同色（`styles.css:10244`）；
  - 危险按钮 native 恒红、三个皮肤包里也是恒红规则；
  - 侧栏 hover 有 translateX(2px) 位移。
- 关键约束：Pane 已有 Magpie 同款分段控件 `.tabs/.tab`（`styles.css:2651-2678`，已用 5 处），三个皮肤包均只样式化既有类名——新造类名会让皮肤失明。
- 皮肤包运行时 CSS 从远端 zip 注入；改 pack 需重新打包 zip 并更新 `site/public/skins/index.json`（当前 generatedAt 2026-10-23，与仓库源版本号一致）。

## 目标

- 把 6 个静态 select（`#locale`、`#appearance`、`#timeformat`、`#jump-animation`、`#spend-tier-mode`、`#update-frequency`）换成 Magpie 式 pill 控件，复用 `.tabs/.tab` 视觉体系；`#pinned`（选项数运行时动态生成）明确排除。
- 行为零变化：隐藏 `<select>` 作唯一真值源，pill 只派发 change；`#jump-animation` 的延迟保存语义和 `#spend-tier-mode` 的联动原样保留；aria 达标（容器 role=group、选项 aria-pressed）。
- 说明文字去重（删 `title=` 双写）+ 副标题两行封顶，行高回落。
- 行分隔线改浅一档（新增 `--row-line` 语义 token，四处规则统一引用，含特异性更高的 `.set-section__card > .setting-row` 与 `.backup-title`）。
- 微调对齐：危险按钮 native 静态中性、hover 才红；侧栏 hover 去位移。
- 四个皮肤 × 亮暗继续工作；皮肤包改动（signal 分段轨道、三个包的恒红重置按钮、comic-pixel 重置按钮圆角既有 bug）单独成票，需 owner 拍板是否重新打包 zip。

## 范围与边界

只做设置面板内容层与样式微调。不重做外壳、侧栏激活态、分区 tab 头；不改任何设置的保存语义（保存按钮行为保持现状）；不做搜索交互重构（Magpie 式行内过滤列为范围 E，Pane 现有跨页索引+双向覆盖校验更防呆，不动）；布尔开关保留现有 switch（skin 已适配）；不引入 UI 框架。

`#pinned`（托盘图标显示，选项数=1+N×M 且语言切换重建）和 `.st-radio-group`（窗口形态二选）暂不换 pill——后者留 owner 明确「留」或「顺手换」。

## Spec 与实施票

权威规格：`.scratch/magpie-settings-polish/spec.md`。

1. `issues/01-pill-segs.md`：6 个静态 select 换 pill 控件（复用 `.tabs/.tab`，隐藏 select 真值源）。
2. `issues/02-desc-dedupe-and-clamp.md`：说明去重 + 副标题两行封顶。
3. `issues/03-row-line-and-hover-trim.md`：`--row-line` 两档线（四处规则）+ 危险按钮 hover 才红 + 侧栏去位移（native）。
4. `issues/04-skin-pack-adjustments.md`：三个皮肤包调整 + 重新打包 zip + 更新 index.json（需 owner 授权打包）。
5. `issues/05-native-screenshots-and-acceptance.md`：native 默认主题截图补齐 + 集成验证 + owner 验收。

依赖：01 与 02 改同一文件相邻区域，串行更稳；04 依赖 01+03；05 依赖全部。本地票以 `Status: ready-for-agent` 起步，执行时领取并更新。

## 验收

- [ ] 6 个 select 全部换成 pill，观感对齐 Magpie 原型图；点击行为、联动、语言切换、延迟保存零回归。
- [ ] 说明文字无双写；副标题两行封顶后行高显著回落，无截断丢关键信息。
- [ ] 两档分隔线在 4 皮肤 × 亮暗下可见且不突兀；native 亮/暗各一张新截图核对。
- [ ] 危险按钮、侧栏 hover 按预期；键盘可达性不退化。
- [ ] 皮肤包改动如需打包，zip 与 `site/public/skins/index.json` 同步更新，并说明与上传/发布的边界。
- [ ] `pnpm build`、相关 Node 测试、代码审查与冗余检查完成，范围如实记录。
- [ ] 用户亲自进行 UI 验收；未验收前不宣称完成，不提交、推送、打 tag、上传或发布产物。

调研底稿与 diff 看板在维护者本地（`temp/research/20261025-magpie-settings-ui-analysis.md`、`temp/ui-diff-20261025/`），不向公开 issue 上传本机路径、配置或运行数据。
