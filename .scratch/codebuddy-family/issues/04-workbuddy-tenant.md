# 04 — 新增 WorkBuddy 租户

**What to build:** 把用户实际要的 **WorkBuddy** 接进来——主机 `https://copilot.tencent.com`、platform `workbuddy`，与前两个租户的差异**仅此两点**。登录、卡片、额度口径与 03 完全一致。

**可搜索是硬要求**（这正是用户最初的抱怨）：自定义面板搜「WorkBuddy」必须搜到；设置图标列表里要能看到它，并且用 **WorkBuddy 自己的图标**，不借用 CodeBuddy 的。

**Blocked by:** 03

**Status:** done（落地 main；用户已实测扫码落库）

- [ ] 设置里能登录 WorkBuddy，登录后出现独立卡片
- [ ] 卡面显示 credits 计量条 + 套餐名 + 重置时间
- [ ] 自定义面板搜「WorkBuddy」能搜到；设置图标列表里有 WorkBuddy 自己的图标
- [ ] 卡片标题显示 WorkBuddy，不与 CodeBuddy 混
- [ ] 目录四层一致，`catalog_consistency` 门禁绿
- [ ] 免费档实机核对读数（需用户登录后确认）

## Comments

- 2026-10-11 出自 `.scratch/codebuddy-family/spec.md`（决策 D1 / D2 / D8）。
- 图标素材本机已有：`temp/icon-pack/codexbar-full/workbuddy.svg`、`temp/icon-pack/magpie-full/workbuddy-color.svg`。
- 注意计费网关对额度接口做 **UA 存在性校验**，缺 UA 返 403 / code 10085——请求必须带浏览器 UA。
- 2026-10-12 **已实现（代码层，待 UI 验收）**：家族表加 `workbuddy` 行（`https://copilot.tencent.com` + platform `workbuddy`）；Rust catalog + 前端镜像各一条；`LOGIN_FAMILIES` / `AUTH_FAMILIES` 纳入；`workbuddy_login_start/poll/cancel` 三条命令注册；前端 `BROWSER_LOGIN_PROVIDERS` 纳入。
  - 图标取 `codexbar-full/workbuddy.svg`（196 B、单色 `currentColor`、无内部 id）落为 `src/assets/providers/workbuddy.svg` 并在 `providerVisuals.ts` 注册。**未取** magpie 的彩色版：它带 `gradient_0`/`clip_0` 等通用 id，多图标同页注入时有 id 冲突风险。
  - 面板搜「WorkBuddy」与设置图标列表**自动生效**（目录派生）。
  - 未做：免费档实机读数核对（需用户登录）。
