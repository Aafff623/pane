# Review — 悬浮板与设置语义批次（2026-10-29）

## 方式

- 独立 code review 子代理审查 `src/main.ts`、`src/styles.css`、`src/i18n.ts`、`src/fontPacks.ts` 的未提交 diff；按 P0–P3 回报。
- 人工复核每条发现，跑构建、根测试、站点测试、真实窗口冒烟和检测器复测。

## 已修复的 review 发现

| 级别 | 发现 | 处理 |
|---|---|---|
| P2 | 折叠卡头仍输出裸已用百分比 | `fold-main-quota` 改用 `headLabel`（剩余/已用 + 量词） |
| P2 | hover、环形图、账号 meter/ring、key 行仍是裸已用百分比 | 新增 `quotaPctLabel()`；这些位置的标题/aria 带量词，环形与 meter 的填充方向跟随 `config.meterMode` |
| P2 | `jumpAnimation` 是唯一延迟落盘项，删掉脏态后关窗会静默丢失 | 改为 change 即 `patchConfig`，删除 `pendingJumpAnimation` |
| P2 | Ctrl+S 在输入框、授权中心、确认框里也会直接重载 | 加 `isTypingTarget`、`authViewActive`、`#confirm-overlay` 守卫 |
| P3 | 键盘跳转用 260ms 定时器，数据 tick 重建列表会丢焦点 | 改为 `pendingProviderFocus`，由 `renderDashboard` 重建后消费 |
| P3 | 字体包成功路径残留旧的错误 title | 成功后清 `title` |
| P3 | 过期注释（数据更新于 / dirty flag / role=button） | 同步改写 |
| P3 | 新反转逻辑没有测试 | 新增 `tests/panel-semantics.test.mjs`，锁住量词、条形方向和已删除的脏态机制 |

## 有意保留的取舍

1. **概览行不挂 `role="button"`**：行内还有账号折叠按钮，`role=button` 会把子按钮从读屏树里抹平。现在用 `tabindex=0` + 标题镜像为 `aria-label` + Enter/Space 跳转，键盘可操作，也不破坏嵌套按钮。
2. **环形图里的可见数字仍是纯数字**：环内空间放不下「剩余 100%」，完整语义放在 `aria-label` 与 tile tooltip；填充方向和数字都跟随同一约定。
3. **授权中心 demo 空白**：真实后端零账号也会返回全部 family（Rust 测试 `empty_store_yields_all_families_with_no_rows`），空白是 demo bridge 返回 `[]` 的数据缺口，不改产品代码。
4. **密度不变**：没有折叠额度总览的 9 个控件；只补标签、分组间距和字号。

## 验证证据

- 构建：`pnpm build` 通过。
- 根测试：257/257（新增 3 条）。
- 站点：`cd site && npm run build && npm test` 7/7；demo bundle 已重建。
- 检测器（`impeccable detect --cdp`）：设置暗/亮、悬浮板暗/亮、环图视图全部 0 findings（修复前 17 + 51）。
- 截图：`temp/logs/panel-semantics-20261029/`（设置 10 页、搜索空态、面板 440/380/环图、亮暗）。
- 真实窗口（debug 构建）：`剩余 100%` 标签、`重新载入`、设置窗 Ctrl+S 重载、密钥页 A–Z 索引完整、vault 提示正常。

## 未覆盖

- owner 的最终视觉验收；线上站点未更新（新站仍按约定保持本地）。

## 追加：双口径开关（2026-10-29 晚）

- Owner 要求保留原来的「已用越多越满」为默认，把「剩余越多越满」做成设置项。
- 新增 `config.meterMode`（used / left），默认 used；设置 → 常规 → 进度条切换，文本/条形/环形/账号小条/hover 同步。
- 实测坑：后端 `set_config` 按 `CONFIG_KEYS` 丢弃未知键，日志 `ignoring unknown key 'meterMode'`，前端看起来切换后又被还原。已把 `meterMode` 加入 whitelist，并在 `config_with_defaults` 种默认 `used`。
- 真实验证：默认 `已用 0%` 空条；切「剩余」后写入 `meterMode: "left"`，悬浮板即变为 `剩余 100%` 满条；再切回 `已用`，配置文件恢复 `meterMode: "used"`。
- 测试：`tests/panel-semantics.test.mjs` 同时断言前端默认值与后端 whitelist/defaults，防止只改一半。
