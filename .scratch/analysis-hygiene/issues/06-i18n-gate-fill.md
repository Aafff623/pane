# 06: i18n 三字典键集 CI 门禁 + 补齐 44 键

**What to build:** 俄语用户在 One/New API 设置区、皮肤预览、页脚提示看到俄语而不是整片英文回退；三字典键集比对进 CI，未来任何语言的漏翻从产品缺陷变成 CI 红一条。

**Blocked by:** None (can start immediately)

**Status:** implemented（2026-10-15，见对应 commit）

- [ ] ru 字典补齐 43 个缺失键（皮肤 5、页脚 onenewapi 9、设置 onenewapi 28、metric.expiry 1——按 en 语义译俄语）
- [ ] en 与 zh 字典各补 1 个 ru 独有键（stepfunPlan 凭据提示），三字典完全对齐
- [ ] 新增零依赖键集比对测试（node --test，照 site 测试模式），接进 CI
- [ ] 比对测试本地跑过；tsc + vite build 过
