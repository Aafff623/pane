# README 素材去重与重拍

**What to build:** 去掉重复截图、替换观感差的，统一由 readme-shots.mjs 重出；保持中英镜像与检查器全绿。

**Blocked by:** None

**Status:** in-review（主图去重重拍完成，owner 视觉终验待做）

- [x] 两份 README 不再出现同一张图重复占用位置
- [x] 每张图都有明确说明价值（无凑数图）
- [x] check-readme.mjs 全绿；中英图集一致

## Comments

- 2026-10-14：由 spec `.scratch/release-082/spec.md` 拆分（owner 批次需求）。

- 2026-10-14：从当前源码构建demo，以浏览器能力重拍 `hero-{dark,light}.jpg`（450×640 CSS裁剪）。主图展示折叠总花费+完整额度总览，功能节保留展开视图，消除同一overview图重复引用。两份README同步，维护表同步。`node temp/scripts/check-readme.mjs` ALL CHECKS PASSED；未批量改动其余已验收图。
