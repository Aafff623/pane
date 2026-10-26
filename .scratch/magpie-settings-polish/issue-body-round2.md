# 第二轮扩展（2026-10-25 用户追加，spec 见 spec-round2.md）

第一轮 01-04 已实现（05 截图验收待 owner）。用户追加四项，均以 magpie UI 机制深挖为支撑（证据：temp/research/20261025-magpie-ui-mechanics.md，本地）：

## 06 全局丝滑基座

magpie 丝滑四习惯的 Pane 平移：切页 page-in 动画（display:none→block 自动重启，与现 block.hidden 切页天然兼容）、全局 --ease 缓动令牌 + hover .15s + :active scale(.96)、折叠改 grid-template-rows 0fr→1fr、prefers-reduced-motion 降级、@container 窄容器查询、pointerdown 先行选中、replaceChildren 替代 innerHTML。

## 07 侧栏二级菜单

设置入口下挂子页（对照 magpie 常规/额度/…八项的丝滑展开）：信息架构先交 owner 拍板 → grid-rows 展开动画 + chevron 旋转 + 子项 stagger + aria-expanded + localStorage 记忆 + 键盘可达。

## 08 文字大小四档 + 代码字体 + 色盲模式

对照 magpie 常规页：文字大小 100/110/125/150% 四档 pill（根字号变量 + 从 popover 自适应缩放中拆出用户偏好因子）；代码字体双下拉+预览行（复用现有 ui-font 字体枚举模式）；色盲模式与燃烧曲线 dash 图例联动。

## 09 pill 大面积推广 + thumb 真滑动

收编第一轮遗留（.st-radio-group 待 owner 拍板）+ 新设置项用 pill + thumb 真滑动升级（WAAPI transform+width + ResizeObserver 重定位，约 60 行 JS）。

验收口径与不抄清单见 `.scratch/magpie-settings-polish/spec-round2.md`。UI 验收由 owner 本人执行。
