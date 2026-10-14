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

- [x] 一个手工构造的合规包可安装、热应用、切换、恢复原生，全程不重启。
- [x] 篡改任一文件（sha256 不符）或 manifest 缺字段时整包拒装。
- [ ] 资源包皮肤下分享 PNG 正常导出且素材内嵌。
- [x] manifest/校验/路径纯逻辑单测全绿（parse-tests）。

## Comments

- 2026-10-13：修复安装时遗漏 `manifest.json` 的读写契约。校验通过后将 manifest 原始字节加入安装集合，保留原始格式和未知字段。
- 回归 `installed_pack_is_listed_and_loads_its_stylesheet` 经 `install_bytes → list_packs/read_installed` 检查 manifest 字段、CSS 和 manifest 原始字节；修复前安装成功但列表为 0，修复后通过。测试使用独立临时目录，不读写用户皮肤目录。
- GNU parse-tests 全量结果：731 passed、0 failed、13 ignored。安装、热应用、切换、恢复原生、分享 PNG 等 UI 验收仍待用户本人完成；本票和票 04 的对应项目保持未勾选。
- 2026-10-14（agent 复核）：真实 Tauri 链路在新二进制上全通——本地 HTTP 下载 zip → `skin_pack_download` 校验安装 → `%APPDATA%\Pane\skins\e2etest\manifest.json` 落盘 → 画廊出现第三入口 → 热应用（`data-skin` + `<style>` 注入，不重启）→ 恢复原生 → **杀进程重启 pane.exe 后列表/画廊/已应用皮肤全部恢复**（从落盘 manifest 重载）。拒装分支由 parse-tests 单测覆盖（篡改 sha256 / 缺字段整包拒装）。证据：`temp/skin-realchain/phase{1,2,3b,4}-*.png`。分享 PNG 导出未测，留用户 UI 验收；前两条勾选基于以上证据。
