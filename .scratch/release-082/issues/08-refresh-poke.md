# 刷新点自动触发（默认关，用户显式开启）

**Status:** removed（2026-10-21，整套 `reset_wake` 后端/前端/测试删除；用户决定后续版本重新设计）

**What to build:** 为「必须被请求一次才进入计费周期」的 5 小时套餐（GLM / MiniMax 等）加调度：到下次刷新时刻自动发一次轻量请求，并在卡片上给出「已自动触发」标识。默认关闭。

**Blocked by:** None

**Status:** in-review（已接线并完成静态/单测；不实发计费请求，owner 终验待做）

- [x] 「何时该触发」抽纯函数（窗口/刷新点计算）并进 parse-tests
- [x] 默认关闭；开启需用户在设置中明确同意（会消耗一次真实请求）
- [x] 触发成功/失败都在卡片上有可读状态，不静默
- [x] 同一刷新点只触发一次（幂等），应用重启后不重复触发
- [x] 覆盖 provider 可配置（GLM / MiniMax / 其它同形态厂商）

## Comments

- 2026-10-14：由 spec `.scratch/release-082/spec.md` 拆分（owner 批次需求）。
- 2026-10-14：后端 `reset_wake.rs` 已实现独立 opt-in 设置、5h 刷新点纯函数、安全边界 2min、冷却/过期保护 10min、请求前 sync + 原子替换状态落盘；成功、失败、崩溃后结果未知分别记录，同一 reset 不重试。每厂商独立开关和区域；禁用 provider 不请求。UI/后台接线由主 Agent 负责，验收待完成。
- 官方端点核实：国际 GLM Coding Plan 的 OpenAI base URL 来自 https://docs.z.ai/devpack/tool/others ，模型/Chat Completion 参数来自 https://docs.z.ai/api-reference/llm/chat-completion 。MiniMax 国际/CN 的 base URL 和 `MiniMax-M2.7` 来自 https://platform.minimax.io/docs/api-reference/text-openai-api 与 https://platform.minimax.cn/docs/api-reference/text-openai-api 。Z.ai CN 官方文档未获取成功，因此该区域自动触发明确拒绝启用。Context7 返回鉴权错误，已改读官方网页。
- 安全边界：固定 URL allowlist、不接受用户 URL、不跟随重定向；只读对应厂商既有 key API；失败消息不回显请求错误链或厂商原始 response body。真实请求会消耗额度，本轮没有实发，运行/UI 验收仍需用户完成。
- 后端验证：`cargo +stable-x86_64-pc-windows-gnu test --manifest-path parse-tests/Cargo.toml reset_wake::tests --quiet`，9 passed / 0 failed（2026-10-14）；覆盖默认关闭/同意、关闭单厂商无需再次同意、2min 边界/10min 冷却、重启与时钟回拨去重、真实文件 sync/替换/损坏拒绝、仅可信5h快照调度、下一窗口已使用时 skipped 不再请求、HTTP-200 业务失败解析。`rustfmt --check` 新模块通过。请求执行与用户 UI 验收未验证，不能据此宣称自动唤醒已实测成功。

- 2026-10-14（主端接线）：专用命令读写设置与状态；Rust 独立60秒调度，使用真实刷新快照，禁用provider跳过；前端设置区提供厂商/区域开关和真实请求确认，卡片显示waiting/attempted/success/failed/skipped，跨窗口事件同步。浏览器demo键盘开启会先弹确认，取消后仍关闭。
- 后续核实已取得智谱中国官方正文 https://docs.bigmodel.cn/cn/coding-plan/tool/others ：OpenAI Coding Base URL 明确为 `https://open.bigmodel.cn/api/coding/paas/v4`，示例模型 `glm-5.2`。因此前述CN不可用限制已解除；固定allowlist加入CN路径，无自动跨区重试。未发送真实请求。
