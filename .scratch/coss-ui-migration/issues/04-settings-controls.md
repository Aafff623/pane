# 04 设置页常用控件

Status: ready-for-human

**Blocked by:** 01, 02 的用户真机验收

按页面接管设置按钮、输入、普通表单与选择控件。每页核对搜索跳转、编辑、保存失败和重置。有下拉定位的任务复用 03 的已验证做法。


## Comments

2026-11-27。设置侧这一轮做的是共用控件而不是逐个换皮：分段选择补了 `radiogroup`
语义、roving tabindex 和方向键/Home/End（5 处调用点带上了 `aria-label`），
表单校验结果通过 `aria-invalid`/`aria-describedby` 关联到输入框，保存/复制/应用
失败的 `settingsToast` 变成 live region（错误用 `assertive`）。凭据库的设置向导和
重置/解锁弹窗改由外壳托管，写库期间禁止关闭。构建与回归通过。逐页的搜索跳转、
编辑与恢复默认仍需在 Windows 上逐页验收。
