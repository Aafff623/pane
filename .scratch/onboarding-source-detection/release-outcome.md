# 0.12.0 发布结果（2026-10-28）

- **模式**：正式版（GitHub Latest）；非 pre-release。
- **版本**：`0.12.0`（minor：本批含首启检测、异常状态卡、帮助弹窗等新功能）。
- **Release commit**：`8098879`；**Tag**：`v0.12.0`（annotated，tag 对象 `09e3f68`）。
- **Release**：https://github.com/Aafff623/pane/releases/tag/v0.12.0，2026-10-28T12:41:52Z 发布。
- **产物**：Windows x64 NSIS 8.4 MB；macOS arm64 dmg 11.9 MB（本批 CI 未产出 x86_64 dmg）；Linux amd64 deb 14.9 MB / AppImage 88.3 MB；矩阵表由 CI 按实际 artifact 生成。
- **CI**：Windows / macOS / Linux 三个 build job 与 release job 全部 success。
- **站点**：`/api/latest` 返回 0.12.0；首页显示 v0.12.0；`/download/windows` 302 到 `Pane-Aafff623_0.12.0_x64-setup.exe`。
- **Updater**：`latest.json` 版本 0.12.0，Windows 安装包 sha256 `157d7f43…2de35` 与实际下载文件一致，签名存在。
- **Gitee 镜像**：release 工作流的 `gitee-mirror` job success。
- **Winget**：自动 dispatch 再次失败，原因仍是 `Aafff623.Pane` 尚无已合并的初始 manifest。首个包 PR [microsoft/winget-pkgs#463652](https://github.com/microsoft/winget-pkgs/pull/463652) 仍在人工审核；合并后需手动 dispatch `winget.yml`（tag `v0.12.0`）补发本版。
- **验收**：owner 已于 2026-10-28 验收本轮 UI（异常状态卡、帮助弹窗、来源探测与首启流程）并授权发布。
- **发布前验证**：`corepack pnpm build` 通过；Node 247/247；站点 55/55（含 i18n pairing、demo provenance、版本/计数锁）；`parse-tests` 串行去代理 1200 passed / 0 failed / 18 ignored；`cargo +stable-x86_64-pc-windows-gnu check` 通过。
