//! Local spend-source registry.
//!
//! One row per source: where its ledger lives, who reads it, and which
//! provider cards it feeds. `source_statuses()` is generated from this list
//! so a new source is a module + fixture + one registration row instead of
//! another hand-written status line.

use std::path::PathBuf;

use super::{source_status, SpendSourceStatus};

pub mod copilot;

pub struct SourceSpec {
    pub id: &'static str,
    pub tool: &'static str,
    pub form: &'static str,
    pub collector: &'static str,
    pub roots: fn() -> Vec<PathBuf>,
    pub feeds: &'static [&'static str],
    /// Needs credentials/network, so local presence cannot prove access.
    pub runtime: bool,
    /// Parser not verified against a real local sample yet.
    pub needs_probe: bool,
    /// Optional cheap schema/readability probe; `Some(reason)` means the
    /// source exists but cannot be parsed (corrupt DB, missing table, …).
    pub parse_probe: Option<fn() -> Option<String>>,
}

fn home() -> PathBuf {
    dirs::home_dir().unwrap_or_default()
}

fn env_home(var: &str, fallback: &str) -> PathBuf {
    // Share the scanner's quote/whitespace handling so /v1/sources reports
    // the same root the collector actually reads.
    super::env_path(var).unwrap_or_else(|| home().join(fallback))
}

pub fn claude_projects_root() -> PathBuf {
    env_home("CLAUDE_CONFIG_DIR", ".claude").join("projects")
}

pub fn codex_home() -> PathBuf {
    env_home("CODEX_HOME", ".codex")
}

pub fn copilot_db() -> PathBuf {
    home().join(".copilot").join("session-store.db")
}

pub fn copilot_session_state() -> PathBuf {
    home().join(".copilot").join("session-state")
}

fn p(parts: &[&str]) -> PathBuf {
    let mut out = home();
    for part in parts {
        out.push(part);
    }
    out
}

pub fn registry() -> Vec<SourceSpec> {
    vec![
        SourceSpec {
            id: "claude-jsonl",
            tool: "Claude Code",
            form: "JSONL session logs",
            collector: "claude_line",
            roots: || vec![claude_projects_root()],
            feeds: &["claude", "minimax", "aihubmix", "kimi"],
            runtime: false,
            needs_probe: false,
            parse_probe: None,
        },
        SourceSpec {
            id: "codex-jsonl",
            tool: "Codex",
            form: "JSONL / JSONL.ZST rollout logs",
            collector: "codex_line",
            roots: || vec![codex_home().join("sessions"), codex_home().join("archived_sessions")],
            feeds: &["codex", "kimi"],
            runtime: false,
            needs_probe: false,
            parse_probe: None,
        },
        SourceSpec {
            id: "zcode-sqlite",
            tool: "ZCode",
            form: "SQLite + JSONL fallback",
            collector: "zcode_db_data + zcode_line",
            roots: || vec![p(&[".zcode", "cli", "db", "db.sqlite"]), p(&[".zcode", "cli", "rollout"])],
            feeds: &["zcode"],
            runtime: false,
            needs_probe: false,
            parse_probe: None,
        },
        SourceSpec {
            id: "ccswitch-sqlite",
            tool: "cc-switch",
            form: "SQLite proxy + session-import ledger",
            collector: "ccswitch_db_data",
            roots: || vec![p(&[".cc-switch", "cc-switch.db"])],
            feeds: &["claude", "codex", "opencode", "grok", "mcode", "gemini", "claude-desktop"],
            runtime: false,
            needs_probe: false,
            parse_probe: None,
        },
        SourceSpec {
            id: "pi-jsonl",
            tool: "Pi coding agent",
            form: "JSONL session logs",
            collector: "pi_line",
            roots: || vec![super::pi_sessions_dir()],
            feeds: &["claude", "codex"],
            runtime: false,
            needs_probe: false,
            parse_probe: None,
        },
        SourceSpec {
            id: "grok-jsonl",
            tool: "Grok CLI",
            form: "JSONL unified log",
            collector: "grok",
            roots: || vec![env_home("GROK_HOME", ".grok").join("logs").join("unified.jsonl")],
            feeds: &["grok"],
            runtime: false,
            needs_probe: false,
            parse_probe: None,
        },
        SourceSpec {
            id: "opencode-sqlite",
            tool: "OpenCode Desktop / CLI",
            form: "SQLite message ledger",
            collector: "providers::opencode::collect_cost_events",
            roots: || vec![crate::providers::opencode::data_dir().join("opencode.db")],
            feeds: &["opencode", "aihubmix"],
            runtime: false,
            needs_probe: false,
            parse_probe: None,
        },
        SourceSpec {
            id: "qodercn-credits",
            tool: "Qoder CN",
            form: "credits ledger (credits cannot be converted to tokens — shown as a separate credits list)",
            collector: "usage_history::credit_extras",
            roots: || vec![crate::providers::config_dir().join("credit_history.json")],
            feeds: &["qodercn", "traecn"],
            runtime: false,
            needs_probe: false,
            parse_probe: None,
        },
        SourceSpec {
            id: "devin-sqlite",
            tool: "Devin",
            form: "SQLite sessions ledger",
            collector: "providers::devin::collect_usage_events",
            roots: || {
                vec![crate::platform::config_home()
                    .map(|d| d.join("devin").join("cli").join("sessions.db"))
                    .unwrap_or_default()]
            },
            feeds: &["devin"],
            runtime: false,
            needs_probe: false,
            parse_probe: None,
        },
        SourceSpec {
            id: "minimax-sqlite",
            tool: "MiniMax Agent",
            form: "SQLite token_usage ledger",
            collector: "providers::minimax::collect_usage_events",
            roots: || {
                vec![
                    p(&[".minimax", "sqlite.db"]),
                    p(&[".minimax", "v2", "sqlite", "runtime-state.sqlite"]),
                ]
            },
            feeds: &["minimax", "mcode"],
            runtime: false,
            needs_probe: false,
            parse_probe: None,
        },
        SourceSpec {
            id: "dsh-jsonl-zstd",
            tool: "DeepSeek Harness",
            form: "multi-frame zstd session logs",
            collector: "spend::dsh",
            roots: || vec![env_home("DSH_HOME", ".dsh").join("sessions")],
            feeds: &["dsh"],
            runtime: false,
            needs_probe: false,
            parse_probe: None,
        },
        SourceSpec {
            id: "hermes-sqlite",
            tool: "Hermes Desktop",
            form: "SQLite session_model_usage ledger",
            collector: "providers::hermes::collect_usage_events",
            roots: || vec![dirs::data_local_dir().unwrap_or_default().join("hermes").join("state.db")],
            feeds: &["hermes", "minimax", "openrouter"],
            runtime: false,
            needs_probe: false,
            parse_probe: None,
        },
        SourceSpec {
            id: "qwen-jsonl",
            tool: "Qwen Code",
            form: "JSONL token ledger",
            collector: "qwen_line",
            roots: || vec![p(&[".qwen", "usage"])],
            feeds: &["qwen"],
            runtime: false,
            needs_probe: false,
            parse_probe: None,
        },
        SourceSpec {
            id: "kimi-jsonl",
            tool: "Kimi Code",
            form: "JSONL wire ledger",
            collector: "kimi_line",
            roots: || vec![crate::providers::kimi::code_home().join("sessions")],
            feeds: &["kimi"],
            runtime: false,
            needs_probe: false,
            parse_probe: None,
        },
        SourceSpec {
            id: "antigravity-sqlite",
            tool: "Antigravity IDE / CLI",
            form: "SQLite protobuf conversation ledger",
            collector: "decode_antigravity_call",
            roots: || {
                vec![
                    p(&[".gemini", "antigravity", "conversations"]),
                    p(&[".gemini", "antigravity-cli", "conversations"]),
                ]
            },
            feeds: &["antigravity"],
            runtime: false,
            needs_probe: false,
            parse_probe: None,
        },
        SourceSpec {
            id: "workbuddy-jsonl",
            tool: "WorkBuddy (CN)",
            form: "JSONL session logs",
            collector: "workbuddy_line",
            roots: || vec![p(&[".workbuddy", "projects"])],
            feeds: &["workbuddy"],
            runtime: false,
            needs_probe: false,
            parse_probe: None,
        },
        SourceSpec {
            id: "workbuddy-ai-jsonl",
            tool: "WorkBuddy AI",
            form: "JSONL session logs",
            collector: "workbuddy_line",
            roots: || vec![p(&[".workbuddy-ai", "projects"]), p(&[".workbuddy-ai", "sessions"])],
            feeds: &["workbuddyai"],
            runtime: false,
            needs_probe: false,
            parse_probe: None,
        },
        SourceSpec {
            id: "codebuddy-cli-jsonl",
            tool: "CodeBuddy CLI",
            form: "JSONL session logs",
            collector: "workbuddy_line",
            roots: || vec![p(&[".codebuddy", "projects"])],
            feeds: &["codebuddy"],
            runtime: false,
            needs_probe: false,
            parse_probe: None,
        },
        SourceSpec {
            id: "codebuddy-ide-index",
            tool: "CodeBuddy IDE",
            form: "extension history index",
            collector: "codebuddy_ide_data",
            roots: || vec![super::codebuddy_ide_root()],
            feeds: &["codebuddy"],
            runtime: false,
            needs_probe: false,
            parse_probe: None,
        },
        SourceSpec {
            id: "gemini-jsonl",
            tool: "Gemini CLI",
            form: "JSON/JSONL chat logs",
            collector: "gemini",
            roots: || vec![p(&[".gemini", "tmp"])],
            feeds: &["gemini"],
            runtime: false,
            // The flag existed because the parser was written from the upstream
            // session schema alone. It now has a fixture under
            // parse-tests/fixtures/gemini plus parse tests, so it resolves
            // against the filesystem like every other source (#42-2).
            needs_probe: false,
            parse_probe: None,
        },
        SourceSpec {
            id: "copilot-local",
            tool: "GitHub Copilot CLI",
            form: "session-store.db assistant_usage_events (+ events.jsonl fallback)",
            collector: "spend::sources::copilot",
            roots: || vec![copilot_db(), copilot_session_state()],
            feeds: &["copilot"],
            runtime: false,
            needs_probe: false,
            parse_probe: Some(copilot::probe_error),
        },
        SourceSpec {
            id: "cursor-csv",
            tool: "Cursor IDE",
            form: "authenticated usage CSV",
            collector: "providers::cursor::fetch_usage_csv",
            roots: Vec::new,
            feeds: &["cursor"],
            runtime: true,
            needs_probe: false,
            parse_probe: None,
        },
    ]
}

fn status_of(spec: &SourceSpec) -> SpendSourceStatus {
    let feeds = || spec.feeds.iter().map(|s| (*s).into()).collect();
    if spec.runtime {
        return SpendSourceStatus {
            id: spec.id.into(),
            tool: spec.tool.into(),
            form: spec.form.into(),
            collector: spec.collector.into(),
            state: "runtime".into(),
            root: String::new(),
            reason: None,
            feeds: feeds(),
        };
    }
    let existing_root = || {
        (spec.roots)()
            .into_iter()
            .find(|p| p.exists())
            .map(|p| p.display().to_string())
            .unwrap_or_default()
    };
    if spec.needs_probe {
        return SpendSourceStatus {
            id: spec.id.into(),
            tool: spec.tool.into(),
            form: spec.form.into(),
            collector: spec.collector.into(),
            state: "needs_probe".into(),
            root: existing_root(),
            reason: Some(
                "parser built from the upstream session schema; waiting for a verified local sample"
                    .into(),
            ),
            feeds: feeds(),
        };
    }
    if let Some(reason) = spec.parse_probe.and_then(|probe| probe()) {
        return SpendSourceStatus {
            id: spec.id.into(),
            tool: spec.tool.into(),
            form: spec.form.into(),
            collector: spec.collector.into(),
            state: "parse_error".into(),
            root: existing_root(),
            reason: Some(reason),
            feeds: feeds(),
        };
    }
    source_status(
        spec.id,
        spec.tool,
        spec.form,
        spec.collector,
        &(spec.roots)(),
        spec.feeds,
    )
}

pub fn statuses() -> Vec<SpendSourceStatus> {
    registry().iter().map(status_of).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corrupt_probe() -> Option<String> {
        Some("assistant_usage_events: no such table".into())
    }

    #[test]
    fn parse_probe_marks_a_present_but_unparseable_source() {
        let spec = SourceSpec {
            id: "probe",
            tool: "Probe",
            form: "db",
            collector: "probe",
            roots: || Vec::new(),
            feeds: &["probe"],
            runtime: false,
            needs_probe: false,
            parse_probe: Some(corrupt_probe),
        };
        let status = status_of(&spec);
        assert_eq!(status.state, "parse_error");
        assert!(status.reason.as_deref().unwrap_or("").contains("no such table"));
        assert_eq!(status.feeds, vec!["probe".to_string()]);
    }
}
