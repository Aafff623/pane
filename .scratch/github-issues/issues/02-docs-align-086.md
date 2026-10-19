# 02: 文档对齐 v0.8.6（GitHub #4）

**What to build:** docs/project-facts.md 版本与关键工程事实更新到 0.8.6；SECURITY.md 与 docs/privacy.md 的凭据存储、更新链路描述与代码一致；与 README、AGENTS.md 交叉核对无新增重复事实源。

**Blocked by:** 01（软阻塞——SECURITY.md 需描述 #3 修复后的最终保存顺序）

**Status:** implemented（2026-10-18，c1cd3b2）

**来源**：<https://github.com/Aafff623/pane/issues/4>

## 验收标准（issue 原文照抄）

- [x] 更新 `docs/project-facts.md` 的版本与关键工程事实
- [x] 统一 `SECURITY.md` 和 `docs/privacy.md` 的凭据存储描述
- [x] 更新检查、镜像与回退机制与实际代码一致
- [x] 与 README、AGENTS.md 交叉核对，避免新增重复的事实来源

## 已知事实（已核实）

- `docs/project-facts.md:15,31` 仍写 "Last reconciled: 2026-10-11 (Pane 0.7.0)" / "Version 0.7.0"。
- `SECURITY.md:28-37` 称 "OS credential vault owns every secret"；`:88-90` 却称登录账号 "stored ... as plaintext JSON"——与 vault-first 实现矛盾。
- `docs/privacy.md:18` 更新检查只列 GitHub Releases latest.json，未提 CN 镜像 Worker 边缘入口与回退。

原则（issue 原文）：修正已有文档，不额外引入新的治理体系。
