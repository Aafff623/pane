# 01 — 接入描述表

**Blocked by:** None
**Status:** done

新增 `src/providerOnboarding.ts`：`CredentialKind` / `AccountAction` / `Onboarding` 类型与 `onboardingFor(family)`。

**规则**
- 所有 catalog family 都能解析出条目（默认 + 覆盖）。
- 登录型 family（`supports_api_key=false && supports_extra_accounts=true`，以及 qodercn/traecn 等本地读取型）不得含 `pasteKey`。
- key 型 family 必须有 `pasteKey`。
- 搜索类（bocha/tavily/firecrawl/brave）用 `pasteKeyPage`。
- CodeBuddy 四兄弟、qoder/kiro/windsurf/trae/zed 用 `addLogin`。

**验收**
- [ ] 类型收窄，无 `any`
- [ ] 70 个 family 均有条目（脚本/人工核对）
- [ ] `tsc` 通过
