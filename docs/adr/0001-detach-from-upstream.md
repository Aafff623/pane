# ADR 0001 — 脱离上游，独立开发

- 状态：已接受（2026-09-06）
- 决策人：仓库所有者 Aafff623

## 背景

Pane 源自 OpenUsage（macOS，[robinebers/openusage](https://github.com/robinebers/openusage)）
的 Windows 复刻，此前以 ItsJazii/pane 为上游维护 fork。2026-09-05 向上游提交
3 个主题 PR（#184 账号体系 / #185 UI 打磨 / #186 dev 文档），同日被批量关闭：
零合并、零留言，仅 Devin bot 挂了自动审查；配套提案 issue #181–183 也无任何
互动。上游唯一一次实质回应是采纳功能请求 #173（标 completed）。结论：上游收
需求、拒代码、不解释。

## 决策

放弃"提 PR 回上游"路线，Pane 作为独立项目由本仓库（Aafff623/pane）直接演进：

1. 删除本地 `upstream` remote（2026-09-06）；
2. GitHub 上解除 fork network（Leave fork network，API 验证 `fork:false`）；
3. 治理文档去上游化：README / SECURITY 的安装、clone、安全报告链接指向本
   仓库；CONTEXT.md 的 git 布局改为"单远端独立项目"；AGENTS.md 删除
   "README 继承自上游"表述；ROADMAP 的对标基准明确写成 macOS 原版
   OpenUsage。
4. 功能演进的参照物改为 OpenUsage，不再是 ItsJazii/pane。

## 后果

- 正面：方向与节奏完全自主。本地 main 已大幅领先（多账号体系、14 家厂商
  额度、订阅徽章、One/New API 站点账号等均为本仓库独有成果）。
- 代价：不再自动获得 ItsJazii/pane 的新提交，上游后续功能按需自行评估。
- 保留的功能性依赖（暂不动，另行决策）：应用标识符 `com.jazii.pane`
  （决定 `%LOCALAPPDATA%` 数据目录，改动需带数据迁移）；自动更新端点
  `trypane.xyz`（长期应换成纯 GitHub Releases 或自有域名）。
- 流程：`CONTRIBUTING.md` 的 "issues first" 家规保留，但对象是本仓库自身；
  "动码前先在上游开 issue"的旧义务作废。
