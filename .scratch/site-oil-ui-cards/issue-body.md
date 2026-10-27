Status: ready
Issue: https://github.com/Aafff623/pane/issues/88
Priority: P2
Area: Site

# oil-ui 卡片移植：beam-graph / metric / terminal / keycaps

## 定位

oil-ui-pro 四款原型适配进官网。**前置：assets/cards 无 LICENSE——进公开仓库前必须确认授权**（未确认前只做本地原型评审）。

## 范围（详见 spec.md，证据 temp/research/20261025-website-ui-deepdive.md §三）

1. beam-graph→why-pane 汇集演示（挂 site.css token，与 #60 拓扑同母题）。
2. metric 卡→hero-stats/stat-row（与 motion-pack count-up 二选一）。
3. terminal 卡→download 命令卡。
4. keycaps 卡→快捷键演示区。
5. 适配纪律：token 冲突以 site.css 为准；明暗双主题；reduced-motion 降级。

## 验收

四卡渲染（明暗+三皮肤）；无硬编码色；许可确认记录后才入公开仓库；owner 验收。
