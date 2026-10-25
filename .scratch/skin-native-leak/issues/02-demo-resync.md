# 02 — 重同步 bundled demo

Status: implemented (2026-10-25) - site tests green
Type: task
Blocked by: 01

## Scope

票 01 落地后运行：

```bash
node site/scripts/build-demo.mjs
```

产物落在 `site/public/demo`（脚本内部以 `base: '/demo/'` 重新构建并清空旧 assets，`__BUILD_STAMP__` = `website`）。

- 修复后 demo 的原生模式（未选皮肤）同样不再显示「云端邮局」——demo 与应用同源同代码，无需为 demo 单独打补丁。
- 提交沿用仓库惯例 `chore(site): resync bundled demo`；**不 push**，提交与否由 owner/主对话按流程定。

## Acceptance

- [ ] `node site/scripts/build-demo.mjs` 执行成功，`site/public/demo` 更新。
- [ ] demo 站原生模式无「云端邮局」及重复时长文本（可本地 serve 后肉眼确认，或由 owner 验收时代确认）。
- 2026-10-25：build-demo 已跑，site 测试 15/15 通过。
