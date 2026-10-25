# 03 — 两档分隔线 + native 微调（危险按钮 hover 红、侧栏去位移）

Status: implemented (2026-10-25) - automated green; owner visual acceptance pending
Type: task
Blocked by: —

## Scope

- 两档线：`:root`（styles.css:60-92）与 `:root[data-theme="light"]`（96-117）各加 `--row-line`（dark：`color-mix(in srgb, var(--border) 55%, var(--card))`；light：`color-mix(in srgb, var(--border) 60%, var(--card))`）。四处规则统一引用：`.set-row`（10244）、`.set-section__card > .setting-row`（10319，特异性 (0,2,0) 压过 .set-row）、`.backup-title`（10305）；两条 `:last-child` 规则保持 0。
- 危险按钮 native：`#reset-all-settings`（7646-7663）静态回落中性（`border-color:var(--border); color:var(--secondary-foreground); background:transparent`），hover 才 `color:var(--destructive)` + 12% 红底（值搬自 7657-7658/7661-7662）；`.mini-btn.danger`（4384-4387）同理。对齐 Magpie `app.css:1986` 的 hover 才红。
- 侧栏 hover：删 `transform: translateX(2px)`（10402），transition 去掉 transform（10384）。已核实三个包 hover 均不重发 transform，此改动 4 皮肤 × 亮暗全有效。

## Acceptance

- [ ] 行线比卡边框浅一档且肉眼可辨；快捷键行/kv-add-row（双类行）与 backup-title 一起换，无漏网行。
- [ ] 三个包的两档线保持自身实现不被 base 改动破坏；native 补课效果需 10-24 Signal 截图之外的新 native 截图核对（票 05 补）。
- [ ] reset 按钮 native 静态中性、hover 才红；`.mini-btn.danger`（密钥删除等）同预期。
- [ ] 侧栏 hover 无位移；键盘 focus-visible 不退化。
- [ ] `pnpm build` 与相关 Node 测试通过。
- [ ] 不提交、不推送。
- 2026-10-25：已实现。--row-line 两档（dark 55% / light 60% mix）落入 :root 与 light 块；.set-row、.set-section__card > .setting-row、.backup-title 三处统一引用，:last-child 保持 0。#reset-all-settings 与 .mini-btn.danger 改静态中性、hover 才红；侧栏 hover 去 translateX 且 transition 摘除 transform。皮肤包未动（见票 04）。
