# 0.11.1 发布结果（2026-10-27）

- **模式**：正式版（GitHub Latest）；未使用 pre-release。
- **版本**：`0.11.1`；0.11.0 是未发布的本地 checkpoint，已并入 0.11.1 的 CHANGELOG 与发布说明。
- **PR**：#108 → 合并提交 `56e6353`。
- **Tag**：`v0.11.1`（annotated，`8eacd1f`）。
- **Release**：https://github.com/Aafff623/pane/releases/tag/v0.11.1 于 2026-10-27T15:48:42Z 发布。
- **产物**：Windows x64 NSIS 8.4 MB；macOS arm64 dmg 11.9 MB；Linux amd64 deb 14.8 MB / AppImage 88.2 MB；`latest.json` 已签名。
- **CI**：PR 与 main 的 CI/frontend/parse-tests/src-tauri-tests/site 全绿；CodeQL `analyze` 通过。PR 的 CodeQL 检查因大 diff 重报了 main 上已存在的 3 条 `js/xss-through-dom`（alerts #2/#3/#4），非本次引入；main 的 CodeQL 扫描为 success。
- **站点**：main push 的 Site 工作流 success；`site-meta.json` 为 0.11.1；`/api/latest` 返回 0.11.1；`/download/windows` 302 到 0.11.1 Windows 安装包。
- **Gitee 镜像**：release 工作流的 `gitee-mirror` job success。
- **Updater**：`latest.json` 的版本为 0.11.1，sha256 与 Windows 安装包一致。
- **Winget**：release 的自动 winget dispatch 失败，原因是新 ID `Aafff623.Pane` 还没有初始 manifest（旧 `Pane.Pane` 正在被移除）。已提交新包 PR：[microsoft/winget-pkgs#463652](https://github.com/microsoft/winget-pkgs/pull/463652)，等待 Microsoft 验证/合并；合并后后续版本恢复自动更新。
- **验收**：#107 皮肤工坊已由 owner 于 2026-10-27 验收通过；本版包含的其余 0.11.x 收尾项以 `.scratch/011-closeout/acceptance.md` 的“明确没做”表为准。
