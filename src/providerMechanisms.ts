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
}

export const MECHANISMS: Record<string, Mechanism> = {
  "claude": { kind: "local", reads: [".claude.json", ".credentials.json"], hosts: ["platform.claude.com", "api.anthropic.com"] },
  "codex": { kind: "local", reads: ["auth.json"], hosts: ["api.openai.com", "auth.openai.com", "chatgpt.com"] },
  "cursor": { kind: "local", reads: ["state.vscdb"], hosts: ["cursor.com", "api2.cursor.sh"] },
  "opencode": { kind: "mixed", reads: ["auth.json", "opencode.db"], env: ["OPENCODE_GO_API_KEY"], hosts: ["opencode.ai"] },
  "copilot": { kind: "local", reads: ["apps.json", "hosts.json"], hosts: ["api.github.com"] },
  "grok": { kind: "local", reads: ["auth.json"], hosts: ["grok.com", "auth.x.ai", "cli-chat-proxy.grok.com"] },
  "devin": { kind: "local", reads: ["sessions.db"], hosts: ["server.codeium.com"] },
  "minimax": { kind: "mixed", reads: ["sqlite.db"], env: ["MINIMAX_API_KEY"], hosts: ["api.minimax.io", "api.minimaxi.com"] },
  "openrouter": { kind: "key", env: ["OPENROUTER_API_KEY"], hosts: ["openrouter.ai"] },
  "zai": { kind: "mixed", reads: ["key.json"], env: ["ZAI_API_KEY", "GLM_API_KEY"], hosts: ["api.z.ai", "open.bigmodel.cn"] },
  "antigravity": { kind: "local", reads: ["antigravity-token.json"], hosts: ["daily-cloudcode-pa.googleapis.com", "cloudcode-pa.googleapis.com", "oauth2.googleapis.com"] },
  "deepseek": { kind: "key", env: ["DEEPSEEK_API_KEY"], hosts: ["api.deepseek.com"] },
  "moonshot": { kind: "key", env: ["MOONSHOT_API_KEY", "KIMI_API_KEY"], hosts: ["api.moonshot.ai", "api.moonshot.cn"] },
  "elevenlabs": { kind: "key", env: ["ELEVENLABS_API_KEY", "XI_API_KEY"], hosts: ["api.elevenlabs.io"] },
  "ollama": { kind: "local", hosts: ["127.0.0.1:11434"] },
  "codebuff": { kind: "mixed", reads: ["credentials.json"], env: ["CODEBUFF_API_KEY"], hosts: ["www.codebuff.com"] },
  "kilo": { kind: "mixed", reads: ["auth.json"], env: ["KILO_API_KEY"], hosts: ["app.kilo.ai"] },
  "aihubmix": { kind: "key", env: ["AIHUBMIX_API_KEY"], hosts: ["aihubmix.com"] },
  "onenewapi": { kind: "local" },
  "qwen": { kind: "mixed", env: ["BAILIAN_TOKEN_PLAN_API_KEY", "DASHSCOPE_API_KEY"], hosts: ["modelstudio.console.alibabacloud.com", "bailian.console.aliyun.com", "coding-intl.dashscope.aliyuncs.com"] },
  "hermes": { kind: "local", reads: ["state.db"], hosts: ["aihubmix.com", "api.minimax.io", "openrouter.ai"] },
  "kimi": { kind: "mixed", reads: ["kimi-code.json", "kimi_probe_log.json"], env: ["KIMI_CODING_API_KEY"], hosts: ["api.kimi.com", "auth.kimi.com", "www.kimi.ai"] },
  "stepfun": { kind: "key", env: ["STEPFUN_API_KEY"], hosts: ["api.stepfun.com", "api.stepfun.ai"] },
  "stepfun-plan": { kind: "local", reads: ["stepfun-plan.json"], hosts: ["platform.stepfun.com"] },
  "shandianshuo": { kind: "local", reads: ["config.json"], hosts: ["api.shandianshuo.cn"] },
  "siliconflow": { kind: "key", env: ["SILICONFLOW_API_KEY"], hosts: ["api.siliconflow.cn", "api.siliconflow.com"] },
  "novita": { kind: "key", env: ["NOVITA_API_KEY"], hosts: ["api.novita.ai"] },
  "relaybalance": { kind: "key", hosts: ["（你自填的中转站 base URL）"] },
  "qodercn": { kind: "local", reads: ["auth.v1.dat", "qoder_checkin.json"], hosts: ["openapi.qoder.com.cn"] },
  "traecn": { kind: "local", reads: ["storage.json"], hosts: ["api.trae.cn"] },
  "commandcode": { kind: "mixed", reads: ["auth.json"], env: ["COMMAND_CODE_API_KEY"], hosts: ["api.commandcode.ai"] },
  "doubao": { kind: "local", reads: ["doubao_cookies.json", "doubao_cookies_snapshot.db"], hosts: ["www.doubao.com"] },
  "clawsgo": { kind: "key", env: ["CLAWSGO_TOKEN"], hosts: ["api.clawsgo.ai"] },
  "bocha": { kind: "key", reads: ["%APPDATA%\\Pane\\config.json"], env: ["BOCHA_API_KEY"], hosts: ["api.bochaai.com"] },
  "tavily": { kind: "key", reads: ["%APPDATA%\\Pane\\tavily-keys.json"], env: ["TAVILY_API_KEY"], hosts: ["api.tavily.com"] },
  "firecrawl": { kind: "key", reads: ["~/.zcode/cli/config.json (旧 MCP 配置)"], env: ["FIRECRAWL_API_KEY", "FIRECRAWL_FIRECRAWL_API_KEY"], hosts: ["api.firecrawl.dev"] },
  "clinepass": { kind: "key", env: ["CLINE_API_KEY"], hosts: ["api.cline.bot"] },
  "sensenova": { kind: "local", reads: ["sensenova.json"], hosts: ["platform.sensenova.cn"] },
  "apigoto": { kind: "key", env: ["APIGOTO_API_KEY"], hosts: ["api.apigoto.com"] },
  "brave": { kind: "key", env: ["BRAVE_API_KEY", "BRAVE_SEARCH_API_KEY"], hosts: ["api.search.brave.com"] },
  "amp": { kind: "key", env: ["AMP_API_KEY"], hosts: ["ampcode.com"] },
  "bedrock": { kind: "local", env: ["AWS_ACCESS_KEY_ID", "AWS_SECRET_ACCESS_KEY", "AWS_SESSION_TOKEN", "AWS_PROFILE"], hosts: ["ce.us-east-1.amazonaws.com"] },
  "chutes": { kind: "key", env: ["CHUTES_API_KEY"], hosts: ["api.chutes.ai"] },
  "deepgram": { kind: "key", env: ["DEEPGRAM_API_KEY"], hosts: ["api.deepgram.com"] },
  "kiro": { kind: "local", reads: ["%LOCALAPPDATA%\\kiro-cli\\data.sqlite3", "%APPDATA%\\kiro-cli\\data.sqlite3"], env: ["KIRO_DATA_DIR"], hosts: ["codewhisperer.us-east-1.amazonaws.com", "q.eu-central-1.amazonaws.com"] },
  "openai-api": { kind: "key", env: ["OPENAI_ADMIN_KEY"], hosts: ["api.openai.com"] },
  "poe": { kind: "key", env: ["POE_API_KEY"], hosts: ["api.poe.com"] },
  "venice": { kind: "key", env: ["VENICE_API_KEY"], hosts: ["api.venice.ai"] },
  "vertexai": { kind: "local", reads: ["%APPDATA%\\gcloud\\application_default_credentials.json"], env: ["CLOUDSDK_CONFIG", "GOOGLE_CLOUD_PROJECT"], hosts: ["oauth2.googleapis.com"] },
  "warp": { kind: "key", env: ["WARP_API_KEY", "WARP_TOKEN"], hosts: ["app.warp.dev"] },
  "mimo": { kind: "key", env: ["MIMO_API_KEY"], hosts: ["platform.xiaomimimo.com", "token-plan-sgp.xiaomimimo.com", "api.xiaomimimo.com"] },
  "trae": { kind: "local", reads: ["%APPDATA%\\Trae\\User\\globalStorage\\storage.json"], hosts: ["grow-normal.trae.ai", "growsg-normal.trae.ai"] },
  "qoder": { kind: "local", reads: ["%APPDATA%\\com.qoder.app.stable\\auth.v1.dat"], hosts: ["openapi.qoder.sh"] },
  "zed": { kind: "local", reads: ["%LOCALAPPDATA%\\Zed\\threads\\threads.db"] },
  "factory": { kind: "key", env: ["FACTORY_API_KEY"], hosts: ["api.factory.ai"] },
  "jetbrains": { kind: "local", reads: ["%APPDATA%\\JetBrains\\<IDE>\\options\\AIAssistantQuotaManager2.xml"] },
  "groq": { kind: "key", env: ["GROQ_API_KEY"], hosts: ["api.groq.com"] },
  "huggingface": { kind: "key", env: ["HF_TOKEN", "HUGGINGFACE_API_KEY", "HUGGING_FACE_HUB_TOKEN"], hosts: ["huggingface.co"] },
  "longcat": { kind: "key", env: ["LONGCAT_COOKIE"], hosts: ["longcat.chat"] },
  "sub2api": { kind: "key", reads: ["%APPDATA%\\Pane\\sub2api.json"], env: ["SUB2API_API_KEY"], hosts: ["(your sub2api deployment URL)"] },
  "mistral": { kind: "key", env: ["MISTRAL_COOKIE"], hosts: ["admin.mistral.ai"] },
  "perplexity": { kind: "key", env: ["PERPLEXITY_SESSION_TOKEN", "PERPLEXITY_COOKIE"], hosts: ["www.perplexity.ai"] },
};
