# 06 — 官网 demo 与本地 1:1 同步

Type: task
Status: ready-for-agent
Blocked by: 01, 02, 05

## What to build

工坊改版与资源包制落地后，官网皮肤演示与本地保持完全一致：

- 重跑 demo 构建（build-demo.mjs），溯源闸门自动校验 demo 与源码一致（既有管线，改完本地必须重构建）。
- demo 内的皮肤工坊视图随新布局/两步按钮/真实组件预览自动更新；官网若存在手写皮肤介绍图或文案，替换为与真实渲染一致的素材。
- 随下次 push 经 site-deploy.yml 上线（push 单独授权，照旧）。

## Acceptance

- [ ] 官网 demo 皮肤工坊视图与本地工坊并排对照无可辨差异。
- [ ] 溯源闸门 CI 全绿（demo 按当前源码构建）。
- [ ] 官网无残留旧版样张截图或过时文案。
