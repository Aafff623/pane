//! RTK command-output savings: the third Total Spend metric.
//!
//! Global windows come from `rtk gain --all --format json` (60 s cache);
//! project attribution comes from a copied `%LOCALAPPDATA%\rtk\history.db`
//! (plus WAL/SHM). Pane also keeps a small daily snapshot so its 30-day
//! sparkline survives RTK's own history retention.
//!
//! `saved_tokens` is RTK's byte/4 *estimate*, never a vendor bill and never
//! a quota. Nothing here feeds `spend.rs`, the donut, or `usage_history`.

use chrono::{DateTime, Local, NaiveDate};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const CACHE_TTL: Duration = Duration::from_secs(60);
const CLI_TIMEOUT: Duration = Duration::from_secs(10);
const INSTALL_TIMEOUT: Duration = Duration::from_secs(240);
const INTEGRATION_TIMEOUT: Duration = Duration::from_secs(120);
const TOP_PROJECTS: usize = 5;
const KEEP_DAYS: i64 = 400;
const HISTORY_VERSION: u32 = 1;
static COPY_SEQ: AtomicU64 = AtomicU64::new(0);
static CACHE: tokio::sync::Mutex<Option<(Instant, RtkSavings)>> =
    tokio::sync::Mutex::const_new(None);

struct IntegrationSpec {
    id: &'static str,
    label: &'static str,
    icon_key: &'static str,
    /// `rtk init --agent <id>` exists. When false the harness can still be
    /// connected by hand (its own hook config), so it is detectable but must
    /// never be installed through the CLI — calling a flag that does not exist
    /// would look like RTK refusing rather than RTK not supporting it.
    cli: bool,
}

const RTK_INTEGRATIONS: &[IntegrationSpec] = &[
    IntegrationSpec {
        id: "codex",
        label: "Codex",
        icon_key: "codex",
        cli: true,
    },
    IntegrationSpec {
        id: "claude",
        label: "Claude Code",
        icon_key: "claude",
        cli: true,
    },
    IntegrationSpec {
        id: "cursor",
        label: "Cursor",
        icon_key: "cursor",
        cli: true,
    },
    IntegrationSpec {
        id: "trae",
        label: "Trae",
        icon_key: "traecn",
        cli: true,
    },
    IntegrationSpec {
        id: "windsurf",
        label: "Windsurf",
        icon_key: "windsurf",
        cli: true,
    },
    IntegrationSpec {
        id: "cline",
        label: "Cline",
        icon_key: "clinepass",
        cli: true,
    },
    IntegrationSpec {
        id: "kilocode",
        label: "Kilo Code",
        icon_key: "kilo",
        cli: true,
    },
    IntegrationSpec {
        id: "antigravity",
        label: "Antigravity",
        icon_key: "antigravity",
        cli: true,
    },
    IntegrationSpec {
        id: "kimi",
        label: "Kimi Code",
        icon_key: "kimi",
        cli: true,
    },
    IntegrationSpec {
        id: "copilot",
        label: "GitHub Copilot",
        icon_key: "copilot",
        cli: true,
    },
    IntegrationSpec {
        id: "gemini",
        label: "Gemini CLI",
        icon_key: "gemini",
        cli: true,
    },
    IntegrationSpec {
        id: "opencode",
        label: "OpenCode",
        icon_key: "opencode",
        cli: true,
    },
    IntegrationSpec {
        id: "droid",
        label: "Factory Droid",
        icon_key: "factory",
        cli: true,
    },
    IntegrationSpec {
        id: "vibe",
        label: "Mistral Vibe",
        icon_key: "mistral",
        cli: true,
    },
    IntegrationSpec {
        id: "hermes",
        label: "Hermes",
        icon_key: "hermes",
        cli: true,
    },
    IntegrationSpec {
        id: "pi",
        label: "Pi",
        icon_key: "pi",
        cli: true,
    },
    IntegrationSpec {
        id: "omp",
        label: "Oh My Pi",
        icon_key: "omp",
        cli: true,
    },
    // RTK has no `--agent qoder`, but Qoder CN reads Claude-compatible hooks,
    // so it can be wired by hand — and on this machine it already is. Without
    // this entry the panel told a correctly-connected user that nothing was
    // detected (#RTK).
    IntegrationSpec {
        id: "qodercn",
        label: "Qoder CN",
        icon_key: "qoder-cn",
        cli: false,
    },
];

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RtkIntegrationTarget {
    pub id: &'static str,
    pub label: &'static str,
    pub icon_key: &'static str,
    /// global | project
    pub scope: &'static str,
    pub detected: bool,
    /// exact | fuzzy | none
    pub match_kind: &'static str,
    /// True when this harness has no `rtk init --agent` flag and must be wired
    /// through its own hook config. The UI offers instructions, not a button
    /// that would fail.
    pub manual: bool,
}

#[derive(Serialize, Clone, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RtkSavings {
    /// ok | missing | unreadable
    pub status: String,
    pub error: Option<String>,
    pub captured_at: i64,
    pub windows: RtkWindows,
    pub daily: Vec<RtkDay>,
    /// Project-only failure; the global card can still be `ok`.
    pub projects_error: Option<String>,
}

#[derive(Serialize, Clone, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RtkWindows {
    pub today: RtkWindow,
    pub d7: RtkWindow,
    pub d30: RtkWindow,
    pub all: RtkWindow,
}

#[derive(Serialize, Clone, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RtkWindow {
    pub saved_tokens: f64,
    pub commands: u64,
    pub compression_pct: Option<f64>,
    pub channels: Vec<RtkChannel>,
    pub other_channels_saved: f64,
    pub other_channels_commands: u64,
    pub unattributed_channel_saved: f64,
    pub unattributed_channel_commands: u64,
    pub projects: Vec<RtkProject>,
    /// Identifiable projects below the Top-N cut; kept separate from the
    /// unattributed bucket so the rows still add up to the window total.
    pub other_saved: f64,
    pub other_commands: u64,
    pub unattributed_saved: f64,
    pub unattributed_commands: u64,
}

#[derive(Serialize, Clone, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RtkChannel {
    pub name: String,
    pub saved_tokens: f64,
    pub commands: u64,
}

#[derive(Serialize, Clone, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RtkProject {
    pub name: String,
    pub saved_tokens: f64,
    pub commands: u64,
}

#[derive(Serialize, Clone, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RtkDay {
    pub date: String,
    pub saved_tokens: f64,
    pub commands: u64,
    pub compression_pct: Option<f64>,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
struct HistoryFile {
    version: u32,
    days: BTreeMap<String, HistoryDay>,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
struct HistoryDay {
    saved_tokens: f64,
    commands: u64,
    #[serde(default)]
    input_tokens: f64,
    #[serde(default)]
    output_tokens: f64,
    #[serde(default)]
    savings_pct: Option<f64>,
    #[serde(default)]
    captured_at: i64,
}

#[derive(Clone, Default, Debug)]
struct CliDay {
    date: String,
    saved_tokens: f64,
    commands: u64,
    input_tokens: f64,
    output_tokens: f64,
    savings_pct: Option<f64>,
}

#[derive(Clone, Default, Debug)]
struct GainDoc {
    total_saved: f64,
    total_commands: u64,
    total_input: f64,
    savings_pct: Option<f64>,
    daily: Vec<CliDay>,
}

/// Read the card with a 60 s memory cache. The lock is intentionally held
/// through collection so two rapid dashboard renders cannot launch two
/// `rtk gain` processes.
pub async fn fetch(force: bool) -> RtkSavings {
    let mut cache = CACHE.lock().await;
    if !force {
        if let Some((at, data)) = cache.as_ref() {
            if at.elapsed() < CACHE_TTL {
                return data.clone();
            }
        }
    }
    let fresh = collect().await;
    *cache = Some((Instant::now(), fresh.clone()));
    fresh
}

#[derive(Default)]
struct IntegrationProbe {
    exact: Vec<(PathBuf, &'static str)>,
    fuzzy_files: Vec<PathBuf>,
    fuzzy_dirs: Vec<PathBuf>,
}

fn integration_probe(id: &str, home: &Path, cwd: Option<&Path>) -> IntegrationProbe {
    let mut probe = IntegrationProbe::default();
    let home_path = |path: &str| home.join(path);
    let project_path = |path: &str| cwd.map(|root| root.join(path));
    let exact = |probe: &mut IntegrationProbe, path: PathBuf, marker: &'static str| {
        probe.exact.push((path, marker));
    };
    let fuzzy_file = |probe: &mut IntegrationProbe, path: Option<PathBuf>| {
        if let Some(path) = path {
            probe.fuzzy_files.push(path);
        }
    };
    match id {
        "codex" => {
            exact(&mut probe, home_path(".codex/hooks.json"), "rtk hook codex");
            fuzzy_file(&mut probe, Some(home_path(".codex/RTK.md")));
            fuzzy_file(&mut probe, Some(home_path(".codex/AGENTS.md")));
        }
        "claude" => {
            exact(
                &mut probe,
                home_path(".claude/settings.json"),
                "rtk hook claude",
            );
            fuzzy_file(&mut probe, Some(home_path(".claude/hooks/rtk-rewrite.sh")));
            fuzzy_file(&mut probe, Some(home_path(".claude/RTK.md")));
        }
        "cursor" => {
            exact(
                &mut probe,
                home_path(".cursor/hooks.json"),
                "rtk hook cursor",
            );
            fuzzy_file(
                &mut probe,
                Some(home_path(".cursor/hooks/rtk-rewrite.json")),
            );
        }
        "trae" => {
            for root in [
                home_path(".trae"),
                home_path(".trae-cn"),
                project_path(".trae").unwrap_or_default(),
                project_path(".trae-cn").unwrap_or_default(),
            ] {
                if !root.as_os_str().is_empty() {
                    exact(&mut probe, root.join("hooks.json"), "rtk hook trae");
                    probe.fuzzy_dirs.push(root);
                }
            }
        }
        "windsurf" => {
            if let Some(path) = project_path(".windsurfrules") {
                exact(&mut probe, path, "rtk");
            }
            fuzzy_file(&mut probe, Some(home_path(".windsurf/rules.md")));
        }
        "cline" => {
            if let Some(path) = project_path(".clinerules") {
                exact(&mut probe, path, "rtk");
            }
            fuzzy_file(&mut probe, Some(home_path(".cline/rules/rtk-rules.md")));
        }
        "kilocode" => {
            if let Some(path) = project_path(".kilocode/rules/rtk-rules.md") {
                exact(&mut probe, path, "rtk");
            }
            fuzzy_file(&mut probe, Some(home_path(".kilocode/rules/rtk-rules.md")));
        }
        "antigravity" => {
            if let Some(root) = project_path(".antigravity") {
                probe.fuzzy_dirs.push(root.clone());
                exact(&mut probe, root.join("rules/rtk-rules.md"), "rtk");
            }
            probe.fuzzy_dirs.push(home_path(".gemini/antigravity"));
            probe.fuzzy_dirs.push(home_path(".gemini/antigravity-cli"));
            probe.fuzzy_dirs.push(home_path(".gemini/antigravity-ide"));
        }
        "kimi" => {
            probe.fuzzy_dirs.push(home_path(".kimi"));
            if let Some(root) = project_path(".kimi") {
                probe.fuzzy_dirs.push(root);
            }
        }
        "copilot" => {
            exact(
                &mut probe,
                home_path(".copilot/hooks.json"),
                "rtk hook copilot",
            );
            exact(
                &mut probe,
                home_path(".github/copilot/hooks.json"),
                "rtk hook copilot",
            );
            fuzzy_file(&mut probe, project_path(".github/hooks/rtk-rewrite.json"));
            probe.fuzzy_dirs.push(home_path(".copilot"));
        }
        "gemini" => {
            exact(
                &mut probe,
                home_path(".gemini/settings.json"),
                "rtk hook gemini",
            );
            fuzzy_file(&mut probe, Some(home_path(".gemini/hooks/rtk-hook.sh")));
        }
        "opencode" => {
            fuzzy_file(
                &mut probe,
                Some(home_path(".config/opencode/plugin/rtk.ts")),
            );
            fuzzy_file(
                &mut probe,
                Some(home_path(".config/opencode/plugins/rtk.ts")),
            );
            fuzzy_file(&mut probe, project_path(".opencode/plugin/rtk.ts"));
        }
        "droid" => {
            exact(
                &mut probe,
                home_path(".factory/hooks.json"),
                "rtk hook droid",
            );
            exact(
                &mut probe,
                home_path(".factory/settings.local.json"),
                "rtk hook droid",
            );
            fuzzy_file(&mut probe, project_path(".factory/settings.local.json"));
        }
        "vibe" => {
            exact(&mut probe, home_path(".vibe/hooks.toml"), "rtk hook vibe");
            fuzzy_file(&mut probe, Some(home_path(".vibe/hooks.toml")));
        }
        "hermes" => {
            probe.fuzzy_dirs.push(home_path(".hermes/plugins"));
            if let Some(root) = project_path(".hermes/plugins") {
                probe.fuzzy_dirs.push(root);
            }
        }
        "pi" => {
            let root = std::env::var_os("PI_CODING_AGENT_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| home_path(".pi/agent"));
            probe.fuzzy_dirs.push(root);
        }
        "qodercn" => {
            // The hook entry is named rtk-rewrite and runs rtk-pretooluse.cjs;
            // matching the hook name keeps a stray mention of "rtk" in some
            // other setting from counting as connected.
            exact(
                &mut probe,
                home_path(".qoder-cn/settings.json"),
                "rtk-rewrite",
            );
            fuzzy_file(&mut probe, Some(home_path(".rtk/hooks/rtk-pretooluse.cjs")));
        }
        "omp" => {
            probe.fuzzy_dirs.push(home_path(".omp"));
            probe.fuzzy_dirs.push(home_path(".oh-my-pi"));
            if let Some(root) = project_path(".omp") {
                probe.fuzzy_dirs.push(root);
            }
        }
        _ => {}
    }
    probe
}

fn probe_text(path: &Path) -> Option<String> {
    let meta = std::fs::symlink_metadata(path).ok()?;
    if !meta.is_file() || meta.len() > 256 * 1024 {
        return None;
    }
    std::fs::read_to_string(path).ok()
}

fn fuzzy_text_is_rtk(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("rtk")
        && ["hook", "plugin", "rewrite", "rules", "instructions"]
            .iter()
            .any(|marker| lower.contains(marker))
}

fn dir_has_rtk_name(path: &Path, depth: u8) -> bool {
    let Ok(entries) = std::fs::read_dir(path) else {
        return false;
    };
    for entry in entries.flatten().take(80) {
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        if name.contains("rtk") {
            return true;
        }
        if depth > 0 && entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            if dir_has_rtk_name(&entry.path(), depth - 1) {
                return true;
            }
        }
    }
    false
}

fn detect_probe(probe: &IntegrationProbe) -> (&'static str, bool) {
    for (path, marker) in &probe.exact {
        if probe_text(path).is_some_and(|text| text.to_ascii_lowercase().contains(marker)) {
            return ("exact", true);
        }
    }
    for path in &probe.fuzzy_files {
        let named = path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().to_ascii_lowercase().contains("rtk"));
        if let Some(text) = probe_text(path) {
            if named || fuzzy_text_is_rtk(&text) {
                return ("fuzzy", true);
            }
        }
    }
    for path in &probe.fuzzy_dirs {
        if dir_has_rtk_name(path, 1) {
            return ("fuzzy", true);
        }
    }
    ("none", false)
}

pub fn integration_targets() -> Vec<RtkIntegrationTarget> {
    let home = dirs::home_dir().unwrap_or_default();
    let cwd = std::env::current_dir().ok();
    RTK_INTEGRATIONS
        .iter()
        .map(|spec| {
            let probe = integration_probe(spec.id, &home, cwd.as_deref());
            let (match_kind, detected) = detect_probe(&probe);
            RtkIntegrationTarget {
                id: spec.id,
                label: spec.label,
                icon_key: spec.icon_key,
                scope: integration_scope(spec.id).unwrap_or("global"),
                detected,
                match_kind,
                manual: !spec.cli,
            }
        })
        .collect()
}

pub fn integration_scope(id: &str) -> Option<&'static str> {
    match id {
        "cline" | "kilocode" | "antigravity" | "kimi" => Some("project"),
        "codex"
        | "claude"
        | "cursor"
        | "trae"
        | "windsurf"
        | "copilot"
        | "gemini"
        | "opencode"
        | "droid"
        | "vibe"
        | "hermes"
        | "pi"
        | "omp" => Some("global"),
        _ => None,
    }
}

fn integration_args(id: &str) -> Option<Vec<&'static str>> {
    match id {
        // `rtk init --codex` rejects --auto-patch (rtk 0.50.0).
        "codex" => Some(vec!["init", "-g", "--codex"]),
        "copilot" => Some(vec!["init", "-g", "--copilot", "--auto-patch"]),
        "gemini" => Some(vec!["init", "-g", "--gemini", "--auto-patch"]),
        "opencode" => Some(vec!["init", "-g", "--opencode", "--auto-patch"]),
        "claude" => Some(vec!["init", "-g", "--agent", "claude", "--auto-patch"]),
        "cursor" => Some(vec!["init", "-g", "--agent", "cursor", "--auto-patch"]),
        "trae" => Some(vec!["init", "-g", "--agent", "trae", "--auto-patch"]),
        "windsurf" => Some(vec!["init", "-g", "--agent", "windsurf", "--auto-patch"]),
        "cline" => Some(vec!["init", "--agent", "cline", "--auto-patch"]),
        "kilocode" => Some(vec!["init", "--agent", "kilocode", "--auto-patch"]),
        "antigravity" => Some(vec!["init", "--agent", "antigravity", "--auto-patch"]),
        "kimi" => Some(vec!["init", "--agent", "kimi", "--auto-patch"]),
        "droid" => Some(vec!["init", "-g", "--agent", "droid", "--auto-patch"]),
        "vibe" => Some(vec!["init", "-g", "--agent", "vibe", "--auto-patch"]),
        "hermes" => Some(vec!["init", "-g", "--agent", "hermes", "--auto-patch"]),
        "pi" => Some(vec!["init", "-g", "--agent", "pi", "--auto-patch"]),
        "omp" => Some(vec!["init", "-g", "--agent", "omp", "--auto-patch"]),
        _ => None,
    }
}

pub async fn invalidate_cache() {
    *CACHE.lock().await = None;
}

pub async fn install_rtk() -> Result<(), String> {
    if rtk_binary().is_some() {
        return Ok(());
    }
    #[cfg(not(windows))]
    {
        return Err("Automatic RTK installation is currently available on Windows only; install it from rtk-ai.app.".into());
    }
    #[cfg(windows)]
    {
        let mut command = crate::platform::hidden_command("winget.exe");
        command.args([
            "install",
            "--id",
            "rtk-ai.rtk",
            "--exact",
            "--source",
            "winget",
            "--accept-package-agreements",
            "--accept-source-agreements",
            "--silent",
            "--disable-interactivity",
        ]);
        let output = tokio::time::timeout(INSTALL_TIMEOUT, command.output())
            .await
            .map_err(|_| "RTK installation timed out".to_string())?
            .map_err(|error| format!("could not start winget: {error}"))?;
        if !output.status.success() {
            return Err(command_error(&output, "winget could not install RTK"));
        }
        invalidate_cache().await;
        if rtk_binary().is_some() {
            Ok(())
        } else {
            Err("winget finished, but Pane still cannot find rtk.exe. Restart Pane or check RTK_BIN.".into())
        }
    }
}

pub async fn install_integration(id: &str, project_dir: Option<&Path>) -> Result<(), String> {
    let scope = integration_scope(id).ok_or_else(|| "unknown RTK integration target".to_string())?;
    let args = integration_args(id).ok_or_else(|| "unknown RTK integration target".to_string())?;
    let binary = rtk_binary().ok_or_else(|| "RTK is not installed".to_string())?;
    let mut command = crate::platform::hidden_command(&binary.to_string_lossy());
    command
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    if scope == "project" {
        let project_dir = project_dir
            .filter(|path| path.is_dir())
            .ok_or_else(|| "Choose an existing project folder first".to_string())?;
        command.current_dir(project_dir);
    }
    let output = tokio::time::timeout(INTEGRATION_TIMEOUT, command.output())
        .await
        .map_err(|_| "RTK integration timed out".to_string())?
        .map_err(|error| format!("could not start RTK: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(command_error(&output, "RTK integration failed"))
    }
}

fn command_error(output: &std::process::Output, fallback: &str) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let text = if stderr.trim().is_empty() {
        stdout.trim()
    } else {
        stderr.trim()
    };
    if text.is_empty() {
        return fallback.to_string();
    }
    let tail: String = text
        .chars()
        .rev()
        .take(600)
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    format!("{fallback}: {tail}")
}

async fn collect() -> RtkSavings {
    let captured_at = now_ms();
    let Some(bin) = rtk_binary() else {
        return RtkSavings {
            status: "missing".into(),
            error: Some("rtk binary was not found".into()),
            captured_at,
            ..RtkSavings::default()
        };
    };
    // Read the project DB before the CLI summary. Commands can land between
    // the two reads, and that ordering makes the CLI total the newer side:
    // the breakdown may lag by a few commands, but it cannot exceed it.
    let project = scan_projects(project_db());
    let output = tokio::time::timeout(CLI_TIMEOUT, async {
        // Console child: go through the platform seam so release builds
        // never flash a terminal window (platform::hidden_command).
        let mut command = crate::platform::hidden_command(&bin.to_string_lossy());
        command
            .args(["gain", "--all", "--format", "json"])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .output()
            .await
    })
    .await;
    let doc = match output {
        Ok(Ok(out)) if out.status.success() => {
            match parse_gain(&String::from_utf8_lossy(&out.stdout)) {
                Ok(doc) => doc,
                Err(message) => {
                    return RtkSavings {
                        status: "unreadable".into(),
                        error: Some(message),
                        captured_at,
                        ..RtkSavings::default()
                    }
                }
            }
        }
        Ok(Ok(_)) => {
            return RtkSavings {
                status: "unreadable".into(),
                error: Some("rtk gain exited with an error".into()),
                captured_at,
                ..RtkSavings::default()
            }
        }
        Ok(Err(_)) | Err(_) => {
            return RtkSavings {
                status: "unreadable".into(),
                error: Some("rtk gain timed out or could not be started".into()),
                captured_at,
                ..RtkSavings::default()
            }
        }
    };

    let mut history = HistoryFile::default();
    let mut history_error = None;
    match load_history() {
        Ok(file) => history = file,
        Err(error) => {
            // Keep the unreadable file for diagnosis instead of silently
            // replacing it with today's rows on the next merge.
            let _ = std::fs::rename(history_path(), history_path().with_extension("json.bad"));
            history_error = Some(error);
        }
    }
    if history_error.is_none() {
        history_error = merge_history(&mut history, &doc.daily, captured_at).err();
    }
    let merged = merged_daily(&history, &doc.daily);
    let today = Local::now().date_naive();
    let daily = last_days(&merged, today, 30);
    let project = project.map(|rows| build_windows(&doc, &merged, today, rows));
    let (windows, projects_error) = match project {
        Ok(windows) => (windows, None),
        Err(message) => {
            let windows = RtkWindows {
                today: window_from_daily(&doc, &merged, today, 1),
                d7: window_from_daily(&doc, &merged, today, 7),
                d30: window_from_daily(&doc, &merged, today, 30),
                all: window_from_daily(&doc, &merged, today, 0),
            };
            (windows, Some(message))
        }
    };
    RtkSavings {
        status: "ok".into(),
        error: history_error,
        captured_at,
        windows,
        daily,
        projects_error,
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn rtk_binary() -> Option<PathBuf> {
    // An explicit override is authoritative: a typo must read as "missing"
    // instead of silently probing PATH and showing some other install.
    if let Ok(path) = std::env::var("RTK_BIN") {
        let path = PathBuf::from(path);
        return path.is_file().then_some(path);
    }
    let names: &[&str] = if cfg!(windows) {
        &["rtk.exe"]
    } else {
        &["rtk"]
    };
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            for name in names {
                let candidate = dir.join(name);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }
    let home = dirs::home_dir()?;
    for name in names {
        let candidate = home.join(".local").join("bin").join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    #[cfg(windows)]
    if let Some(local) = dirs::data_local_dir() {
        let candidate = local
            .join("Microsoft")
            .join("WinGet")
            .join("Links")
            .join("rtk.exe");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn parse_gain(raw: &str) -> Result<GainDoc, String> {
    let value: Value = serde_json::from_str(raw.trim())
        .map_err(|_| "rtk gain returned invalid JSON".to_string())?;
    let summary = value
        .get("summary")
        .ok_or_else(|| "rtk gain JSON has no summary".to_string())?;
    let total_saved =
        num(summary.get("total_saved")).ok_or_else(|| "summary.total_saved missing".to_string())?;
    let total_commands = num(summary.get("total_commands"))
        .map(|v| v.max(0.0) as u64)
        .unwrap_or(0);
    let total_input = num(summary.get("total_input")).unwrap_or(0.0);
    let savings_pct = num(summary.get("avg_savings_pct"));
    let daily = value
        .get("daily")
        .and_then(Value::as_array)
        .map(|rows| rows.iter().filter_map(parse_cli_day).collect())
        .unwrap_or_default();
    Ok(GainDoc {
        total_saved,
        total_commands,
        total_input,
        savings_pct,
        daily,
    })
}

fn parse_cli_day(row: &Value) -> Option<CliDay> {
    let date = row.get("date")?.as_str()?.trim().to_string();
    if date.len() != 10 || parse_date(&date).is_none() {
        return None;
    }
    Some(CliDay {
        date,
        saved_tokens: num(row.get("saved_tokens")).unwrap_or(0.0),
        commands: num(row.get("commands")).unwrap_or(0.0).max(0.0) as u64,
        input_tokens: num(row.get("input_tokens")).unwrap_or(0.0),
        output_tokens: num(row.get("output_tokens")).unwrap_or(0.0),
        savings_pct: num(row.get("savings_pct")),
    })
}

fn num(value: Option<&Value>) -> Option<f64> {
    match value? {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
}

impl GainDoc {
    fn today_saved(&self, merged: &BTreeMap<String, CliDay>, today: NaiveDate) -> f64 {
        merged
            .get(&today.format("%Y-%m-%d").to_string())
            .map(|d| d.saved_tokens)
            .unwrap_or(0.0)
    }

    fn window_saved(&self, merged: &BTreeMap<String, CliDay>, today: NaiveDate, days: i64) -> f64 {
        if days <= 0 {
            return self.total_saved;
        }
        let start = today - chrono::Duration::days(days - 1);
        merged
            .values()
            .filter(|d| parse_date(&d.date).is_some_and(|date| date >= start && date <= today))
            .map(|d| d.saved_tokens)
            .sum()
    }
}

fn parse_date(text: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(text, "%Y-%m-%d").ok()
}

fn merged_daily(history: &HistoryFile, cli: &[CliDay]) -> BTreeMap<String, CliDay> {
    let mut merged: BTreeMap<String, CliDay> = history
        .days
        .iter()
        .map(|(date, day)| {
            (
                date.clone(),
                CliDay {
                    date: date.clone(),
                    saved_tokens: day.saved_tokens,
                    commands: day.commands,
                    input_tokens: day.input_tokens,
                    output_tokens: day.output_tokens,
                    savings_pct: day.savings_pct,
                },
            )
        })
        .collect();
    for day in cli {
        merged.insert(day.date.clone(), day.clone());
    }
    merged
}

fn last_days(merged: &BTreeMap<String, CliDay>, today: NaiveDate, days: i64) -> Vec<RtkDay> {
    (0..days)
        .map(|offset| today - chrono::Duration::days(days - 1 - offset))
        .map(|date| {
            let key = date.format("%Y-%m-%d").to_string();
            let day = merged.get(&key);
            RtkDay {
                date: key,
                saved_tokens: day.map(|d| d.saved_tokens).unwrap_or(0.0),
                commands: day.map(|d| d.commands).unwrap_or(0),
                compression_pct: day.and_then(|d| d.savings_pct),
            }
        })
        .collect()
}

fn history_path() -> PathBuf {
    crate::providers::config_dir().join("rtk_savings_history.json")
}

fn load_history() -> Result<HistoryFile, String> {
    let raw = match std::fs::read_to_string(history_path()) {
        Ok(raw) => raw,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(HistoryFile {
                version: HISTORY_VERSION,
                ..HistoryFile::default()
            })
        }
        Err(err) => return Err(format!("read rtk history: {err}")),
    };
    let mut file: HistoryFile = serde_json::from_str(raw.trim_start_matches('\u{feff}'))
        .map_err(|err| format!("parse rtk history: {err}"))?;
    file.version = HISTORY_VERSION;
    Ok(file)
}

fn merge_history(file: &mut HistoryFile, daily: &[CliDay], captured_at: i64) -> Result<(), String> {
    merge_into(file, daily, captured_at);
    if let Some(today) = file.days.keys().filter_map(|d| parse_date(d)).max() {
        let cutoff = today - chrono::Duration::days(KEEP_DAYS - 1);
        file.days
            .retain(|date, _| parse_date(date).is_some_and(|d| d >= cutoff));
    }
    file.version = HISTORY_VERSION;
    let path = history_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| format!("create rtk history dir: {err}"))?;
    }
    let tmp = path.with_extension("json.tmp");
    let bytes =
        serde_json::to_vec_pretty(file).map_err(|err| format!("encode rtk history: {err}"))?;
    std::fs::write(&tmp, bytes).map_err(|err| format!("write rtk history: {err}"))?;
    std::fs::rename(tmp, path).map_err(|err| format!("replace rtk history: {err}"))?;
    Ok(())
}

/// Same-day rows are replaced, never added — RTK's daily row already covers
/// the whole day, so a second refresh must not double the snapshot.
fn merge_into(file: &mut HistoryFile, daily: &[CliDay], captured_at: i64) {
    for day in daily {
        file.days.insert(
            day.date.clone(),
            HistoryDay {
                saved_tokens: day.saved_tokens,
                commands: day.commands,
                input_tokens: day.input_tokens,
                output_tokens: day.output_tokens,
                savings_pct: day.savings_pct,
                captured_at,
            },
        );
    }
}

fn project_db() -> PathBuf {
    let local = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| dirs::data_local_dir().unwrap_or_default());
    local.join("rtk").join("history.db")
}

#[derive(Clone, Default)]
struct DbRow {
    date: NaiveDate,
    channel: Option<String>,
    name: Option<String>,
    saved: f64,
    commands: u64,
}

fn scan_projects(db: PathBuf) -> Result<Vec<DbRow>, String> {
    if !db.is_file() {
        return Ok(Vec::new());
    }
    let dir = std::env::temp_dir().join(format!(
        "pane-rtk-{}-{}",
        std::process::id(),
        COPY_SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir)
        .map_err(|_| "copy RTK history: create temp dir failed".to_string())?;
    let local = dir.join("history.db");
    let copied = std::fs::copy(&db, &local).map_err(|_| "copy RTK history failed".to_string())?;
    if copied == 0 {
        let _ = std::fs::remove_dir_all(&dir);
        return Err("RTK history is empty".into());
    }
    for suffix in ["-wal", "-shm"] {
        let sidecar = PathBuf::from(format!("{}{}", db.display(), suffix));
        if sidecar.is_file() {
            let _ = std::fs::copy(&sidecar, dir.join(format!("history.db{suffix}")));
        }
    }
    let result = read_commands(&local);
    let _ = std::fs::remove_dir_all(&dir);
    result
}

fn read_commands(db: &Path) -> Result<Vec<DbRow>, String> {
    let conn =
        rusqlite::Connection::open_with_flags(db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|_| "open RTK history copy failed".to_string())?;
    let _ = conn.pragma_update(None, "query_only", 1);
    let mut stmt = conn
        .prepare("SELECT timestamp, project_path, COALESCE(saved_tokens, 0), rtk_cmd FROM commands")
        .map_err(|_| "RTK commands table is unavailable".to_string())?;
    let rows = stmt
        .query_map([], |row| {
            let ts: String = row.get(0)?;
            let path: String = row.get(1)?;
            let saved: f64 = row.get::<_, f64>(2).unwrap_or(0.0);
            let rtk_cmd: String = row.get(3).unwrap_or_default();
            Ok((ts, path, saved, rtk_cmd))
        })
        .map_err(|_| "read RTK commands failed".to_string())?;
    let mut out = Vec::new();
    for row in rows.flatten() {
        let (ts, path, saved, rtk_cmd) = row;
        let Ok(parsed) = DateTime::parse_from_rfc3339(&ts) else {
            continue;
        };
        let date = parsed.with_timezone(&Local).date_naive();
        out.push(DbRow {
            date,
            channel: clean_channel(&rtk_cmd),
            name: clean_project(&path),
            saved: saved.max(0.0),
            commands: 1,
        });
    }
    Ok(out)
}

fn clean_channel(raw: &str) -> Option<String> {
    let mut parts = raw.split_whitespace();
    let first = parts.next()?;
    let channel = if first.rsplit(['\\', '/']).next().is_some_and(|name| {
        name.eq_ignore_ascii_case("rtk") || name.eq_ignore_ascii_case("rtk.exe")
    }) {
        parts.next()?
    } else {
        first
    };
    let channel = channel.trim().to_ascii_lowercase();
    (!channel.is_empty() && !channel.starts_with('-')).then_some(channel)
}

fn clean_project(raw: &str) -> Option<String> {
    clean_project_with_home(raw, dirs::home_dir().as_deref())
}

fn clean_project_with_home(raw: &str, home: Option<&Path>) -> Option<String> {
    let path = raw.trim().strip_prefix(r"\\?\").unwrap_or(raw.trim());
    if path.is_empty() {
        return None;
    }
    let lower = path.replace('/', "\\").to_ascii_lowercase();
    let normalized = lower.trim_end_matches('\\');
    let is_home = home.is_some_and(|home| {
        let home = home.to_string_lossy();
        let home = home.strip_prefix(r"\\?\").unwrap_or(&home);
        normalized
            == home
                .replace('/', "\\")
                .to_ascii_lowercase()
                .trim_end_matches('\\')
    });
    let is_drive_root = normalized.len() == 2
        && normalized.as_bytes()[0].is_ascii_alphabetic()
        && normalized.as_bytes()[1] == b':';
    if lower.contains("\\appdata\\")
        || lower.contains("\\temp\\")
        || lower.contains("\\.qoder-cn")
        || lower.ends_with("\\resources\\app")
        || lower.contains("\\resources\\app\\")
        || is_home
        || is_drive_root
    {
        return None;
    }
    let name = path.rsplit(['\\', '/']).find(|part| !part.is_empty())?;
    if name.ends_with(':') || name == "." || name == ".." || name.len() > 80 {
        return None;
    }
    if looks_like_date(name) || looks_like_hash(name) {
        return None;
    }
    Some(name.to_string())
}

fn looks_like_date(text: &str) -> bool {
    text.len() == 10
        && text.as_bytes()[4] == b'-'
        && text.as_bytes()[7] == b'-'
        && text
            .bytes()
            .enumerate()
            .all(|(i, b)| i == 4 || i == 7 || b.is_ascii_digit())
}

fn looks_like_hash(text: &str) -> bool {
    text.len() >= 8 && text.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-')
}

fn build_windows(
    doc: &GainDoc,
    merged: &BTreeMap<String, CliDay>,
    today: NaiveDate,
    rows: Vec<DbRow>,
) -> RtkWindows {
    let mut today_win = WindowAgg::default();
    let mut d7 = WindowAgg::default();
    let mut d30 = WindowAgg::default();
    let mut all = WindowAgg::default();
    let d7_start = today - chrono::Duration::days(6);
    let d30_start = today - chrono::Duration::days(29);
    for row in rows {
        let windows = [
            (row.date == today, &mut today_win),
            (row.date >= d7_start && row.date <= today, &mut d7),
            (row.date >= d30_start && row.date <= today, &mut d30),
            (true, &mut all),
        ];
        for (include, win) in windows {
            if !include {
                continue;
            }
            win.commands += row.commands;
            if let Some(channel) = &row.channel {
                let entry = win.channels.entry(channel.clone()).or_default();
                entry.0 += row.saved;
                entry.1 += row.commands;
            }
            if let Some(name) = &row.name {
                let entry = win.projects.entry(name.clone()).or_default();
                entry.0 += row.saved;
                entry.1 += row.commands;
                win.clean_saved += row.saved;
            } else {
                win.noise_saved += row.saved;
                win.noise_commands += row.commands;
            }
        }
    }
    RtkWindows {
        today: today_win.finish(
            doc.today_saved(merged, today),
            doc.today_commands(merged, today),
            doc.window_compression(merged, today, 1),
        ),
        d7: d7.finish(
            doc.window_saved(merged, today, 7),
            doc.window_commands(merged, today, 7),
            doc.window_compression(merged, today, 7),
        ),
        d30: d30.finish(
            doc.window_saved(merged, today, 30),
            doc.window_commands(merged, today, 30),
            doc.window_compression(merged, today, 30),
        ),
        all: all.finish(doc.total_saved, doc.total_commands, doc.compression()),
    }
}

impl GainDoc {
    fn compression(&self) -> Option<f64> {
        if self.total_input > 0.0 {
            Some(self.total_saved / self.total_input * 100.0)
        } else {
            self.savings_pct
        }
    }

    fn window_compression(
        &self,
        merged: &BTreeMap<String, CliDay>,
        today: NaiveDate,
        days: i64,
    ) -> Option<f64> {
        if days <= 0 {
            return self.compression();
        }
        let start = today - chrono::Duration::days(days - 1);
        let mut input = 0.0;
        let mut saved = 0.0;
        for day in merged.values() {
            if parse_date(&day.date).is_some_and(|date| date >= start && date <= today) {
                input += day.input_tokens;
                saved += day.saved_tokens;
            }
        }
        (input > 0.0).then_some(saved / input * 100.0)
    }

    fn today_commands(&self, merged: &BTreeMap<String, CliDay>, today: NaiveDate) -> u64 {
        merged
            .get(&today.format("%Y-%m-%d").to_string())
            .map(|d| d.commands)
            .unwrap_or(0)
    }

    fn window_commands(
        &self,
        merged: &BTreeMap<String, CliDay>,
        today: NaiveDate,
        days: i64,
    ) -> u64 {
        if days <= 0 {
            return self.total_commands;
        }
        let start = today - chrono::Duration::days(days - 1);
        merged
            .values()
            .filter(|d| parse_date(&d.date).is_some_and(|date| date >= start && date <= today))
            .map(|d| d.commands)
            .sum()
    }
}

#[derive(Default)]
struct WindowAgg {
    commands: u64,
    clean_saved: f64,
    noise_saved: f64,
    noise_commands: u64,
    channels: BTreeMap<String, (f64, u64)>,
    projects: BTreeMap<String, (f64, u64)>,
}

impl WindowAgg {
    fn finish(self, saved: f64, commands: u64, compression: Option<f64>) -> RtkWindow {
        // The RTK CLI summary owns the window total. Its history DB can briefly
        // lead the summary by a few hundred bytes when a command is landing, so
        // scale the attribution to the CLI budget instead of hiding all rows.
        let db_saved = self.clean_saved + self.noise_saved;
        let scale = if db_saved > saved && db_saved > 0.0 {
            saved.max(0.0) / db_saved
        } else {
            1.0
        };
        let clean_saved = self.clean_saved * scale;
        let channel_commands: u64 = self.channels.values().map(|(_, commands)| commands).sum();
        let mut channels: Vec<RtkChannel> = self
            .channels
            .into_iter()
            .map(|(name, (saved, commands))| RtkChannel {
                name,
                saved_tokens: saved * scale,
                commands,
            })
            .collect();
        channels.sort_by(|a, b| {
            b.saved_tokens
                .partial_cmp(&a.saved_tokens)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.name.cmp(&b.name))
        });
        let total_channel_saved: f64 = channels.iter().map(|channel| channel.saved_tokens).sum();
        channels.truncate(TOP_PROJECTS);
        let shown_channel_saved: f64 = channels.iter().map(|channel| channel.saved_tokens).sum();
        let shown_channel_commands: u64 = channels.iter().map(|channel| channel.commands).sum();
        let attributed_commands: u64 = self.projects.values().map(|(_, commands)| commands).sum();
        let mut projects: Vec<RtkProject> = self
            .projects
            .into_iter()
            .map(|(name, (saved, commands))| RtkProject {
                name,
                saved_tokens: saved * scale,
                commands,
            })
            .collect();
        projects.sort_by(|a, b| {
            b.saved_tokens
                .partial_cmp(&a.saved_tokens)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.name.cmp(&b.name))
        });
        let total_attributed: f64 = projects.iter().map(|p| p.saved_tokens).sum();
        let total_attributed_commands: u64 = projects.iter().map(|p| p.commands).sum();
        projects.truncate(TOP_PROJECTS);
        let shown: f64 = projects.iter().map(|p| p.saved_tokens).sum();
        let shown_commands: u64 = projects.iter().map(|p| p.commands).sum();
        RtkWindow {
            saved_tokens: saved,
            commands,
            compression_pct: compression,
            channels,
            other_channels_saved: (total_channel_saved - shown_channel_saved).max(0.0),
            other_channels_commands: channel_commands.saturating_sub(shown_channel_commands),
            unattributed_channel_saved: (saved - total_channel_saved).max(0.0),
            unattributed_channel_commands: commands.saturating_sub(channel_commands),
            projects,
            other_saved: (total_attributed - shown).max(0.0),
            other_commands: total_attributed_commands.saturating_sub(shown_commands),
            unattributed_saved: (saved - clean_saved).max(0.0),
            unattributed_commands: commands.saturating_sub(attributed_commands),
        }
    }
}

fn window_from_daily(
    doc: &GainDoc,
    merged: &BTreeMap<String, CliDay>,
    today: NaiveDate,
    days: i64,
) -> RtkWindow {
    let saved = doc.window_saved(merged, today, days);
    let commands = doc.window_commands(merged, today, days);
    let compression = doc.window_compression(merged, today, days);
    RtkWindow {
        saved_tokens: saved,
        commands,
        compression_pct: compression,
        unattributed_saved: saved,
        unattributed_commands: commands,
        unattributed_channel_saved: saved,
        unattributed_channel_commands: commands,
        ..RtkWindow::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
      "summary":{"total_commands":10,"total_input":1000,"total_saved":900,"avg_savings_pct":90},
      "daily":[
        {"date":"2026-10-24","commands":7,"input_tokens":700,"output_tokens":70,"saved_tokens":630,"savings_pct":90},
        {"date":"2026-10-23","commands":3,"input_tokens":300,"output_tokens":30,"saved_tokens":270,"savings_pct":90}
      ]
    }"#;

    #[test]
    fn parses_gain_and_builds_windows() {
        let doc = parse_gain(SAMPLE).expect("parse");
        assert_eq!(doc.total_saved, 900.0);
        assert_eq!(doc.total_commands, 10);
        assert_eq!(doc.daily.len(), 2);
        let today = parse_date("2026-10-24").unwrap();
        let merged = merged_daily(&HistoryFile::default(), &doc.daily);
        assert_eq!(doc.window_saved(&merged, today, 1), 630.0);
        assert_eq!(doc.window_saved(&merged, today, 7), 900.0);
        assert_eq!(doc.window_commands(&merged, today, 7), 10);
    }

    #[test]
    fn rejects_invalid_or_old_json() {
        assert!(parse_gain("not json").is_err());
        assert!(parse_gain(r#"{"daily":[]}"#).is_err());
    }

    #[test]
    fn project_names_filter_noise_and_show_basename() {
        assert_eq!(clean_project(r"\\?\D:\code\pane").as_deref(), Some("pane"));
        assert_eq!(
            clean_project(r"D:\code\Firefly").as_deref(),
            Some("Firefly")
        );
        assert_eq!(
            clean_project(r"C:\Users\Lenovo\AppData\Local\Temp\2026-10-24\f628eddd"),
            None
        );
        assert_eq!(clean_project(r"C:\Users\Lenovo\.qoder-cn"), None);
        assert_eq!(
            clean_project(r"C:\Users\Lenovo\.qoder-cn\resources\app"),
            None
        );
        assert_eq!(clean_project(r"D:\tools\resources\app\project"), None);
        assert_eq!(clean_project(r"D:\code\2026-10-24"), None);
        for home in [r"C:\Users\alice", r"D:\Users\bob", "/home/tester"] {
            assert_eq!(clean_project_with_home(home, Some(Path::new(home))), None);
            assert_eq!(
                clean_project_with_home(&format!("{home}/"), Some(Path::new(home))),
                None
            );
        }
        assert_eq!(
            clean_project_with_home(r"\\?\D:\Users\BOB", Some(Path::new("d:/Users/bob/"))),
            None
        );
        for root in ["C:\\", "D:/", "z:\\", "/"] {
            assert_eq!(clean_project_with_home(root, None), None);
        }
        assert_eq!(
            clean_project_with_home(r"D:\projects\bob", Some(Path::new(r"D:\Users\bob")))
                .as_deref(),
            Some("bob")
        );
    }

    #[tokio::test]
    #[ignore = "real-machine RTK probe; reads the local rtk CLI and history.db"]
    async fn real_machine_rtk_savings_is_readable() {
        let data = collect().await;
        eprintln!(
            "rtk status={} windows={{today:{},d7:{},d30:{},all:{}}} projects={} unattributed={} projects_error={:?}",
            data.status,
            data.windows.today.saved_tokens,
            data.windows.d7.saved_tokens,
            data.windows.d30.saved_tokens,
            data.windows.all.saved_tokens,
            data.windows.d30.projects.len(),
            data.windows.d30.unattributed_saved,
            data.projects_error,
        );
        assert!(data.status == "ok" || data.status == "missing");
    }

    #[test]
    fn database_overshoot_is_scaled_into_the_cli_budget() {
        let mut agg = WindowAgg::default();
        agg.clean_saved = 700.0;
        agg.commands = 8;
        agg.projects.insert("project".into(), (700.0, 8));
        let win = agg.finish(630.0, 7, None);
        assert_eq!(win.projects[0].saved_tokens, 630.0);
        assert_eq!(win.other_saved, 0.0);
        assert_eq!(win.unattributed_saved, 0.0);
        assert_eq!(
            win.projects[0].saved_tokens + win.other_saved + win.unattributed_saved,
            win.saved_tokens,
        );
    }

    #[test]
    fn command_channel_is_the_rtk_subcommand_family() {
        assert_eq!(clean_channel("rtk git status"), Some("git".into()));
        assert_eq!(
            clean_channel(r"C:\tools\rtk.exe cargo test"),
            Some("cargo".into())
        );
        assert_eq!(clean_channel("rtk rg pattern"), Some("rg".into()));
        assert_eq!(clean_channel(""), None);
        assert_eq!(clean_channel("rtk"), None);
    }

    #[test]
    fn integration_targets_are_known_and_map_to_fixed_commands() {
        let targets = integration_targets();
        assert_eq!(targets.len(), 18);
        for target in &targets {
            if target.manual {
                assert!(
                    integration_args(target.id).is_none(),
                    "{} is hook-only, so it must not advertise a CLI install",
                    target.id
                );
                continue;
            }
            let args = integration_args(target.id).expect("known target");
            if target.scope == "project" {
                assert_eq!(args[0], "init");
                assert!(!args.contains(&"-g"));
                assert!(args.contains(&"--auto-patch"));
            } else {
                assert_eq!(&args[..2], ["init", "-g"]);
                assert_eq!(args.contains(&"--auto-patch"), target.id != "codex");
            }
        }
        assert_eq!(
            integration_args("codex"),
            Some(vec!["init", "-g", "--codex"]),
        );
        assert_eq!(
            integration_args("cursor"),
            Some(vec!["init", "-g", "--agent", "cursor", "--auto-patch"]),
        );
        assert_eq!(
            integration_args("kilocode"),
            Some(vec!["init", "--agent", "kilocode", "--auto-patch"]),
        );
        assert_eq!(integration_scope("kilocode"), Some("project"));
        assert_eq!(integration_scope("cursor"), Some("global"));
        assert!(integration_args("arbitrary-command").is_none());
    }

    #[test]
    fn detector_prefers_exact_markers_and_falls_back_to_fuzzy_files() {
        let root = std::env::temp_dir().join(format!("pane-rtk-detect-{}", std::process::id()));
        let codex = root.join(".codex");
        std::fs::create_dir_all(&codex).unwrap();
        std::fs::write(
            codex.join("hooks.json"),
            r#"{"hooks":{"PreToolUse":[{"command":"rtk hook codex"}]}}"#,
        )
        .unwrap();
        let codex_probe = integration_probe("codex", &root, None);
        assert_eq!(detect_probe(&codex_probe), ("exact", true));

        let kilocode = root.join("project/.kilocode/rules");
        std::fs::create_dir_all(&kilocode).unwrap();
        std::fs::write(kilocode.join("rtk-rules.md"), "Use RTK for commands.").unwrap();
        let kilo_probe = integration_probe("kilocode", &root, Some(&root.join("project")));
        assert_eq!(detect_probe(&kilo_probe), ("exact", true));

        let plugin = root.join(".config/opencode/plugin");
        std::fs::create_dir_all(&plugin).unwrap();
        std::fs::write(plugin.join("rtk.ts"), "export const plugin = true;").unwrap();
        let opencode_probe = integration_probe("opencode", &root, None);
        assert_eq!(detect_probe(&opencode_probe), ("fuzzy", true));

        let empty = integration_probe("claude", &root.join("empty"), None);
        assert_eq!(detect_probe(&empty), ("none", false));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn history_merge_replaces_same_day() {
        let mut file = HistoryFile::default();
        let day = CliDay {
            date: "2026-10-24".into(),
            saved_tokens: 100.0,
            commands: 1,
            input_tokens: 120.0,
            output_tokens: 20.0,
            savings_pct: Some(83.3),
        };
        merge_into(&mut file, &[day.clone()], 1);
        merge_into(
            &mut file,
            &[CliDay {
                saved_tokens: 140.0,
                ..day
            }],
            2,
        );
        assert_eq!(file.days["2026-10-24"].saved_tokens, 140.0);
        assert_eq!(file.days.len(), 1);
    }

    #[test]
    fn history_fills_days_the_cli_no_longer_returns() {
        let mut file = HistoryFile::default();
        merge_into(
            &mut file,
            &[CliDay {
                date: "2026-10-01".into(),
                saved_tokens: 42.0,
                commands: 2,
                input_tokens: 50.0,
                output_tokens: 5.0,
                savings_pct: Some(84.0),
            }],
            1,
        );
        let merged = merged_daily(&file, &[]);
        let today = parse_date("2026-10-03").unwrap();
        let daily = last_days(&merged, today, 7);
        let old = daily
            .iter()
            .find(|d| d.date == "2026-10-01")
            .expect("snapshot day");
        assert_eq!(old.saved_tokens, 42.0);
        assert_eq!(old.commands, 2);
    }

    #[test]
    fn missing_or_corrupt_project_db_degrades_without_panicking() {
        let dir = std::env::temp_dir().join(format!("pane-rtk-probe-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert!(scan_projects(dir.join("absent.db")).unwrap().is_empty());
        let bad = dir.join("history.db");
        std::fs::write(&bad, b"this is not sqlite").unwrap();
        assert!(scan_projects(bad).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn projects_and_unattributed_add_up_to_the_window_total() {
        let doc = parse_gain(SAMPLE).expect("parse");
        let today = parse_date("2026-10-24").unwrap();
        let merged = merged_daily(&HistoryFile::default(), &doc.daily);
        let rows = vec![
            DbRow {
                date: today,
                channel: Some("rg".into()),
                name: Some("pane".into()),
                saved: 400.0,
                commands: 2,
            },
            DbRow {
                date: today,
                channel: Some("read".into()),
                name: None,
                saved: 100.0,
                commands: 1,
            },
        ];
        let windows = build_windows(&doc, &merged, today, rows);
        let today_win = &windows.today;
        assert_eq!(today_win.saved_tokens, 630.0);
        assert_eq!(today_win.commands, 7);
        assert_eq!(today_win.projects.len(), 1);
        assert_eq!(today_win.projects[0].name, "pane");
        // 630 total − 400 attributed = 230; the hidden path never leaks.
        assert_eq!(today_win.unattributed_saved, 230.0);
        assert_eq!(today_win.unattributed_commands, 5);
        assert_eq!(today_win.compression_pct, Some(90.0));
        assert_eq!(today_win.channels.len(), 2);
        assert_eq!(today_win.channels[0].name, "rg");
        assert_eq!(today_win.unattributed_channel_saved, 130.0);
        let channel_total: f64 = today_win.channels.iter().map(|c| c.saved_tokens).sum();
        assert_eq!(
            channel_total + today_win.other_channels_saved + today_win.unattributed_channel_saved,
            today_win.saved_tokens,
        );
    }

    #[test]
    fn project_breakdown_never_exceeds_the_global_total() {
        let doc = parse_gain(SAMPLE).expect("parse");
        let today = parse_date("2026-10-24").unwrap();
        let merged = merged_daily(&HistoryFile::default(), &doc.daily);
        let rows = (0..7)
            .map(|i| DbRow {
                date: today,
                channel: Some(if i % 2 == 0 {
                    "rg".into()
                } else {
                    "git".into()
                }),
                name: Some(format!("project-{i}")),
                saved: if i < TOP_PROJECTS { 60.0 } else { 50.0 },
                commands: 1,
            })
            .collect();
        let win = &build_windows(&doc, &merged, today, rows).today;
        assert_eq!(win.projects.len(), TOP_PROJECTS);
        let top: f64 = win.projects.iter().map(|p| p.saved_tokens).sum();
        assert_eq!(top, 300.0);
        assert_eq!(win.other_saved, 100.0);
        assert_eq!(win.unattributed_saved, 230.0);
        // Top + other + unattributed always reconstructs the CLI window.
        assert_eq!(
            top + win.other_saved + win.unattributed_saved,
            win.saved_tokens
        );
    }
}
