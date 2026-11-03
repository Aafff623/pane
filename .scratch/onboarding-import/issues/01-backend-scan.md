# 01 · 后端：cc-switch 扫描 + 动态匹配

**Status:** done（待实机验证） · **依赖:** 无

## 做什么

1. 新模块 `src-tauri/src/import_sources.rs`：
   - 只读解析 `~/.cc-switch/config.json`（≤4 MB），遍历分区（claude/codex/gemini/grok/kimi/opencode/hermes/mcode/agy/omp/pi/openclaw）里的 providers。
   - 取 key：已知 env/auth 键名优先（ANTHROPIC_AUTH_TOKEN、ANTHROPIC_API_KEY、OPENAI_API_KEY、GEMINI_API_KEY、GOOGLE_API_KEY、apiKey/api_key），递归兜底；base URL 取 env 的 *_BASE_URL / baseUrl / base_url / endpoint，以及 codex `config` TOML 里的 `base_url = "…"`。
   - 动态匹配：host 优先（与 `providerMechanisms.ts` 的 hosts 对齐），名称别名兜底；只接受 `provider_dispatch::supports_key_probe` 的 family。
   - 候选去重（section+providerId+key hash）；密钥只留后端缓存，DTO 只给掩码与稳定 id。
   - 已存在同 family 同密钥 → `duplicate`。
2. `provider_dispatch::supports_key_probe()`：与 `key_snapshot_for` 分支保持一致（stepfun-plan、openai-api 不进自动匹配）。
3. 命令：`import_scan`。

## 验收

- 单测：claude/codex/gemini/其它分区解析、缺键、TOML base_url、掩码、去重、host/name 匹配优先级、duplicate。
- `cargo check` + `parse-tests` 通过。
