//! Read-only local source detection for onboarding and diagnostics.
//!
//! This module never executes a CLI and never opens a network connection. It
//! answers three separate questions per family: is a tool installed, is a
//! credential present, and where did that credential come from. Environment
//! variables are read from the process plus the Windows user/system
//! environment (a `setx` in another terminal must be visible after a rescan);
//! only names and presence are exposed, never values.

use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceProbe {
    pub family: String,
    pub display_name: String,
    /// installed | missing | unknown
    pub install_state: &'static str,
    pub install_evidence: Vec<String>,
    /// present | absent
    pub credential_state: &'static str,
    /// env:NAME | keyvault | local:<hint> | none
    pub credential_source: String,
    pub env_vars: Vec<String>,
    pub local_hint: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvKeyProbe {
    pub name: String,
    pub family: Option<String>,
    /// False: the key was found but Pane has no quota reader for it yet.
    pub supported: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceProbeReport {
    pub families: Vec<SourceProbe>,
    pub env_keys: Vec<EnvKeyProbe>,
}

struct InstallHint {
    family: &'static str,
    executables: &'static [&'static str],
    dirs: &'static [&'static str],
}

/// Executable/directory hints for the tools users actually install. Families
/// without a hint still get environment/stored-key detection; their install
/// state is reported as `unknown`, never as "missing".
const HINTS: &[InstallHint] = &[
    InstallHint { family: "claude", executables: &["claude"], dirs: &["~/.claude", "%APPDATA%/Claude"] },
    InstallHint { family: "codex", executables: &["codex"], dirs: &["~/.codex", "%APPDATA%/Codex"] },
    InstallHint { family: "opencode", executables: &["opencode"], dirs: &["~/.opencode", "~/.local/share/opencode"] },
    InstallHint { family: "gemini", executables: &["gemini"], dirs: &["~/.gemini"] },
    InstallHint { family: "qwen", executables: &["qwen"], dirs: &["~/.qwen"] },
    InstallHint { family: "kimi", executables: &["kimi"], dirs: &["~/.kimi"] },
    InstallHint { family: "grok", executables: &["grok"], dirs: &["~/.grok"] },
    InstallHint { family: "copilot", executables: &["copilot"], dirs: &["%LOCALAPPDATA%/github-copilot", "~/.config/github-copilot"] },
    InstallHint { family: "cursor", executables: &["cursor"], dirs: &["%LOCALAPPDATA%/Programs/cursor", "~/.cursor"] },
    InstallHint { family: "windsurf", executables: &["windsurf"], dirs: &["%LOCALAPPDATA%/Programs/Windsurf"] },
    InstallHint { family: "trae", executables: &["trae"], dirs: &["%LOCALAPPDATA%/Programs/Trae", "%LOCALAPPDATA%/Programs/Trae CN"] },
    InstallHint { family: "zed", executables: &["zed"], dirs: &["%LOCALAPPDATA%/Programs/Zed"] },
    InstallHint { family: "kiro", executables: &["kiro"], dirs: &["%LOCALAPPDATA%/Programs/Kiro"] },
    InstallHint { family: "qoder", executables: &["qoder"], dirs: &["%LOCALAPPDATA%/Programs/Qoder", "~/.qoder"] },
    InstallHint { family: "antigravity", executables: &["antigravity"], dirs: &["%LOCALAPPDATA%/Programs/Antigravity"] },
    InstallHint { family: "commandcode", executables: &["commandcode"], dirs: &["~/.commandcode"] },
    InstallHint { family: "amp", executables: &["amp"], dirs: &["~/.amp"] },
    InstallHint { family: "clinepass", executables: &["cline"], dirs: &["~/.cline"] },
];

/// Generic keys that are useful evidence but have no quota reader in Pane.
/// They are reported as found-but-unsupported instead of being hidden.
const COMMON_ENV_KEYS: &[(&str, Option<&str>)] = &[
    ("ANTHROPIC_API_KEY", Some("claude")),
    ("OPENAI_API_KEY", Some("codex")),
    ("OPENAI_BASE_URL", None),
    ("OPENAI_ORG_ID", None),
    ("GEMINI_API_KEY", Some("gemini")),
    ("GOOGLE_API_KEY", Some("gemini")),
    ("AZURE_OPENAI_API_KEY", None),
    ("AZURE_OPENAI_ENDPOINT", None),
];

const MAX_EVIDENCE: usize = 4;

/// All families that can be detected. Family metadata comes from the catalog;
/// the env/credential probes remain available even when install state is
/// unknown.
pub fn probe_all() -> SourceProbeReport {
    let env = env_snapshot();
    let definitions = crate::provider_catalog::provider_definitions();
    let mut families = Vec::new();
    for def in definitions {
        if !HINTS.iter().any(|hint| hint.family == def.family_id)
            && crate::provider_catalog::provider_env_vars(def.family_id).is_empty()
            && local_hint(def.family_id).is_none()
        {
            continue;
        }
        families.push(probe_family_with_env(def.family_id, def.display_name, &env));
    }
    families.sort_by(|a, b| {
        let rank = |p: &SourceProbe| {
            if p.install_state == "installed" || p.credential_state == "present" { 0 } else { 1 }
        };
        rank(a).cmp(&rank(b)).then_with(|| a.display_name.cmp(&b.display_name))
    });
    let mut env_keys: Vec<EnvKeyProbe> = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for (name, family) in COMMON_ENV_KEYS {
        if env_value(&env, name).is_some() && seen.insert((*name).to_string()) {
            env_keys.push(EnvKeyProbe {
                name: (*name).to_string(),
                family: family.map(str::to_string),
                supported: false,
            });
        }
    }
    for def in definitions {
        for name in crate::provider_catalog::provider_env_vars(def.family_id) {
            if env_value(&env, name).is_some() && seen.insert((*name).to_string()) {
                env_keys.push(EnvKeyProbe {
                    name: (*name).to_string(),
                    family: Some(def.family_id.to_string()),
                    supported: true,
                });
            }
        }
    }
    SourceProbeReport { families, env_keys }
}

/// Probe one family for the HTTP API and the credential-status command.
pub fn probe_family(family: &str) -> SourceProbe {
    let env = env_snapshot();
    let display_name = crate::provider_catalog::provider_definitions()
        .iter()
        .find(|def| def.family_id == family)
        .map(|def| def.display_name)
        .unwrap_or(family);
    probe_family_with_env(family, display_name, &env)
}

fn probe_family_with_env(family: &str, display_name: &str, env: &HashMap<String, String>) -> SourceProbe {
    let hint = HINTS.iter().find(|hint| hint.family == family);
    let mut evidence = Vec::new();
    if let Some(hint) = hint {
        evidence.extend(executable_evidence(hint.executables, env));
        evidence.extend(dir_evidence(hint.dirs, env));
    }
    evidence.sort();
    evidence.dedup();
    evidence.truncate(MAX_EVIDENCE);
    let install_state = match hint {
        None => "unknown",
        Some(_) if !evidence.is_empty() => "installed",
        Some(_) => "missing",
    };
    let env_vars: Vec<String> = crate::provider_catalog::provider_env_vars(family)
        .iter()
        .filter(|name| env_value(env, name).is_some())
        .map(|name| (*name).to_string())
        .collect();
    let local = local_hint(family);
    let stored = stored_credential_exists(family);
    let (credential_state, credential_source) = if let Some(name) = env_vars.first() {
        ("present", format!("env:{name}"))
    } else if stored {
        ("present", "keyvault".to_string())
    } else if local.is_some() {
        ("present", "local".to_string())
    } else {
        ("absent", "none".to_string())
    };
    SourceProbe {
        family: family.to_string(),
        display_name: display_name.to_string(),
        install_state,
        install_evidence: evidence,
        credential_state,
        credential_source,
        env_vars,
        local_hint: local,
    }
}

/// Same store order as `get_credential_status`: login accounts, then the
/// multi-account list, then the saved single key. Checking only the key file
/// made account-based families (ClinePass, APIGOTO, …) look credential-less.
fn stored_credential_exists(family: &str) -> bool {
    if crate::login_accounts::takes_login_accounts(family) {
        !crate::login_accounts::load(family).is_empty()
    } else if crate::accounts::provider_takes_accounts(family) {
        !crate::accounts::load_accounts(family).is_empty()
    } else {
        crate::providers::stored_key_file(family).is_some()
    }
}

fn local_hint(family: &str) -> Option<String> {
    match family {
        "claude" => crate::providers::claude::local_credential_hint(),
        "codex" => crate::providers::codex::local_credential_hint(),
        "opencode" => crate::providers::opencode::local_credential_hint(),
        "cursor" => crate::providers::cursor::local_credential_hint(),
        "copilot" => crate::providers::copilot::local_credential_hint(),
        "grok" => crate::providers::grok::local_credential_hint(),
        "devin" => crate::providers::devin::local_credential_hint(),
        "antigravity" => crate::providers::antigravity::local_credential_hint(),
        "qoder" => crate::providers::qoder::local_credential_hint(),
        "kiro" => crate::providers::kiro::local_credential_hint(),
        "trae" => crate::providers::trae::local_credential_hint(),
        "zed" => crate::providers::zed::local_credential_hint(),
        "kimi" => crate::providers::kimi::local_credential_hint(),
        "clinepass" => crate::providers::clinepass::local_credential_hint(),
        "amp" => crate::providers::amp::local_credential_hint(),
        "commandcode" => crate::providers::commandcode::local_credential_hint(),
        "doubao" => crate::providers::doubao::local_credential_hint(),
        "clawsgo" => crate::providers::clawsgo::local_credential_hint(),
        "gemini" => None,
        "qwen" => crate::providers::qwen::local_credential_hint(),
        "windsurf" => crate::providers::windsurf::local_credential_hint(),
        _ => None,
    }
}

fn env_value<'a>(env: &'a HashMap<String, String>, name: &str) -> Option<&'a str> {
    env.get(name).map(|value| value.trim()).filter(|value| !value.is_empty())
}

fn executable_evidence(names: &[&str], env: &HashMap<String, String>) -> Vec<String> {
    let path = env.get("PATH").map(String::as_str).unwrap_or("");
    let dirs = split_paths(path);
    let exts = if cfg!(windows) {
        env.get("PATHEXT")
            .map(|value| value.split(';').map(str::to_string).collect::<Vec<_>>())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| vec![".COM".into(), ".EXE".into(), ".BAT".into(), ".CMD".into()])
    } else {
        vec![String::new()]
    };
    let mut out = Vec::new();
    for name in names {
        for dir in &dirs {
            let base = Path::new(dir).join(name);
            for ext in &exts {
                let candidate = if ext.is_empty() {
                    base.clone()
                } else {
                    PathBuf::from(format!("{}{}", base.display(), ext.to_lowercase()))
                };
                if candidate.is_file() {
                    out.push(display_path(&candidate));
                }
            }
            if !cfg!(windows) && base.is_file() {
                out.push(display_path(&base));
            }
        }
    }
    out
}

fn dir_evidence(dirs: &[&str], env: &HashMap<String, String>) -> Vec<String> {
    dirs.iter()
        .filter_map(|dir| expand_path(dir, env))
        .filter(|path| path.is_dir())
        .map(|path| display_path(&path))
        .collect()
}

fn split_paths(path: &str) -> Vec<String> {
    if path.is_empty() {
        return Vec::new();
    }
    std::env::split_paths(path).map(|p| p.to_string_lossy().to_string()).collect()
}

fn expand_path(raw: &str, env: &HashMap<String, String>) -> Option<PathBuf> {
    let mut value = raw.trim().to_string();
    if value.is_empty() {
        return None;
    }
    if let Some(rest) = value.strip_prefix("~/") {
        let home = env.get("USERPROFILE").or_else(|| env.get("HOME"))?;
        value = format!("{home}/{rest}");
    }
    // Expand %VAR% (Windows) or $VAR / ${VAR} (POSIX) from the snapshot.
    let mut expanded = String::new();
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '%' {
            let mut name = String::new();
            let mut closed = false;
            for next in chars.by_ref() {
                if next == '%' {
                    closed = true;
                    break;
                }
                name.push(next);
            }
            if closed {
                if let Some(found) = env.get(&name) {
                    expanded.push_str(found);
                } else {
                    expanded.push('%');
                    expanded.push_str(&name);
                    expanded.push('%');
                }
            } else {
                expanded.push('%');
                expanded.push_str(&name);
            }
        } else if ch == '$' {
            let mut name = String::new();
            let mut closed = false;
            if chars.peek() == Some(&'{') {
                chars.next();
                for next in chars.by_ref() {
                    if next == '}' {
                        closed = true;
                        break;
                    }
                    name.push(next);
                }
            } else {
                while let Some(next) = chars.peek().copied() {
                    if next.is_ascii_alphanumeric() || next == '_' {
                        name.push(next);
                        chars.next();
                    } else {
                        break;
                    }
                }
            }
            if !name.is_empty() {
                if let Some(found) = env.get(&name) {
                    expanded.push_str(found);
                } else {
                    expanded.push('$');
                    if closed {
                        expanded.push('{');
                        expanded.push_str(&name);
                        expanded.push('}');
                    } else {
                        expanded.push_str(&name);
                    }
                }
            } else {
                expanded.push('$');
            }
        } else {
            expanded.push(ch);
        }
    }
    let path = PathBuf::from(expanded);
    Some(if path.is_absolute() {
        path
    } else {
        // Relative hints are relative to the user's home, never the process
        // working directory (which can be anywhere for a tray app).
        let home = env.get("USERPROFILE").or_else(|| env.get("HOME"))?;
        Path::new(home).join(path)
    })
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn env_snapshot() -> HashMap<String, String> {
    let mut map: HashMap<String, String> = std::env::vars().collect();
    #[cfg(windows)]
    read_windows_environment(&mut map);
    map
}

#[cfg(windows)]
fn read_windows_environment(map: &mut HashMap<String, String>) {
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    use winreg::RegKey;
    let sources = [
        (HKEY_CURRENT_USER, "Environment"),
        (
            HKEY_LOCAL_MACHINE,
            r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment",
        ),
    ];
    for (root, path) in sources {
        let Ok(key) = RegKey::predef(root).open_subkey(path) else { continue };
        for item in key.enum_values().flatten() {
            let (name, value) = item;
            map.entry(name).or_insert(value.to_string());
        }
    }
    // A user PATH entry added by setx is not in the running process; merge it
    // so executable evidence follows the same freshness as environment keys.
    let mut merged: BTreeMap<String, String> = map
        .iter()
        .map(|(name, value)| (name.to_lowercase(), value.clone()))
        .collect();
    if let Ok(user) = RegKey::predef(HKEY_CURRENT_USER).open_subkey("Environment") {
        if let Ok(user_path) = user.get_value::<String, _>("Path") {
            let process_path = map.get("PATH").cloned().unwrap_or_default();
            let joined = if process_path.is_empty() {
                user_path
            } else {
                format!("{process_path};{user_path}")
            };
            merged.insert("path".into(), joined);
        }
    }
    for (name, value) in merged {
        if name == "path" {
            map.insert("PATH".into(), value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs.iter().map(|(k, v)| ((*k).to_string(), (*v).to_string())).collect()
    }

    #[test]
    fn expands_windows_and_posix_variables_without_touching_unknown_names() {
        let snapshot = env(&[("USERPROFILE", r"C:\Users\me"), ("APPDATA", r"C:\Users\me\AppData\Roaming")]);
        assert_eq!(
            expand_path("%APPDATA%/Claude", &snapshot).unwrap(),
            PathBuf::from(r"C:\Users\me\AppData\Roaming/Claude")
        );
        assert_eq!(
            expand_path("~/.claude", &snapshot).unwrap(),
            PathBuf::from(r"C:\Users\me/.claude")
        );
        // Unknown names are preserved literally; the hint is still relative,
        // so it resolves against the home directory like any other hint.
        assert_eq!(
            expand_path("$HOME/.config", &snapshot).unwrap(),
            PathBuf::from(r"C:\Users\me").join("$HOME/.config")
        );
    }

    #[test]
    fn empty_or_unset_paths_are_not_evidence() {
        let snapshot = env(&[("PATH", "")]);
        assert!(executable_evidence(&["claude"], &snapshot).is_empty());
        assert!(dir_evidence(&["%MISSING%/x"], &snapshot).is_empty());
    }
}
