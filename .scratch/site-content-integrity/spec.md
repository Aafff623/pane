# Spec: 官网内容口径整改

**Status:** ready · **Priority:** P1 · **Area:** Site
**Issue:** https://github.com/Aafff623/pane/issues/79
**Evidence:** temp/research/20261025-website-reference-sweep.md §一（审计发现 1/5/8）+ §二 codexbar-site 平台卡模式

## 问题

1. **macOS/Linux 三处口径矛盾**（P1）：下载区「未上线」死按钮 vs FAQ「Releases 提供」vs release.yml 三平台构建（index.html:320 vs :311）。codexbar-site 的正解：平台卡 badge 三态（ok/info/soon）+「状态先行、不给死链接」。
2. **版本号 5 处硬编码**（index.html/i18n.js 等），测试兜底模式。
3. **provider 数漂移**：FE 镜像 78 vs Rust catalog 77（多 mcode），测试只对 FE 计数。
4. **winget/CN 镜像无人类入口**：winget 有 CI 发布但官网无说明；CN 镜像只惠及 app 更新（/update/latest.json），人类访客无 Gitee 链接。
5. **「免费开源」无表达卡**：FAQ 有问答需查重；pricing 区可反向借用 codexbar drafts 的免费/Pro checklist 结构。

## 范围（2 票）

**票 01 平台口径与免费表达**：
1. 平台卡改 badge 三态：Windows=Available（主按钮）/macOS=Early access/Beta（链接到 GitHub Releases，**不再死按钮**）/Linux=Planned（无链接）；消除下载区 vs FAQ 矛盾，i18n 三语文案同步。
2. pricing/FAQ 区加「全功能免费」对照卡：无 Pro/无订阅/无账号+MIT+源码链接（查重 FAQ 现有问答）。

**票 02 版本号与计数治理**：
1. 版本号构建期注入（从 package.json → index.html/i18n.js），消灭硬编码；测试从「兜底比對」改「注入后存在性」。
2. provider 计数单一事实源：Rust catalog 为准（77），FE 镜像同步；或加 Rust 对齐测试。
3. 下载区加 winget 安装说明（`winget install` 命令+链接）+CN 镜像人类入口（Gitee releases 链接，文案注明国内加速）。

## 验收

`node --test site/*.test.mjs` 绿；三语无矛盾口径；版本号改动只碰 package.json；owner 文案验收（尤其免费卡措辞）。

## 协调

与 site-download-conversion（UA 分流+plats 矩阵）同改下载区——本票先做静态口径，分流票后做动态；错峰提交。
