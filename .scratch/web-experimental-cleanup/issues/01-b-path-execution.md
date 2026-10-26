Status: ready
Type: task
Blocked by: owner 决策（10-23 删除是否有意）

# 01 · B 路执行：提取 demo 窗外壳 + 清理 web/ 尸体

## 做什么

1. 从 web/out/index.html 提取 demo 产品窗外壳（mono 标题栏+状态+玻璃卡）纯 CSS → site/public/demo-frame.css 增强（site/ 已有 1KB 基础）。
2. owner 授权后清理 web/（node_modules 455MB+.next+out+logs）；保留 temp/template-analysis/relay 与 temp/sites/。
3. issue 评论记录：删除性质结论、B 路执行清单、relay 副本位置。

## 验收

demo-frame 增强（owner 视觉验收）；清理完成；git status 无 web/ 残留记录；结论在案。
