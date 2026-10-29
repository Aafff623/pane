# 06 · FAQ / 下载区 / Closing CTA

Status: done 2026-10-29 | Priority: P1 | Depends: 03

## Scope

- FAQ：沿用上游 accordion（Radix）结构与「分组 + 单开」行为；内容换 Pane 真实问答（支持哪些工具、是否开源、数据是否上传、多账号、平台支持、如何开始），中英两套。
- 下载区：主 CTA Windows（`/download/windows`）、macOS Beta、Linux 计划中；保留免费口径（全功能免费 / 无订阅 / 无需注册）、winget 命令、国内镜像、版本号与平台行。
- 平台分流：迁移旧 `download-routing.js` 的能力（UA + architecture + `/api/latest` 资产匹配 + 失败回退），在新工程里以客户端组件实现，保持 `?os=&arch=` 验收钩子。
- Closing CTA：深色/浅色大块收尾（沿用上游 `ClosingCta` 版式），主按钮下载 + 次按钮演示。
- Footer 与免责声明保持事实准确（独立工具、商标归属）。

## Done when

- 所有下载入口指向正确资产；Windows 302 生效。
- 免费口径与旧站一致，无新增承诺。
