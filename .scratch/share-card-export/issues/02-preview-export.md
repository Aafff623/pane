# 02 预览、复制与保存 PNG

**Status:** ready-for-agent  
**Blocked by:** 01  
**关联:** #106，`../spec.md`，`.scratch/release-082/issues/04-copy-card-image.md`

## What to build

从总花费入口支持范围与风格选择、预览、复制和保存；复用 `copyElementSnapshot` 的样式、资源、测量及 2x 渲染流程，按需拆出共享 PNG 结果。复制仍走原生剪贴板，provider 卡即时复制不变。

## Acceptance criteria

- [ ] 五种范围切换后，预览、复制和保存引用同一份报告快照；关闭、Esc、焦点及失败反馈可用。
- [ ] 保存生成 PNG；复制失败可选择保存回退，不报告虚假成功。
- [ ] 文件自包含必要字体和图片；检查输出无敏感数据，长内容不裁切。
- [ ] provider 卡复制回归通过；适当测试并执行 `pnpm build`。

## Comments

- 2026-10-27：当前渲染函数直接调用 `copy_share_image`，保存能力和跨平台路径需实施时核实。
