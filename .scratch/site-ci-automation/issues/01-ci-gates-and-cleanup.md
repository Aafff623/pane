Status: ready
Type: task
Blocked by: —

# 01 · skin gate + demo 漂移 + fonts 清理 + 部署 smoke

## 做什么

1. ci.yml site job 加 `node site/scripts/check-skin-packs.mjs`（先本地跑通再进 CI）。
2. demo 漂移：CI 检出哈希失败时输出重建命令提示（build-demo.mjs 用法）；自动 PR 为可选增强。
3. fonts：删 public/fonts 孤儿 zip+127B 占位 woff2；让 index.json 反映真实状态（删或重建，二选一）。
4. site-deploy.yml 部署后 smoke：curl /download/windows（期望 302）、/skins/index.json（200）、/（200）。

## 验收

四项分别可验证（skin gate 红/绿、demo 提示、fonts 干净、smoke 过）；owner 验收 CI 配置。
