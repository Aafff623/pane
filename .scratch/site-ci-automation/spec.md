# Spec: 官网 CI 自动化与清理

**Status:** implemented (2026-10-26) — skin gate + demo drift + deploy smoke in CI; fonts directory clean with the intentional empty pack index.
**Issue:** https://github.com/Aafff623/pane/issues/81
**Evidence:** temp/research/20261025-website-reference-sweep.md §一（审计发现 2/3/4）+ §二自动化清单

## 问题

1. **check-skin-packs.mjs 自称 release gate 但任何 CI 都不跑**（ci.yml:80 只 node --test）——发布门禁只活在人的记忆里。
2. **demo 重建纯手工**：11 次 rebuild/resync 提交，最近一周 5 次；CI 能检测哈希漂移不能自动修。
3. **fonts 孤儿**：public/fonts/ 2 个孤儿 zip+index.json 空列表+127 字节占位 woff2（build-font-packs.py FONTS=[] 空转）。
4. 无部署后 smoke。

## 范围（1 票，4 子项）

1. **skin gate 进 CI**：ci.yml site job 加一行 `node site/scripts/check-skin-packs.mjs`；`--live` 版可挂部署后 smoke（可选）。
2. **demo 漂移自动 PR**：ci.yml 检出哈希失败时跑 build-demo.mjs 并自动开 PR（或 fail+提示已足够，当前已检测——先做 fail 增强：提示信息含重建命令）。
3. **fonts 清理+一致性**：删孤儿 zip 与占位 woff2（或让 build-font-packs.py 产出真实 index）；仿 check-skin-packs 写 fonts 校验（二选一，推荐先删后建）。
4. **部署后 smoke**：site-deploy.yml 加 curl `/download/windows` 期望 302、`/skins/index.json` 期望 200、`/` 期望 200。

## 验收

改 skins index 不放行的包 → CI 红；demo 漂移 → CI 给明确提示（或自动 PR）；fonts 目录干净；部署后 smoke 三步过；owner 验收 CI 配置。

## 协调

与 site-basic-contracts（404/robots 断言同批进 site.test.mjs，但本票改 ci.yml/site-deploy.yml 不冲突）。
