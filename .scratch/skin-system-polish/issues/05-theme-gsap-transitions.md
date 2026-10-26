# 05 — 主题专属 GSAP 设置与工坊转场

Type: task  
Status: done (2026-10-26) — three timelines wired to Settings + workshop; reduced motion direct; overlay cleanup verified.  
Blocked by: 03, 04  
Spec: ../spec.md  
Issue: [#57](https://github.com/Aafff623/pane/issues/57)

## Scope

- 抽统一接口，例如 `playSkinTransition(skinId, source, { reduced, apply })`。
- Comic Pixel：22px 像素格按顺序/距离擦除。
- Signal：圆形邮章放大、抖动、落章。
- Ink：6 个不规则墨团扩散。
- 先接入设置入口和皮肤工坊；不扩到所有页面。
- `reduceAnimations` 或 `prefers-reduced-motion` 直接切页。

## Acceptance

- [ ] 三主题各有不同时间线，不是换色共用一套。
- [ ] 动画期间只执行一次 `apply()`；结束清理 overlay。
- [ ] 可重播；连续点击不会叠出多个 stage。
- [ ] 键盘 Enter/Space 与鼠标点击同路径。
- [ ] 减少动画时无覆盖层残留。
- [ ] 原型动画作为视觉参考，真实 WebView2 由 owner 目检。

## Comments

- 2026-10-26: 实施主对话回报见 `docs/project-facts.md` 的 Skin system 节；截图在本地 `temp/skin-shots/`（untracked）。未 commit/push。
