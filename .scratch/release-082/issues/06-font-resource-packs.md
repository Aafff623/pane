# 字体资源包（按需下载，不占安装包）

**What to build:** 按皮肤资源包的成熟形态做字体包：zip + manifest + 逐文件 sha256，校验通过才落盘，安装后字体出现在字体选择里；设置页给出入口与已装列表。

**Blocked by:** None

**Status:** implemented — awaiting owner UI acceptance / site publication

- [x] 字体包格式与校验复用 skin_packs 的既有模式（不新造轮子）
- [x] 安装后字体下拉出现该字体并可直接应用；卸载后回落系统字体（已接线，UI 待 owner 验收）
- [x] 损坏/篡改的包整包拒装，不留残骸
- [x] Rust 侧纯逻辑（manifest 解析/校验/路径）进 parse-tests 并有测试
- [x] 设置页入口在「常规」或字体行附近，三语文案（主端接线）

## Comments

- 2026-10-14：由 spec `.scratch/release-082/spec.md` 拆分（owner 批次需求）。
- 2026-10-14：新增 `font_packs.rs` / `fontPacks.ts`；复用 `skin_packs::verify_pack` 与 `validate_manifest`，仅扩大两函数到 crate 可见；安装前全量校验，临时目录写入后 rename 替换，旧包保留至替换成功；字体 CSS 由 Rust 生成，不加载包内任意 CSS。
- 包格式：皮肤 manifest 基础字段（`entry` 为字体文件）加 `family`（`Pane Pack ` 前缀的应用内别名）、`license`（已列入 SHA256 表的许可证路径）、`faces: [{path,weight,style}]`。支持 TTF/OTF/WOFF/WOFF2；继承 12 MiB/文件、32 MiB/包上限，并限制总解压大小。下载另验 zip 整体 SHA256。
- `python site/scripts/build-font-packs.py` 生成官网 `site/public/fonts/index.json` 与两个 zip：马善政 UI 子集（OFL 1.1，103433 bytes）、Permanent Marker（Apache 2.0，46459 bytes）。复用已跟踪字体原始字节，各包附原许可证与来源；无新增安装包字体导入。马善政仅覆盖现有 UI 字符，缺字由系统字体兜底。
- 静态检查 `node node_modules/typescript/bin/tsc --noEmit` 通过。最终 Rust `cargo +stable-x86_64-pc-windows-gnu test --manifest-path parse-tests/Cargo.toml font_packs:: --quiet` 四项通过：真实官方包、安装/替换/篡改无残留、生成 CSS/许可/face 校验、ZIP 路径/大小写重复/Windows 歧义路径拒绝。Rust 文件已 rustfmt。官网资源当前仅本地生成，需 owner 授权发布站点后下载入口才可在线使用。
- Owner UI 验收：设置字体附近打开资源包入口 → 下载/安装 → 字体下拉选择 → 重启保持 → 卸载当前字体回系统默认；导入篡改包应报错且原包保留。原生 WebView 实机验收未执行。
- 目录回退：安装包只带两条官方 catalog 元数据，入口立即展示可用包，官网目录暂不可达时保留列表并明确报源不可用；下载字体仍走网络和整包 SHA256；官网 `/demo/` 使用同源 `/fonts/index.json`。Rust list/install/remove 使用同一进程互斥锁，避免列表扫描读到替换/卸载中间态。
- 分享链路静态核实：`copyElementSnapshot` 会遍历 `document.styleSheets`，包含字体模块动态 `style` 的 `@font-face`；字体源已是 data URL，不需 SVG 再发网络请求。实际截图字体效果待 owner 验收。
