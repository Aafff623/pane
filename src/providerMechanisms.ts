/// How every provider gets its numbers — the explainer behind the "?" on
/// the settings key rows. Structural facts only (credential kind, files read,
/// env vars, hosts): the prose labels live in i18n, so a path or host can
/// never drift into a translated string. Every entry is taken from the
/// shipped provider source; update it when a provider's credential path or
/// endpoint changes (parse-tests' catalog gate keeps the family list honest).

export type MechanismKind = "key" | "local" | "mixed";

export interface Mechanism {
  kind: MechanismKind;
  /** Files / databases read on this machine (absent = nothing local). */
  reads?: string[];
  /** Environment variables honored, in priority order. */
  env?: string[];
  /** Hosts queried for the numbers (absent = local-only). */
  hosts?: string[];
  /** Literal quota fields and documented limitations; labels are translated by the caller. */
  provides?: string[];
  limitsKey?: string;
  credential?: "key" | "session" | "cookie" | "oauth" | "local" | "cloud";
}

export const MECHANISMS: Record<string, Mechanism> = {
  "claude": { credential: "local", limitsKey: "provider.limit.general", provides: ["5-hour usage", "Weekly usage", "Sonnet / Opus weekly", "Extra usage"], kind: "local", hosts: ["platform.claude.com", "api.anthropic.com"], reads: ["~/.claude/.credentials.json", "~/.claude.json (identity)", "CLAUDE_CONFIG_DIR"], env: ["CLAUDE_CONFIG_DIR"] },
  "codex": { credential: "local", limitsKey: "provider.limit.general", provides: ["Session", "Weekly", "Additional model windows", "Credits"], kind: "local", hosts: ["api.openai.com", "auth.openai.com", "chatgpt.com"], reads: ["~/.codex/auth.json", "CODEX_HOME/auth.json"], env: ["CODEX_HOME"] },
  "cursor": { credential: "local", limitsKey: "provider.limit.general", provides: ["Plan usage", "Requests", "On-demand", "Credits"], kind: "local", hosts: ["cursor.com", "api2.cursor.sh"], reads: ["%APPDATA%/Cursor/User/globalStorage/state.vscdb"] },
  "opencode": { credential: "key", limitsKey: "provider.limit.general", provides: ["Session", "Weekly", "Monthly"], kind: "mixed", reads: ["auth.json", "opencode.db"], env: ["OPENCODE_GO_API_KEY"], hosts: ["opencode.ai"] },
  "copilot": { credential: "local", limitsKey: "provider.limit.general", provides: ["Premium requests", "Other returned quotas", "Reset time"], kind: "local", hosts: ["api.github.com"], reads: ["%LOCALAPPDATA%/github-copilot/apps.json", "%LOCALAPPDATA%/github-copilot/hosts.json", "gh/hosts.yml + system credential store"] },
  "grok": { credential: "local", limitsKey: "provider.limit.general", provides: ["Usage windows", "Reset credits"], kind: "local", hosts: ["grok.com", "auth.x.ai", "cli-chat-proxy.grok.com"], reads: ["~/.grok/auth.json"] },
  "devin": { credential: "local", limitsKey: "provider.limit.general", provides: ["Daily quota", "Weekly quota"], kind: "local", hosts: ["server.codeium.com"], reads: ["%APPDATA%/devin/credentials.toml", "%LOCALAPPDATA%/devin/credentials.toml", "~/.local/share/devin/credentials.toml"] },
  "minimax": { credential: "key", limitsKey: "provider.limit.general", provides: ["Session", "Weekly"], kind: "mixed", hosts: ["api.minimax.io", "api.minimaxi.com"], reads: ["~/.minimax/config.yaml (provider.minimax.options.apiKey)", "~/.minimax/sqlite.db", "~/.minimax/v2/sqlite/runtime-state.sqlite"], env: ["MINIMAX_API_KEY"] },
  "openrouter": { credential: "key", limitsKey: "provider.limit.management", provides: ["Balance", "Credits", "Key limit"], kind: "key", hosts: ["openrouter.ai"], reads: ["OpenCode auth.json: openrouter"], env: ["OPENROUTER_API_KEY"] },
  "zai": { credential: "key", limitsKey: "provider.limit.general", provides: ["Web Searches"], kind: "mixed", hosts: ["api.z.ai", "open.bigmodel.cn"], reads: ["~/.config/zai/key.json", "~/.zcode/v2/credentials.json (weekend grant)"], env: ["ZAI_API_KEY", "GLM_API_KEY"] },
  "antigravity": { credential: "local", limitsKey: "provider.limit.general", provides: ["Per-model quota", "Reset time"], kind: "local", hosts: ["daily-cloudcode-pa.googleapis.com", "cloudcode-pa.googleapis.com", "oauth2.googleapis.com"], reads: ["Running IDE language_server process", "System credential store: gemini:antigravity"] },
  "deepseek": { credential: "key", limitsKey: "provider.limit.general", provides: ["Quota / balance fields returned by this provider"], kind: "key", env: ["DEEPSEEK_API_KEY"], hosts: ["api.deepseek.com"] },
  "moonshot": { credential: "key", limitsKey: "provider.limit.general", provides: ["Vouchers", "Cash"], kind: "key", env: ["MOONSHOT_API_KEY", "KIMI_API_KEY"], hosts: ["api.moonshot.ai", "api.moonshot.cn"] },
  "elevenlabs": { credential: "key", limitsKey: "provider.limit.general", provides: ["Characters", "Voice slots", "Overage"], kind: "key", env: ["ELEVENLABS_API_KEY", "XI_API_KEY"], hosts: ["api.elevenlabs.io"] },
  "ollama": { credential: "local", limitsKey: "provider.limit.localService", provides: ["Local models", "Running models"], kind: "local", hosts: ["127.0.0.1:11434"] },
  "codebuff": { credential: "key", limitsKey: "provider.limit.general", provides: ["Credits", "Balance", "Weekly"], kind: "mixed", hosts: ["www.codebuff.com"], reads: ["~/.config/manicode/credentials.json"], env: ["CODEBUFF_API_KEY"] },
  "kilo": { credential: "key", limitsKey: "provider.limit.general", provides: ["Credits", "Kilo Pass"], kind: "mixed", hosts: ["app.kilo.ai"], reads: ["~/.local/share/kilo/auth.json"], env: ["KILO_API_KEY"] },
  "aihubmix": { credential: "key", limitsKey: "provider.limit.general", provides: ["Usage", "Limit", "Used"], kind: "key", hosts: ["aihubmix.com"], reads: ["OpenCode auth.json: aihubmix"], env: ["AIHUBMIX_API_KEY"] },
  "onenewapi": { credential: "key", limitsKey: "provider.limit.general", provides: ["Key billing usage / balance", "Subscription windows with dashboard token"], kind: "local" },
  "qwen": { credential: "key", limitsKey: "provider.limit.qwen", provides: ["Local request / token ledger", "Cloud quota when accessible"], kind: "mixed", hosts: ["modelstudio.console.alibabacloud.com", "bailian.console.aliyun.com", "coding-intl.dashscope.aliyuncs.com"], reads: ["~/.qwen/usage/token-usage-YYYY-MM.jsonl"], env: ["BAILIAN_TOKEN_PLAN_API_KEY", "DASHSCOPE_API_KEY"] },
  "hermes": { credential: "local", limitsKey: "provider.limit.ledger", provides: ["Local tokens", "Sessions", "Estimated local cost"], kind: "local", reads: ["%LOCALAPPDATA%/hermes/state.db"] },
  "kimi": { credential: "key", limitsKey: "provider.limit.general", provides: ["Monthly"], kind: "mixed", hosts: ["api.kimi.com", "auth.kimi.com", "www.kimi.ai"], reads: ["~/.kimi-code/credentials/kimi-code.json", "~/.kimi/credentials/kimi-code.json", "KIMI_CODE_HOME/credentials/kimi-code.json"], env: ["KIMI_CODE_HOME", "KIMI_CODING_API_KEY"] },
  "stepfun": { credential: "key", limitsKey: "provider.limit.general", provides: ["Balance", "Cash", "Vouchers"], kind: "key", env: ["STEPFUN_API_KEY"], hosts: ["api.stepfun.com", "api.stepfun.ai"] },
  "stepfun-plan": { credential: "session", limitsKey: "provider.limit.stepPlan", provides: ["Session", "Weekly", "Credit", "Top-up"], kind: "local", reads: ["stepfun-plan.json"], hosts: ["platform.stepfun.com"] },
  "shandianshuo": { credential: "local", limitsKey: "provider.limit.general", provides: ["Speech minutes", "Assistant credits", "Reset time"], kind: "local", hosts: ["api.shandianshuo.cn"], reads: ["Windows Credential Manager: auth_token.Shandianshuo Safe Storage"] },
  "siliconflow": { credential: "key", limitsKey: "provider.limit.siliconflow", provides: ["API key connectivity (models)", "Balance API discontinued"], kind: "key", env: ["SILICONFLOW_API_KEY"], hosts: ["api.siliconflow.cn", "api.siliconflow.com"] },
  "novita": { credential: "key", limitsKey: "provider.limit.general", provides: ["Balance"], kind: "key", env: ["NOVITA_API_KEY"], hosts: ["api.novita.ai"] },
  "relaybalance": { credential: "key", limitsKey: "provider.limit.general", provides: ["Usage", "Balance", "Limit"], kind: "key", hosts: ["（你自填的中转站 base URL）"] },
  "qodercn": { credential: "local", limitsKey: "provider.limit.general", provides: ["Credits", "Model-specific packs", "Expiry"], kind: "local", hosts: ["openapi.qoder.com.cn"], reads: ["%APPDATA%/com.qodercn.app.stable/auth.v1.dat", "%APPDATA%/com.qodercn.app.stable/Local State"] },
  "traecn": { credential: "local", limitsKey: "provider.limit.general", provides: ["Credits", "Plan"], kind: "local", reads: ["storage.json"], hosts: ["api.trae.cn"] },
  "commandcode": { credential: "key", limitsKey: "provider.limit.general", provides: ["Monthly", "Extra credits"], kind: "mixed", hosts: ["api.commandcode.ai"], reads: ["~/.commandcode/auth.json"], env: ["COMMAND_CODE_API_KEY"] },
  "doubao": { credential: "local", limitsKey: "provider.limit.doubao", provides: ["Subscription", "Reset cards"], kind: "local", hosts: ["www.doubao.com"], reads: ["%APPDATA%/Doubao/User Data/Default/Network/Cookies", "%APPDATA%/Doubao/User Data/Local State", "%APPDATA%/Pane/doubao_cookies.json"] },
  "clawsgo": { credential: "session", limitsKey: "provider.limit.general", provides: ["Monthly", "Balance", "30-Day Usage"], kind: "key", env: ["CLAWSGO_TOKEN"], hosts: ["api.clawsgo.ai"] },
  "bocha": { credential: "key", limitsKey: "provider.limit.general", provides: ["Balance per key"], kind: "key", hosts: ["api.bochaai.com"], reads: ["Pane MCP key store (search-keys.json)"] },
  "tavily": { credential: "key", limitsKey: "provider.limit.shared", provides: ["Account plan credits", "Key usage / limit", "PAYG usage"], kind: "key", hosts: ["api.tavily.com"], reads: ["Pane MCP key store (search-keys.json)"] },
  "firecrawl": { credential: "key", limitsKey: "provider.limit.firecrawl", provides: ["Remaining credits", "Plan credits", "Billing reset"], kind: "key", hosts: ["api.firecrawl.dev"], reads: ["Pane MCP key store (search-keys.json)"] },
  "clinepass": { credential: "key", limitsKey: "provider.limit.general", provides: ["Quota / balance fields returned by this provider"], kind: "key", env: ["CLINE_API_KEY"], hosts: ["api.cline.bot"] },
  "sensenova": { credential: "session", limitsKey: "provider.limit.session", provides: ["5h / 7d default credits", "Flash-Lite credits", "Grants"], kind: "local", reads: ["sensenova.json"], hosts: ["platform.sensenova.cn"] },
  "apigoto": { credential: "key", limitsKey: "provider.limit.general", provides: ["Subscription", "Balance", "Points", "Credits", "Plan"], kind: "key", env: ["APIGOTO_API_KEY"], hosts: ["api.apigoto.com"] },
  "brave": { credential: "key", limitsKey: "provider.limit.brave", provides: ["Monthly search allowance", "Remaining searches"], kind: "key", hosts: ["api.search.brave.com"], reads: ["Pane MCP key store (search-keys.json)"] },
  "amp": { credential: "key", limitsKey: "provider.limit.general", provides: ["Amp Free", "Individual credits"], kind: "key", env: ["AMP_API_KEY"], hosts: ["ampcode.com"] },
  "bedrock": { credential: "local", limitsKey: "provider.limit.general", provides: ["Bedrock monthly cost", "User-configured budget"], kind: "local", env: ["AWS_ACCESS_KEY_ID", "AWS_SECRET_ACCESS_KEY", "AWS_SESSION_TOKEN", "AWS_PROFILE"], hosts: ["ce.us-east-1.amazonaws.com"] },
  "chutes": { credential: "key", limitsKey: "provider.limit.general", provides: ["4-hour quota", "Monthly"], kind: "key", env: ["CHUTES_API_KEY"], hosts: ["api.chutes.ai"] },
  "deepgram": { credential: "key", limitsKey: "provider.limit.billing", provides: ["Quota / balance fields returned by this provider"], kind: "key", env: ["DEEPGRAM_API_KEY"], hosts: ["api.deepgram.com"] },
  "kiro": { credential: "local", limitsKey: "provider.limit.general", provides: ["Credits", "Overage", "Overage charges"], kind: "local", reads: ["%LOCALAPPDATA%\\kiro-cli\\data.sqlite3", "%APPDATA%\\kiro-cli\\data.sqlite3"], env: ["KIRO_DATA_DIR"], hosts: ["codewhisperer.us-east-1.amazonaws.com", "q.eu-central-1.amazonaws.com"] },
  "openai-api": { credential: "key", limitsKey: "provider.limit.admin", provides: ["Today cost", "Last 30 days cost"], kind: "key", env: ["OPENAI_ADMIN_KEY"], hosts: ["api.openai.com"] },
  "poe": { credential: "key", limitsKey: "provider.limit.general", provides: ["Balance"], kind: "key", env: ["POE_API_KEY"], hosts: ["api.poe.com"] },
  "venice": { credential: "key", limitsKey: "provider.limit.general", provides: ["Quota / balance fields returned by this provider"], kind: "key", env: ["VENICE_API_KEY"], hosts: ["api.venice.ai"] },
  "vertexai": { credential: "local", limitsKey: "provider.limit.identity", provides: ["Account identity", "Project"], kind: "local", reads: ["%APPDATA%\\gcloud\\application_default_credentials.json"], env: ["CLOUDSDK_CONFIG", "GOOGLE_CLOUD_PROJECT"], hosts: ["oauth2.googleapis.com"] },
  "warp": { credential: "key", limitsKey: "provider.limit.general", provides: ["Requests", "Bonus credits"], kind: "key", env: ["WARP_API_KEY", "WARP_TOKEN"], hosts: ["app.warp.dev"] },
  "mimo": { credential: "session", limitsKey: "provider.limit.general", provides: ["Monthly tokens"], kind: "key", env: ["MIMO_API_KEY"], hosts: ["platform.xiaomimimo.com", "token-plan-sgp.xiaomimimo.com", "api.xiaomimimo.com"] },
  "trae": { credential: "local", limitsKey: "provider.limit.general", provides: ["Credits", "Plan"], kind: "local", reads: ["%APPDATA%\\Trae\\User\\globalStorage\\storage.json"], hosts: ["grow-normal.trae.ai", "growsg-normal.trae.ai"] },
  "qoder": { credential: "local", limitsKey: "provider.limit.general", provides: ["Credits", "Model-specific packs", "Expiry"], kind: "local", hosts: ["openapi.qoder.sh"], reads: ["%APPDATA%/com.qoder.app.stable/auth.v1.dat", "%APPDATA%/com.qoder.app.stable/Local State"] },
  "zed": { credential: "local", limitsKey: "provider.limit.zed", provides: ["Local tokens (all time / 30 days)", "Recent models", "Login identity only"], kind: "local", reads: ["%LOCALAPPDATA%\\Zed\\threads\\threads.db"] },
  "factory": { credential: "key", limitsKey: "provider.limit.general", provides: ["Extra usage balance", "Usage"], kind: "key", env: ["FACTORY_API_KEY"], hosts: ["api.factory.ai"] },
  "jetbrains": { credential: "local", limitsKey: "provider.limit.cached", provides: ["Cached monthly credits", "Top-up credits", "Subscription expiry"], kind: "local", reads: ["%APPDATA%\\JetBrains\\<IDE>\\options\\AIAssistantQuotaManager2.xml"] },
  "groq": { credential: "key", limitsKey: "provider.limit.general", provides: ["Requests per day (RPD)", "Tokens per minute (TPM)"], kind: "key", env: ["GROQ_API_KEY"], hosts: ["api.groq.com"] },
  "huggingface": { credential: "key", limitsKey: "provider.limit.billing", provides: ["Billable usage", "Included usage", "Requests"], kind: "key", env: ["HF_TOKEN", "HUGGINGFACE_API_KEY", "HUGGING_FACE_HUB_TOKEN"], hosts: ["huggingface.co"] },
  "longcat": { credential: "cookie", limitsKey: "provider.limit.general", provides: ["Token packs", "Usage", "Pending packs"], kind: "key", env: ["LONGCAT_COOKIE"], hosts: ["longcat.chat"] },
  "sub2api": { credential: "key", limitsKey: "provider.limit.general", provides: ["Balance"], kind: "key", reads: ["%APPDATA%\\Pane\\sub2api.json"], env: ["SUB2API_API_KEY"], hosts: ["(your sub2api deployment URL)"] },
  "mistral": { credential: "cookie", limitsKey: "provider.limit.general", provides: ["Monthly tokens", "Vibe usage", "Credits"], kind: "key", env: ["MISTRAL_COOKIE"], hosts: ["admin.mistral.ai"] },
  "perplexity": { credential: "cookie", limitsKey: "provider.limit.general", provides: ["Recurring credits", "Purchased credits", "Bonus credits"], kind: "key", env: ["PERPLEXITY_SESSION_TOKEN", "PERPLEXITY_COOKIE"], hosts: ["www.perplexity.ai"] },
  "volcengine": { credential: "cloud", limitsKey: "provider.limit.general", provides: ["Agent Plan 5h / weekly / monthly", "Coding Plan level windows"], kind: "key", env: ["VOLC_ACCESS_KEY", "VOLC_SECRET_KEY"], hosts: ["open.volcengineapi.com"] },
  "giphy": { credential: "session", limitsKey: "provider.limit.giphy", provides: ["Completed-day calls", "7-day calls", "Beta-derived ceiling"], kind: "key", env: ["GIPHY_PARTNER_TOKEN", "GIPHY_ACCESS_TOKEN"], hosts: ["partner-hub-api.giphy.com"] },
  "codebuddy": { credential: "oauth", limitsKey: "provider.limit.general", provides: ["Credits", "Reserved", "Cycle reset"], kind: "local", reads: ["Pane auth center"], hosts: ["www.codebuddy.ai"] },
  "codebuddycn": { credential: "oauth", limitsKey: "provider.limit.general", provides: ["Credits", "Reserved", "Cycle reset"], kind: "local", reads: ["Pane auth center"], hosts: ["www.codebuddy.cn"] },
  "workbuddy": { credential: "local", limitsKey: "provider.limit.workbuddy", provides: ["Credits", "Reserved", "Resource usage"], kind: "mixed", reads: ["%LOCALAPPDATA%/CodeBuddyExtension/Data/Public/auth/workbuddy-desktop.info", "Explicit Chrome Cookies import or pasted Cookie"], hosts: ["www.workbuddy.cn"] },
  "windsurf": { credential: "oauth", limitsKey: "provider.limit.general", provides: ["Daily", "Weekly", "Overage"], kind: "local", reads: ["Pane auth center"], hosts: ["server.codeium.com"] },
  "mcode": { credential: "local", limitsKey: "provider.limit.ledger", provides: ["Local tokens", "Local spend"], kind: "local", reads: ["~/.minimax/v2/sqlite/runtime-state.sqlite", "~/.cc-switch/cc-switch.db"] },
};
