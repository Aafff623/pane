# 06 · 进度条两种口径与设置开关

Status: done 2026-10-29 | Priority: P0 | Depends: 01

## Scope

- 新配置 `meterMode: "used" | "left"`，默认 `used`（原来的“已用越多越满”）；`left` 时剩余越多越满。
- 文本、条形、环形、折叠徽章、账号小条、hover 与 aria 全部跟随同一口径并带「已用/剩余」。
- 设置 → 常规 → 进度条：已用 / 剩余 分段开关，即切即存；跨窗口通过 config-updated 触发重绘。
- 后端 `src-tauri/src/lib.rs`：`CONFIG_KEYS` 收录 `meterMode`，`config_with_defaults` 种默认 `used`；否则 `set_config` 会按未知键丢弃（实测日志 `ignoring unknown key 'meterMode'`）。
- demo bridge 支持 `?meter=left`，方便展示与截图。
- 回归测试断言前后端都保留该键与默认值。

## Done when

- 真实窗口：默认显示「已用 0%」空条；切到「剩余」后同一张卡显示「剩余 100%」满条并持久化。
- `pnpm build`、根测试、站点测试通过，检测器两套口径均为 0。
