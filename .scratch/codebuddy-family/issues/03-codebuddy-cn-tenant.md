# 03 — 新增 CodeBuddy CN 租户（全链路）

**What to build:** 让中国站 `www.codebuddy.cn` 成为家族里第一个**新租户**，走通全链路：设置里能扫码登录、卡片出现、额度按 02 的卡面显示。

这是对「租户维度」的端到端验证——目录四层、图标、i18n、登录流程、快照一次打通，且与国际站的差异最小（同 platform、同接口形状），风险最低。

**可搜索是硬要求**：自定义面板搜「CodeBuddy CN」必须搜到；设置界面的图标列表里要能看到它。卡片标题必须能与国际站区分。auth center 里出现它的账号行（可改名、可删除）。

**Blocked by:** 02

**Status:** ready-for-agent

- [ ] 设置里能登录 CodeBuddy CN，登录后出现独立卡片
- [ ] 卡面按 02 口径显示额度；标题能与 CodeBuddy（国际站）区分
- [ ] 自定义面板搜「CodeBuddy CN」能搜到；设置图标列表里有它
- [ ] auth center 出现该租户的账号行，改名 / 删除可用
- [ ] token 过期自动刷新；失败给「重新登录」的可读提示
- [ ] 目录四层一致（Rust catalog / 前端镜像 / 图标 / i18n），`catalog_consistency` 门禁绿

## Comments

- 2026-10-11 出自 `.scratch/codebuddy-family/spec.md`（决策 D1 / D2 / D8 / D9）。
- 主机 `https://www.codebuddy.cn`、platform `ide`，前缀与计费接口形状同国际站。
- 2026-10-12 **已实现（代码层，待 UI 验收）**：家族表加 `codebuddycn` 行；Rust catalog + 前端镜像各一条（`iconKey: "codebuddy"`，与国际站共用一个品牌 mark）；`LOGIN_FAMILIES` / `AUTH_FAMILIES` 纳入；`codebuddycn_login_start/poll/cancel` 三条命令注册；前端 `BROWSER_LOGIN_PROVIDERS` 纳入。
  - 面板搜索与设置图标列表**自动生效**：二者都由 `providerCatalog` 派生（`ALL_PROVIDERS`），无需额外索引——这正是选「目录层三条目」而非「单条目 + 租户别名」的原因。
  - 未做：实机扫码登录验证（需用户操作）。
