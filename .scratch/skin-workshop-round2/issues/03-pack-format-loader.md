# 03 — 皮肤资源包格式与热加载器

Type: task
Status: done（见对应 commit，2026-10-12）
Blocked by: None

## What to build

皮肤=资源包的本地基座（与分发渠道无关，可先行）：

- 包格式：zip；`manifest.json`（id、名称、版本、minAppVersion、入口 CSS、资产清单、逐文件 sha256）+ 资产文件。
- 安装位置：`%APPDATA%\Pane\skins\<id>\`；安装 = 解压 + 逐文件校验 + manifest 合法性检查，任何一项不过整包拒装不留残骸。
- 热加载：CSS 动态注入、资产经 Tauri 资产协议解析、字体经 @font-face 指向本地资产；应用 / 切换 / 恢复原生全程不重启。皮肤命名空间沿用 `data-skin` 机制延伸到动态包。
- 分享 PNG：`embedSkinImages` 适配包资产（本地文件 → data URL）；安全边界不变——只嵌本机已装包资产，绝不抓远程、不读账号数据。
- 安全校验底线：只接受官方源索引列出的包（票 04 接线）；manifest 不合法整包拒装。
- 纯逻辑（manifest 解析、校验、路径解析、索引解析）进 parse-tests seam。

## Acceptance

- [ ] 一个手工构造的合规包可安装、热应用、切换、恢复原生，全程不重启。
- [ ] 篡改任一文件（sha256 不符）或 manifest 缺字段时整包拒装。
- [ ] 资源包皮肤下分享 PNG 正常导出且素材内嵌。
- [ ] manifest/校验/路径纯逻辑单测全绿（parse-tests）。
