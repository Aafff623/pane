Status: ready
Issue: https://github.com/Aafff623/pane/issues/86
Priority: P2
Area: Site

# 官网动效包：count-up/nav 磨砂/打字/POP/glow + 文字揭示 + 排印收口

## 范围（详见 spec.md，证据 temp/research/20261025-website-ui-deepdive.md §四）

- 票 01 即日件（半天）：count-up+nav 磨砂板+mono 打字 chip+POP 徽章+footer glow+节奏 token（零新依赖，gsap 已 vendored）。
- 票 02（1-2 天）：text-reveal.js（blur+mask 扫光/逐行滑入，leading≥1.1 防裁切）+hero 编排表+magpie 九档 weight 规范化。

## 验收

reduced-motion 直出终态；中英双语测；ego 实测 WebView2；owner 视觉验收。
