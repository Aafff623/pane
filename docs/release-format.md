# Release 发布格式规范

> 生效日期：2026-10-05 ｜ 状态：强制执行
> 本规范对标 OhMyMeme/OhMyMeme 的发布风格（分析报告见 `temp/release-style/ANALYSIS-20261005.md`，本地参考不入库）。
> 每次 `v*` tag 发布都必须按本规范执行。AGENTS.md「Validation & delivery」第 7 条为对应规则。

## 0. 发布前置门禁

发布只能发生在本地需求实现完成、相关构建与测试通过，并且用户本人已经
完成 UI 验收且在当前对话中明确授权之后。没有这两个条件，不得创建 release、
tag、commit 或 push；本地端口可用、截图、构建成功和 Agent 自检都不能代替
用户验收。

前端（`src/`）有改动的版本，发布前必须重跑 `node site/scripts/build-demo.mjs`
并让 `node --test site/site.test.mjs` 全绿——官网交互演示是真实前端的构建产物，
provenance 哈希失配即表示演示区已过期（2026-10 曾发生 src 先行合入、演示区
落后两个提交的漂移，此后该闸门同时挂在 ci.yml 的 site job 上）。

## 1. 发布正文结构（固定骨架）

```markdown
# Pane v<版本> — <一句话主题：3~4 个关键词，用 / 分隔>

| 系统 \ 架构 | x86_64 | arm64 / aarch64 |
| --- | --- | --- |
| Windows | [exe](下载直链) (体积 MB) | — |
| macOS | [dmg](下载直链) (体积 MB) | [dmg](下载直链) (体积 MB) |
| Linux | [deb](下载直链) (体积 MB)<br>[AppImage](下载直链) (体积 MB) | [deb](下载直链) (体积 MB) |

## 新增
## 变更
## 修复
```

- 矩阵表**置顶**，行 = 系统、列 = 架构、单元格 = `[格式](直链) (体积)`；不支持的组合写 `—`；一格多包用 `<br>` 堆叠。
- 表格让 release 页折叠摘要和分享卡片自动携带下载信息。
- 三个 changelog 分区固定顺序：`## 新增` → `## 变更` → `## 修复`；某区为空则整节省略。

## 2. 措辞规范

1. 全简体中文叙述；技术字面量（配置键、API、文件名、命令参数）保留英文并用反引号。
2. 每条 = `**粗体条目名**：用户效果；技术细节`（早期风格用 `—` em-dash 亦可，同一版本内保持一致）。
3. 数字精确到可复现（阈值、体积、时长、版本号要求），不写"若干""大幅"。
4. 边界条件单独说清：默认开/关、旧版本兼容、降级路径、开关变更时的行为。
5. 修复条目讲根因和后果，不抄 commit message。
6. 每条 1~2 行分号短句，条目名本身可扫读；条目按用户可感知的模块排列，不按代码目录顺序。
7. Pane 已脱离上游、无 issue 可引；有对应 PR 时可注明，否则省略。

## 3. 内容来源（唯一事实源）

- **`CHANGELOG.md` 是唯一内容源**：每个版本发布前，该版本的 CHANGELOG 节必须收口（Unreleased 内容归位、条目按本规范第 2 节重写），不得出现"tag 已发但 CHANGELOG 没写"的状态。
- Release 正文 = 版本主题句 + 下载矩阵表 + CHANGELOG 对应版本节的全文，全部由 `release.yml` 的 "Compose release notes" 步骤自动拼装。
- **主题句写法**：写在 CHANGELOG 版本节的第一段纯文本行（列表项之前，3~4 个关键词用 ` / ` 分隔）；纯列表节则发布标题只有版本号，不强行造主题句。
- **CI 硬闸门**：CHANGELOG 缺少该版本节或节为空时，release.yml 直接失败拒绝发布——这是"先收口 changelog 再打 tag"的强制保证。
- CHANGELOG.md 按版本分节的现有结构保留；新条目在开发过程中随时追加到对应版本节。

## 4. 资产命名与矩阵

- 版本号一律进文件名：`Pane-Aafff623_<版本>_<os-arch>.<ext>`（如 `Pane-Aafff623_0.4.69_x64-setup.exe`、`Pane-Aafff623_0.4.69_aarch64.dmg`、`Pane-Aafff623_0.4.69_amd64.deb`、`Pane-Aafff623_0.4.69_amd64.AppImage`）。
- arch 名按发行惯例：deb 用 `amd64/arm64`，AppImage/dmg 用 `x86_64/aarch64`。
- 矩阵表里的体积取实际 artifact 大小，与资产列表一一对应。
- 更新通道资产 `latest.json`（minisign 签名）继续保留，它是 updater 的契约，不因格式规范改动。
- Windows NSIS setup.exe 是主力分发形态，矩阵表必须置于最前行；Linux rpm、Linux arm64、macOS x86_64 为补齐项，按 CI 实际产出的资产如实填表，未发布的架构写 `—`，不得虚构链接。

## 5. 强制执行方式

- `release.yml` 创建 release 时**必须使用 `--notes-file`** 传入按本规范生成的正文，**禁止裸用 `--generate-notes`**（自动正文只有 compare 链接，视为发布流程缺陷）。**已实现**：`Compose release notes` 步骤从 CHANGELOG.md 对应版本节 + artifacts 实际体积自动生成 `notes.md`，`Create GitHub release` 改用 `--notes-file notes.md`（2026-10-05 起）。
- 矩阵表的体积由 CI 从 artifact 实际大小自动计算（MB 一位小数），缺失的架构组合填 `—`，不虚构链接。
- 手动 `gh release create` 或 CI 之外的应急发布同样适用本规范；应急后须补全 CHANGELOG.md。

## 6. 发布模式：正式版 / 预览版（2026-10-15 起）

两种模式**只由 tag 决定**，不需要额外参数：

| 模式 | tag 形式 | 效果 |
|---|---|---|
| **正式版** | `v0.8.4` | Release 成为 GitHub 的 **Latest**——应用内自动更新与官网下载按钮都指向它；winget 与 Gitee CN 镜像照常 |
| **预览版** | `v0.9.0-rc.1`（版本段含 `-`） | 同样构建三平台安装包，但 Release 标为 **pre-release**：GitHub 的 `/releases/latest` 跳过它 → 自动更新、官网下载、winget、CN 镜像**全部不涉及**；把 Release 链接直接发给测试者即可 |

判定规则：tag 去掉 `v` 后含 `-` 即预览（语义化版本里 `-` 就是预发布标记）。`release.yml` 据此加 `--prerelease`，并跳过 winget 派发与 Gitee 镜像作业；`winget.yml` 自身也有 `release.prerelease` 守卫，手动翻转预发布标记也不会误发到 winget。

**版本号策略（推荐 tag-only）**：预览阶段三处版本文件（`tauri.conf.json` / `package.json` / `Cargo.toml`）**保持数字版**（如 `0.9.0`），预览标识只放 tag——零风险、无需额外的 CHANGELOG 分节、正式版发布时版本文件无缝接续。此时 Release 标题是 `Pane 0.9.0-rc.1`（取自 tag），安装包文件名用数字版（`Pane-Aafff623_0.9.0_x64-setup.exe`），应用内显示 `0.9.0`。

若希望应用内也显示 `-rc.1`（测试者一眼可辨、且正式版发布后 RC 用户会收到升级提示），可把三处版本写成带后缀的 `0.9.0-rc.1`：此时 CHANGELOG 必须存在同名分节 `## 0.9.0-rc.1`（否则 CI 按第 3 节硬闸门拒绝发布），且**首次这类构建要在 CI 里盯一眼**——Tauri 的 Windows NSIS 打包器对预发布版本号的接受度未在本项目实测过。

无论哪种策略，**CHANGELOG 必须有对应版本分节**（tag-only 策略下即数字版分节：`v0.9.0-rc.1` 对应 `## 0.9.0`）——第 3 节的硬闸门对预览版同样生效，缺节时 CI 直接拒绝发布。

**命令示例**：

```bash
# 正式版
git tag -a v0.8.4 -F notes.md && git push origin v0.8.4
# 预览版（用户端零影响）
git tag -a v0.9.0-rc.1 -F notes.md && git push origin v0.9.0-rc.1
# 只想留个版本记录、不发 Release：用非 v 开头的 tag（release.yml 只监听 v*）
git tag -a pre-cleanup-1 && git push origin pre-cleanup-1
```

注意：预发布的资产名与正式版同号时（如 RC 与正式版都是 `0.9.0`）文件名相同，但分属不同 Release，互不覆盖；预发布 Release 里的 `latest.json` 不会被任何自动通道读取。

**失败与补救**：预览与正式共用同一条流水线，任一步失败（最常见的是 CHANGELOG 缺该版本分节、或 tag 与 `tauri.conf.json` 版本不匹配）都会停在「tag 已推、Release 未建」。补救：修好内容后在 Actions 里对该 run 点 **Re-run**，或删掉 tag 重推（`git push origin :refs/tags/vX` 后重打）；构建产物本身保留 90 天，也可手动 `gh release create <tag> --prerelease --notes-file notes.md <files>` 应急。版本匹配规则：tag 必须等于 `v<tauri.conf.json 版本>`，或在该版本后追加 `-` 后缀（如 `v0.8.4-rc.1` 配版本 `0.8.4`）。
