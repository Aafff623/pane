Status: ready
Issue: https://github.com/Aafff623/pane/issues/81
Priority: P2
Area: Site

# 官网 CI 自动化：skin gate 进 CI + demo 漂移 + fonts 清理 + 部署 smoke

## 问题

check-skin-packs.mjs 自称 release gate 但无 CI 跑它；demo 重建 11 次纯手工；fonts 孤儿+空 index；无部署 smoke。

## 方案（详见 spec.md，证据 temp/research/20261025-website-reference-sweep.md §二）

1. ci.yml site job 加 check-skin-packs（一行）。
2. demo 漂移：CI 检出失败时给重建命令提示（或自动 PR）。
3. fonts 孤儿清理+index 一致性（先删后建）。
4. site-deploy.yml 加部署后 smoke（/download/windows 302、/skins/index.json 200、/ 200）。

## 验收

skin gate 红/绿可验证；fonts 干净；smoke 三步过；owner 验收 CI。
