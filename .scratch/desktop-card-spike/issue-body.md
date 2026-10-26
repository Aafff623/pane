Status: ready
Issue: https://github.com/Aafff623/pane/issues/77
Priority: P3
Area: UI

# 桌面常驻卡片悬浮档预研（8h spike，owner 决策项）

## 定位

预研票非实现票。QuotaBar 桌面卡片形态的 Pane 化可行性核验：Tauri 2.11.5 能力件全有（transparent/skip_taskbar/focusable(false)=WS_EX_NOACTIVATE/always_on_top/分数坐标换算均有先例——lib.rs:6061/6268/:6088-6094、webview_window.rs:862-865）。「桌面档」WorkerW 维持否决。

## 范围（详见 spec.md，证据 temp/research/20261025-final-sweep-7repos.md §1-8-D）

1. 新窗口 card（transparent+skip_taskbar+免焦点+always_on_top）+分数坐标存取（复用 config.pinned 结构）。
2. IS_CARD_FORM 渲染子集（renderCard/renderMetric 子集+skins）；数据 cached_usage+低频轮询。
3. 验证三域：WebView2 常驻内存、Win11 合成视觉、焦点行为与 popover 并存。

## 验收

spike 交付「能亮/能定位/能关」+三域数据+owner 决策记录（进实现票 24-40h 或明确不做）。

## 协调

依赖 capture-exclusion-privacy（card 窗口共享排除）——先隐私票。
