# 06 — 重打皮肤包、集成验收与交付边界

Type: task  
Status: in progress (2026-10-26) — 1.0.2 packs built/installed, screenshots captured, fresh-profile verified. Live deploy + owner UI acceptance pending.  
Blocked by: 01, 02, 03, 04, 05  
Spec: ../spec.md  
Issue: [#57](https://github.com/Aafff623/pane/issues/57)

## Scope

- 用当时工作树重新构建三个 pack，更新 index 的 size/sha。
- 重新安装本机 pack，清 WebView2 cache。
- 跑全部自动化与 fresh-profile 市场测试。
- 生成截图与验收清单，不在 owner 验收前宣称完成。

## Acceptance

- [ ] `pnpm build` 通过。
- [ ] Node tests、site tests 全绿。
- [ ] parse-tests 全量结果如实记录。
- [ ] `check-skin-packs.mjs --live` 全绿。
- [ ] 全新 profile / 空 skins 目录能列出并下载三款皮肤。
- [ ] 4 主题 × 亮暗 × 380/450px 的关键页面截图齐全。
- [ ] 三主题转场、减少动画、分享 PNG 都有结果。
- [ ] owner 亲自完成 UI 验收后，才进入 commit/push/release 决策。

## Comments

- 2026-10-25：共享工作树已有主对话 1.0.1 候选包；执行时必须重新核对实际版本，不沿用 1.0.0 旧结论。
