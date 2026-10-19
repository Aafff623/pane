# 01 — 弹窗置顶/锁定/位置记忆 + 回到上次停留的界面

**What to build:** 弹窗（popover）自己的窗口 chrome：总花费行加置顶与锁定
两个按钮；解锁后窗口可拖动且位置被记住；弹窗关闭时记住当时停留在哪个界面，
下次唤出回到那里。

**Blocked by:** None

**Status:** implemented（2026-10-19，3 个 commit：
`364fd76` 唤出时回到上次停留的界面 / `2962ffe` 置顶+锁定+位置记忆 /
`bbc9f71` 定制面板「去设置里登录」直达按钮。885:0:13、tsc、51 node、38 site
全绿。**UI 验收待 owner**——「解锁后拖着走」未经真机实测）

## 要点

- 置顶（`popoverPin`）= 窗口 always-on-top；锁定（`popoverLock`）默认开，
  每次唤出重新居中且不可拖动
- 位置记忆（`popoverPos`）只在锁关时生效——锁着的弹窗位置不属于用户
- chrome 放在总花费行头部（每日明细按钮左边），因为那行永远最先看到；
  面板表单窗口自带边框，不给 chrome
- 卡片每次渲染都是重建的，解锁态的拖拽退出（卡片排序 grip）要重新挂上
- 「回到上次停留的界面」用 `lastView`（`{kind: main|customize|skin, id}`），
  provider 已被删除时退回列表

## 验收

- [x] 置顶/锁定按钮渲染在总花费行，两态文案与图标齐（en/zh/ru）
- [x] 锁定时每次唤出重新居中；解锁后拖拽区域生效
- [x] 位置在弹窗消失时记住，下次唤出原位放回
- [x] 切到自定义看板/皮肤工坊/provider 配置页后关闭弹窗，再次唤出回到那里
- [x] 新配置键 `popoverPin` / `popoverLock` / `popoverPos` 在前端
      `FRONTEND_CONFIG_KEYS` 与 Rust `CONFIG_KEYS` 两侧同步，默认值一致
- [x] 三个窗口权限（`allow-set-always-on-top` / `allow-center` /
      `allow-start-dragging`）加进 `src-tauri/capabilities/default.json`，
      只给 main 与 panel 窗口
- [x] 定制面板的基元律动行有「去设置里登录」按钮，点击后设置面板打开并
      定位到该 provider 的密钥行
- [ ] **待 owner UI 验收**：拖拽必须用真鼠标按住拖——agent 环境无法把按住的
      真实鼠标送进 WebView2，**「解锁后拖着走」只验证了接线**（属性生效、
      拖拽脚本响应、`start-dragging` 权限放行），未经真机实测
- [ ] **待 owner UI 验收**：置顶、锁定、位置记忆、回到上次界面的实际观感

## Comments

- 2026-10-19：本批原本是 3 个混在一起的 commit，拆分时按内容正则分桶
  （`temp/scripts/split-step.mjs`）。拆分后 `git diff backup-presplit HEAD`
  为 0 字节——内容零损失，验证后已删备份分支。
- 2026-10-19：定制面板的「去设置里登录」按钮与本票同批落地，但归属
  tokenrhythm provider（见 `.scratch/tokenrhythm-provider/`），此处只记
  一笔，避免两处票仓重复追踪。
