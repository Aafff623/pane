Status: in progress — #88 与 #57 已核验完成；#58-05 截图待生成（等三个 cargo 构建腾出机器）
Type: task
Blocked by: 01

# 09 · 皮肤、截图与许可（#57 / #58-05 / #88）

1. **#88 许可**：owner 已买断 oil-ui Pro；在 `site/DESIGN.md` §6 与
   `.scratch/site-oil-ui-cards/` 记录“自有 Pro 授权”，不再等待第三方许可。
2. **#57 市场**：确认 `skins/index.json`、zip 哈希与站点部署路径一致；
   测试通过后部署皮肤索引（owner 已授权）。
3. **#58-05**：生成 4 张原生（非 Signal）设置页截图，覆盖浅/深/三皮肤；
   写 6 项回归清单（行结构/pill/字号/折叠/滚动/键盘）；owner 逐项勾选。

## 验收

市场页能安装皮肤；截图与清单在 issue 中可见；owner 视觉通过。

## 结果

### #88 许可 — 已落地

- `.scratch/site-oil-ui-cards/license.md`（2026-10-26）：owner 买断 oil-ui Pro，
  四张信号卡按自有 Pro 授权使用；不复制未购买的第三方素材，不把 oil-ui 私有
  组件整包引入站点（现有实现是本仓库原生 CSS/JS）。
- `site/DESIGN.md` §6 现在也带同一条 **License** 记录，指向该文件——之前只在
  scratch 里有，设计契约本身没有写，等于下一个改这段的人看不到授权依据。

### #57 市场 — 一致性已端到端核验，且线上已发布

核验方式是「本地包 → 索引 → 线上文件」三处对齐，不是只跑一遍脚本：

| 项 | 结果 |
| --- | --- |
| `node scripts/check-skin-packs.mjs` | signal / ink / comic-pixel 均 v1.0.2，size ok、sha ok |
| 本地 zip sha256 | `88d97d67…`(signal) `61a375b7…`(ink) `a9376c70…`(comic-pixel)，与 `index.json` 三条逐一相符 |
| 本地 zip 字节 | 1528012 / 946684 / 109652，与 `index.json` 相符 |
| 线上 `https://pane.threetwoa.live/skins/index.json` | 与本地 `site/public/skins/index.json` **逐字节相同** |
| 线上三个 zip | HTTP 200；下载后 sha256 与索引完全一致（signal `88d97d67…`、ink `61a375b7…`、comic-pixel `a9376c70…`），字节数 946684 / 109652 与索引相符 |
| 客户端指向 | `src-tauri/src/skin_packs.rs:25` 与 `src/skins.ts:34` 都是同一个 `https://pane.threetwoa.live/skins/index.json` |
| 边缘路由 | `site/worker.js:245` 对 `/skins/` 加 CORS（GET/HEAD，OPTIONS 直接 204，不碰 asset store） |

**结论：皮肤市场当前就是可安装状态，不需要额外部署动作。** 之前记忆里「线上索引
过期，要等 0.8.10」已经不成立——线上跑的就是 1.0.2 三件套。

部署路径说明：`site/public/skins/` 随站点一起由 `.github/workflows/site-deploy.yml`
在 push main 且 `vars.SITE_DEPLOY == 'true'` 时 `wrangler deploy`。所以「部署皮肤
索引」不是一个独立动作，它跟站点发布同一条门；本票不去手动 `wrangler deploy`，
以免绕过 CI 门、把未验收的 0.11.0 站点提前推上线。后续皮肤版本更新仍走同一条路。

### #58-05 — 待做

**为什么还没做**：需要跑起来的应用截图（浅/深 × 三皮肤，原生设置页而非站点
Signal 卡），而当前有三个 implementer 子代理在并行跑 cargo 构建；此刻再起一个
dev 构建 + 启动 pane.exe 会抢机器，也容易和被杀掉的 dev 进程互相干扰。等它们
合并完再一次性做。

**6 项回归清单（owner 逐项勾选）** — 依据本票要求的 行结构 / pill / 字号 /
折叠 / 滚动 / 键盘：

1. **行结构**：设置页每一行仍是「左标签 + 右控件」单行结构；没有因为新增项
   而出现两行挤一行、标签被截断、或控件换行到标签下面。
2. **pill**：状态 pill（健康/错误/额度）形状与配色跟主面板一致；文字不溢出、
   不被裁切；错误态是红色错误而不是「0」。
3. **字号**：三皮肤下字号层级不变（标题 / 正文 / 标签三档）；换皮肤只换配色
   与质感，不把字号一起换掉；不出现合成粗体。
4. **折叠**：可折叠分组能正常展开/收起，展开后不遮挡下方内容，收起状态在
   重开设置窗口后保持一致。
5. **滚动**：长列表（账号、服务商、密钥）滚动顺滑无跳动；滚动到底部最后一行
   完整可见，不被底部按钮条压住；滚动条不覆盖内容。
6. **键盘**：Tab 能走遍所有可操作项且焦点可见；Enter/Space 能切换开关与按钮；
   Esc 能关闭设置窗口；焦点不会掉进不可见元素。

截图交付方式：4 张原生设置页截图（浅/深 × 三皮肤覆盖），生成后贴进 GitHub
issue #58 并在此文件登记文件名，owner 视觉通过后才关票。
