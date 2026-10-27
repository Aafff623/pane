Status: ready
Issue: https://github.com/Aafff623/pane/issues/79
Priority: P1
Area: Site

# 官网内容口径整改：平台 badge / 版本注入 / provider 计数 / winget·CN 入口 / 免费表达

## 问题

macOS/Linux 口径三处矛盾（下载区死按钮 vs FAQ vs release.yml）；版本号 5 处硬编码；provider 数 78 vs 77 漂移；winget/CN 镜像无人类入口；「免费开源」无表达。

## 方案（详见 spec.md，证据 temp/research/20261025-website-reference-sweep.md）

- 票 01：平台卡 badge 三态（Available/Beta/Planned，**不给死链接**，FAQ 口径对齐）+「全功能免费」对照卡（无 Pro/无订阅/无账号+MIT）。
- 票 02：版本号构建期注入（package.json 单一源）+provider 计数以 Rust catalog 为准+winget/CN 镜像人类入口。

## 验收

测试绿；三语无矛盾；版本号只碰 package.json；owner 文案验收。
