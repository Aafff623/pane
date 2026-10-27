# Spec: oil-ui 卡片移植（beam-graph / metric / terminal / keycaps）

**Status:** ready · **Priority:** P2 · **Area:** Site
**Issue:** https://github.com/Aafff623/pane/issues/88
**Evidence:** temp/research/20261025-website-ui-deepdive.md §三（oil-ui-pro 组件清单+许可警告）

## 定位

把 oil-ui-pro 技能库的四款卡片原型适配进 Pane 官网。**前置硬条件：原型代码（~/.zcode/skills/oil-ui-pro/assets/cards/）无 LICENSE 文件——搬进公开仓库前必须确认授权**（技能为付费购买，需向 oiloil 确认条款；未确认前只做不落库的本地原型评审）。

## 范围（1 票）

1. **beam-graph**（点阵底纹+发光连线+中心枢纽+微胶囊节点）：移植到 why-pane 汇集演示——变量改挂 site.css 现有 token（冲突时以 site.css 为准）；与 #60 拓扑总览共用母题（点阵+连线+枢纽）与 OKLCH 派生色板。
2. **metric 卡**（数字滚轮+趋势微图+浮动百分比）：升级 hero-stats/stat-row（替代 site-motion-pack 的 count-up 或与之合并——实施时二选一，避免双实现）。
3. **terminal 卡**（窗口顶栏+语法高亮+状态灯+闪烁光标）：download 区命令卡（`winget install` / `iwr … | iex` 式）。
4. **keycaps 卡**（立体按键投影）：快捷键演示区（Alt+2 等——与 #64 ZCode 快捷键体系的官网表达协同）。
5. 适配纪律：shell.css 变量架构（宿主注入 --card-accent，oklch 派生）与 site.css token 冲突时以 site.css 为准；明暗双主题各一套；reduced-motion 降级。

## 验收

四卡在官网目标位置渲染（明暗+三皮肤）；变量全部挂 site.css token（无硬编码色）；reduced-motion 降级；**许可确认记录进 issue 评论后才允许代码入公开仓库**；owner 视觉验收。

## 协调

依赖 site-design-governance（令牌锚）；与 #60 拓扑总览（应用侧同母题）约定互引；metric 卡与 site-motion-pack count-up 去重二选一。
