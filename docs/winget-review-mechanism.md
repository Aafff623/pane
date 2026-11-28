# WinGet 新包审核：机制、队列与我们的处境

2026-11-28 实测。给外部顾问（哈基米）补充背景用。

## 一、我们卡在哪

**PR [microsoft/winget-pkgs#463652](https://github.com/microsoft/winget-pkgs/pull/463652)**，提交于 2026-10-28，今天是第 31 天。

时间线：

| 日期 | 事件 |
|---|---|
| 2026-10-28 04:08 | 提交 |
| 2026-10-28 04:52 | 自动验证通过，转入人工审核 |
| 2026-11-06 02:46 | 审核员 stephengillie 发现 `Policy-Test-1.2` 误报 |
| 2026-11-06 05:53 | 同一审核员豁免该检查 |
| 2026-11-06 06:37 | 重新进入人工审核队列 |
| 2026-11-19 04:20 | 我们把 manifest 从 0.11.1 升到 0.12.3 |
| 2026-11-21 23:26 | 系统再次确认"等待审核员批准" |
| 2026-11-28 | 仍在队列，无新进展 |

当前标签：`Azure-Pipeline-Passed`、`Validation-Completed`、`New-Package`。
**没有 `Needs-Author-Feedback`** —— 不是要我们改什么，就是在排队。

那个 `Policy-Test-1.2` 误报值得单独说：检查器把描述里的 `reset times` 里的 "reset" 当成了杀毒软件品牌 **ESET**，判定为"冒用品牌名"。审核员手动豁免了。

## 二、队列有多长（实测数据）

| 指标 | 数字 | 取法 |
|---|---|---|
| 待审新包总数 | **4800** | `label:New-Package label:Validation-Completed is:open` |
| 比我们早提交的 | **1471** | 同上加 `created:<2026-10-28` |
| 队列最老的一个 | 2026-09-09 提交 | 已压 **80 天** |
| 本月合入新包 | 686 | `is:merged label:New-Package merged:2026-11-01..` |
| 最近 8 天合入 | 127 | ≈16 个/天 |

按 16 个/天推算，清空 4800 个队列要 **300 天**。我们在第 1471 位，理论上还要 92 天。

**注意**：这不是严格 FIFO。官方文档说"审核员通常从最老的合格 PR 开始，但具体位置会变化，因为不同提交需要不同程度的审查"。

## 三、审核是志愿者吗？多少人？

**是。** 官方 `doc/Moderation.md` 明确写：

> In addition to Microsoft employees, several community members have been identified through their high-quality submissions, willingness to help others, and adherence to our code of conduct.

名单规模（文档原文列出）：

| 类别 | 人数 |
|---|---|
| Windows Package Manager 管理员（微软员工） | **11** |
| 社区审核员 | **7** |
| 已退休审核员 | 3 |

合计 **18 人**，服务 4800 个待审 PR。

**成为审核员的门槛**，文档原话：

> Becoming a moderator is like becoming a Microsoft Most Valuable Professional. **There isn't a set formula.** We're looking at what individual contributors are doing, and how they are doing it. If the need arises to add additional moderators, nominations may come from one of the core team members or an existing moderator.

翻译：**没有申请入口**。由核心团队或现有审核员提名，公开讨论后授予。评估维度是提交 PR 的数量、在项目里的活跃时长、与其他提交者的互动。

**成本**：没有金钱成本，但有真实的时间成本——需要长期（数月）持续贡献 manifest 维护、回答别人的问题，且要被现有审核员看见。**对我们来说不现实**：我们是一个产品的维护者，不是 winget 的长期贡献者。

## 四、审核员具体看什么？

自动验证已经覆盖了绝大部分技术检查（10 项流水线，`Installation Validation` 一项就要跑约 34 分钟，实际安装你的包并验证）。

人工审核要判断的是自动检查**做不到**的那部分，官方文档对"新包"特别说明：

> New packages generally require **more scrutiny** because moderators must establish **package identity, installer provenance, metadata accuracy, and policy compliance without prior manifest history**.

拆开就是四件事：

| 审核项 | 具体判断 | 我们的状态 |
|---|---|---|
| **包身份** | ID 是否属于提交者，会不会冒用他人 | ✅ 已通过 issue #105 解决 |
| **安装包来源** | 下载地址是不是发布者的官方渠道 | ✅ GitHub Releases |
| **元数据准确性** | 发布者、支持链接、许可证是否属实 | ✅ 已核对 |
| **政策合规** | 是否含恶意行为、品牌词误用等 | ✅ 误报已豁免 |

**关键差异**：更新已有包（`New-Manifest` 标签）审核成本低得多，因为"prior manifests provide clear precedent"。**新包没有历史可参照，每项都要从头建立信任。** 这就是我们慢的根本原因。

## 五、为什么会有这个 PR？包名的历史

**这是理解全局的关键，不能漏。**

1. **上游项目叫 Pane**，作者 [ItsJazii](https://github.com/ItsJazii/pane)，MIT 许可。
2. 我们基于它开发（Pane 现在已是独立产品，见 `CONTEXT.md`）。
3. **早期我们用了上游的 winget 包 ID `Pane.Pane`** 发布自己的版本。
4. 上游作者 [开 issue #105](https://github.com/Aafff623/pane/issues/105) 指出问题，列了三条硬伤：
   - 每个 manifest 仍写着他是发布者、支持页、版权 —— 用户分不清这不是他的构建
   - 我们的版本号更高，`winget upgrade Pane.Pane` 会把他的应用替换成我们的
   - **Tauri identifier 还是 `com.jazii.pane`**，安装目录和卸载项也叫 `Pane` —— 两个应用互相覆盖、共用数据目录
5. 我们照做了身份分离（这就是 0.11.x 那些「identity separation」提交的来源）。
6. 代价：`Pane.Pane` 下的 13 个历史版本要全部删除，然后以新 ID `Aafff623.Pane` 重新提交 —— **于是有了这个新包 PR，于是进了 4800 人的队列**。

**所以队列不是运气问题，是纠正一个历史错误必须付的账。** 上游作者的要求完全合理，我们做得也对。

## 六、A/B/C/D 四个方案（已确认执行）

| 方案 | 内容 | 结论 |
|---|---|---|
| A | 继续等，每个版本更新 manifest | 保留 PR，但不再为它付出额外精力 |
| B | **改做 Chocolatey** | ✅ 执行 |
| C | 双管：winget 挂着 + 补 Chocolatey | ✅ 执行 |
| D | 不做包管理器 | 官网下载 + 自建更新器已覆盖 |

**"换仓库重新申请包名能不能绕过排队"——答案是不能。**

这是最容易误解的一点，必须讲清：

```
winget 的排队单位是「新包 ID」，不是「仓库」
```

- 新建一个 GitHub 仓库再提 `New-Package` PR → **进入同一个 4800 人队列**，因为它同样是"没有历史记录的新 ID"
- 只有「更新已有包」才快（`New-Manifest`），而 `Aafff623.Pane` 现在还不存在，无法更新
- 更糟：新仓库会把 issue #105 里已经解决的身份问题重新搞乱

**唯一能绕开的条件**：如果 `Aafff623.Pane` 已经有过任何一个合并的版本，之后的升级就快。所以我们真正需要的是**第一次合并**，无论多慢。

## 七、真正的替代方案

| 渠道 | 审核机制 | 实际周期 | 成本 |
|---|---|---|---|
| **官网直装** | 无 | 即时 | 已有 |
| **自建更新器** | 无 | 即时 | 已有（minisign 签名） |
| **Chocolatey 社区库** | 人工审核，但队列远短 | 1–7 天 | 免费 |
| **Chocolatey 自建源** | 无 | 即时 | 免费，需用户加源 |
| **MSIX / Microsoft Store** | 微软审核 | 1–3 天 | ≈¥700/年开发者账号 |
| **Scoop** | 社区 PR | 数天 | 免费 |
| winGet | 志愿者，4800 队列 | **80+ 天** | 免费 |

## 八、可提取的共性知识点

1. **社区包管理器的瓶颈是"首次提交"，不是包名冲突。** 命名唯一性解决的是能不能提，历史记录决定提得快不快。
2. **志愿者审核的吞吐量与队列规模成反比，且没有 SLA。** 4800 个待审、18 个审核员、16 个/天 → 数学上必然数月。
3. **自动验证通过 ≠ 快。** 自动化只筛掉不合格的，合格的全堆在人工环节。
4. **"没有历史"意味着更高的审查成本。** 有先例可循的更新（New-Manifest）比首次提交（New-Package）快一个数量级。
5. **换仓库不能规避队列。** 排队单位是包 ID 的全生命周期，不是代码托管位置。
6. **成为审核员没有申请通道。** 是提名制 + 长期贡献，不是"报名"。
7. **单一分发渠道是风险。** 官网 + 自建更新器 + 至少一个第三方渠道，才能保证任何一环堵塞都不影响用户。
8. **改正历史错误的代价是可见的、可计算的。** 我们这次是 31 天（截至今天）并且还在计。

## 九、需要外部顾问回答的（哈基米）

1. 有没有**官方渠道**能查询 PR 在队列中的实际位置，而不是靠推测？
2. 社区是否有**加速惯例**（例如同时维护多个包、或先贡献 manifest 再提自己的）？
3. Chocolatey 的社区库对 Tauri 应用有无已知问题（NSIS 安装包是否被接受）？
4. MSIX 打包 Tauri 应用的实际工作量与坑？
5. 如果我们**关闭当前 PR 重开**，会不会反而排到队尾？（看起来会，所以没做）
