# Spec: 桌面常驻卡片悬浮档预研（spike）

**Status:** ready · **Priority:** P3 · **Area:** UI
**Issue:** https://github.com/Aafff623/pane/issues/77
**Evidence:** temp/research/20261025-final-sweep-7repos.md §1-8-D（可行性逐项核验+范围建议）

## 定位

**这是预研票（owner 决策项），不是实现票。** QuotaBar 的桌面常驻卡片形态的 Pane 化可行性核验+1 天 spike，产出结论后再定是否投入实现（预估 24-40h）。

## 可行性结论（Tauri 2.11.5 能力件全有）

- 窗口能力先例：`transparent(true)`（panel 在用，lib.rs:6061-6064）、`skip_taskbar(true)`（main 在用）、`focusable(false)`（webview_window.rs:862-865，Windows 映射 WS_EX_NOACTIVATE）、`always_on_top(true)`（:6268 在用）、分数坐标换算（:6088-6094 monitor.work_area() 现成）。
- 前端：WINDOW_LABEL（main.ts:52-58）加 IS_CARD_FORM 分支；渲染复用 renderCard(:3462)/renderMetric(:2666)/renderGroupgedCards(:12710) 子集+skins。
- 数据：cached_usage+listen(usage-updated)+另起低频轮询（常驻窗口不能靠 popover 触发刷新）。
- 与 popover/panel **并存非模式切换**；配置复用 config.pinned（main.ts:542）{provider,label} 结构。
- 「桌面档」（WorkerW parenting：SetParent 到 Explorer WorkerW）**维持否决**（动态壁纸/UWP 壳不可靠+WebView2 输入命中问题，alwaysOnTop 已满足常驻诉求）。

## 范围（1 票，8h spike）

1. 新窗口 card（transparent+skip_taskbar+focusable false+always_on_top）；分数坐标存取（monitor.work_area 换算）。
2. IS_CARD_FORM 渲染子集（单卡/分组卡二选一，spike 内定）；数据链路（cached_usage+低频轮询）。
3. 验证点：WebView2 常驻内存占用、Win11 合成视觉效果（透明+圆角+阴影）、焦点行为（不抢焦点）、与 popover 并存的快捷键冲突。
4. 产出：可行性报告（内存/视觉/交互三域结论）+ 实现票范围建议（或「不做」结论）。

## 验收

spike 交付：能亮/能定位/能关；三域验证数据；owner 决策记录（进 spec 或 issue 评论）。

## 协调

**依赖 capture-exclusion-privacy 票**（card 窗口开启共享时同样要 set_capture_excluded）——先隐私票后本票。
