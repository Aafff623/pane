Status: ready
Issue: https://github.com/Aafff623/pane/issues/90
Priority: P2
Area: Site

# web/ 实验站清理（B 路：提取 demo 窗外壳 + 清理 480MB 尸体）

## 背景

web/ 源码 10-23 被删（无备份、从未进 git），只剩构建尸体约 480MB。**执行前必须 owner 确认：删除是有意还是事故**——有意走本票 B 路；事故先讨论抢救（relay 副本+out 重建 ≈2 天）。

## 范围（B 路，详见 spec.md）

1. 从 web/out/index.html 提取 demo 产品窗外壳（mono 标题栏+玻璃卡纯 CSS）→ 增强 site/public/demo-frame.css。
2. 清理 web/ 剩余（node_modules+.next+out 约 480MB，**需 owner 明确授权**）。
3. worker /en 分支记录结论（web/ 死后无冲突）。

## 验收

demo-frame 增强落地（owner 验收）；480MB 清理完成（授权后）；relay 副本保留；结论记录在案。
