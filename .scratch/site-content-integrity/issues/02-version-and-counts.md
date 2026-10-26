Status: ready
Type: task
Blocked by: —

# 02 · 版本号注入 + provider 计数治理 + winget/CN 入口

## 做什么

1. 版本号构建期从 package.json 注入 index.html/i18n.js（消灭 5 处硬编码）；site.test.mjs 从「兜底比对」改「注入存在性」断言。
2. provider 计数单一事实源：以 Rust catalog（77）为准同步 FE 镜像；或加 Rust 侧计数测试（二选一，实施时定）。
3. 下载区加 winget 安装说明（命令+链接）+CN 镜像人类入口（Gitee releases 链接+国内加速文案）。

## 验收

改版本号只碰 package.json；计数测试绿；winget/CN 入口可点通；owner 验收。
