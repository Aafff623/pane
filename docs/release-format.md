# Release 发布格式规范

> 生效日期：2026-10-05 ｜ 状态：强制执行
> 本规范对标 OhMyMeme/OhMyMeme 的发布风格（分析报告见 `temp/release-style/ANALYSIS-20261005.md`，本地参考不入库）。
> 每次 `v*` tag 发布都必须按本规范执行。AGENTS.md「Validation & delivery」第 6 条为对应规则。

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

- 版本号一律进文件名：`Pane_<版本>_<os-arch>.<ext>`（如 `Pane_0.4.69_x64-setup.exe`、`Pane_0.4.69_aarch64.dmg`、`Pane_0.4.69_amd64.deb`、`Pane_0.4.69_amd64.AppImage`）。
- arch 名按发行惯例：deb 用 `amd64/arm64`，AppImage/dmg 用 `x86_64/aarch64`。
- 矩阵表里的体积取实际 artifact 大小，与资产列表一一对应。
- 更新通道资产 `latest.json`（minisign 签名）继续保留，它是 updater 的契约，不因格式规范改动。
- Windows NSIS setup.exe 是主力分发形态，矩阵表必须置于最前行；Linux rpm、Linux arm64、macOS x86_64 为补齐项，按 CI 实际产出的资产如实填表，未发布的架构写 `—`，不得虚构链接。

## 5. 强制执行方式

- `release.yml` 创建 release 时**必须使用 `--notes-file`** 传入按本规范生成的正文，**禁止裸用 `--generate-notes`**（自动正文只有 compare 链接，视为发布流程缺陷）。**已实现**：`Compose release notes` 步骤从 CHANGELOG.md 对应版本节 + artifacts 实际体积自动生成 `notes.md`，`Create GitHub release` 改用 `--notes-file notes.md`（2026-10-05 起）。
- 矩阵表的体积由 CI 从 artifact 实际大小自动计算（MB 一位小数），缺失的架构组合填 `—`，不虚构链接。
- 手动 `gh release create` 或 CI 之外的应急发布同样适用本规范；应急后须补全 CHANGELOG.md。
