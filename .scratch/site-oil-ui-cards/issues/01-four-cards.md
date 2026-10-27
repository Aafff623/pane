Status: ready
Type: task
Blocked by: site-design-governance

# 01 · 四卡移植（beam-graph/metric/terminal/keycaps）

## 做什么

1. beam-graph→why-pane（点阵+连线+枢纽；变量挂 site.css token；与 #60 拓扑同母题同色板）。
2. metric 卡→hero-stats/stat-row（与 motion-pack count-up 实施时二选一）。
3. terminal 卡→download 区命令卡（winget install 等）。
4. keycaps 卡→快捷键演示区（Alt+2）。
5. 适配：shell.css 变量架构与 site.css 冲突以 site.css 为准；明暗双主题；reduced-motion 降级。

## 验收

四卡位置渲染正确；色全走 token；reduced-motion 降级；**许可确认记录进 issue 评论前代码不入库**；owner 验收。
