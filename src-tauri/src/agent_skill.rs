//! Ship the `pane` troubleshooting skill to the shared agent-skills
//! directory, REA-style: show the plan first, back up what already exists,
//! write only on approval, and stay idempotent.
//!
//! The skill lives in the repo under `skills/pane/` and is compiled into the
//! binary with `include_str!`, so the button works on a machine that only has
//! the installed app (no repo checkout). Content is the single source: the
//! repo copy, the installer download and this button all ship the same bytes.

use std::path::PathBuf;

/// (relative path inside the skill dir, contents) — the same five files the
/// repo and install.ps1 distribute.
const SKILL_FILES: &[(&str, &str)] = &[
    ("SKILL.md", include_str!("../../skills/pane/SKILL.md")),
    (
        "references/symptoms.md",
        include_str!("../../skills/pane/references/symptoms.md"),
    ),
    (
        "references/providers.md",
        include_str!("../../skills/pane/references/providers.md"),
    ),
    (
        "references/spend.md",
        include_str!("../../skills/pane/references/spend.md"),
    ),
    (
        "references/update.md",
        include_str!("../../skills/pane/references/update.md"),
    ),
];

/// `~/.agents/skills/pane` — the shared directory every major agent reads
/// (Claude Code, Codex, Cursor, ZCode, Qwen…), so one copy covers them all.
pub fn target_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".agents").join("skills").join("pane"))
}

/// What an install would do — one human-readable line per action. Pure
/// read-only; the UI shows this and asks before calling [`install`].
pub fn plan() -> Result<Vec<String>, String> {
    let dir = target_dir().ok_or("cannot locate the home directory")?;
    let mut lines = vec![format!("target: {}", dir.display())];
    let mut changes = 0;
    for (rel, contents) in SKILL_FILES {
        let path = dir.join(rel);
        match std::fs::read_to_string(&path) {
            Ok(existing) if existing == *contents => lines.push(format!("keep   {rel} (already current)")),
            Ok(_) => {
                changes += 1;
                lines.push(format!("update {rel} (backs up the existing file to {rel}.bak)"));
            }
            Err(_) => {
                changes += 1;
                lines.push(format!("create {rel}"));
            }
        }
    }
    if changes == 0 {
        lines.push("nothing to do — the skill is already installed and current".into());
    }
    Ok(lines)
}

/// Execute the plan. Backups are written once per file (`.bak` is never
/// overwritten, matching the "keep the first backup" convention), and the
/// whole thing is idempotent: identical content is left untouched.
pub fn install() -> Result<Vec<String>, String> {
    let dir = target_dir().ok_or("cannot locate the home directory")?;
    let mut out = Vec::new();
    for (rel, contents) in SKILL_FILES {
        let path = dir.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
        }
        match std::fs::read_to_string(&path) {
            Ok(existing) if existing == *contents => {
                out.push(format!("kept {rel} (already current)"));
                continue;
            }
            Ok(_) => {
                let bak = path.with_extension("md.bak");
                if !bak.exists() {
                    std::fs::copy(&path, &bak).map_err(|e| format!("backup {}: {e}", bak.display()))?;
                    out.push(format!("backed up {rel} → {}", bak.display()));
                }
            }
            Err(_) => {}
        }
        std::fs::write(&path, contents).map_err(|e| format!("write {}: {e}", path.display()))?;
        out.push(format!("wrote {rel}"));
    }
    out.push(format!("done — skill at {}", dir.display()));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_shipped_file_is_nonempty_and_named() {
        assert_eq!(SKILL_FILES.len(), 5);
        for (rel, contents) in SKILL_FILES {
            assert!(!contents.trim().is_empty(), "{rel} is empty");
            assert!(*rel == "SKILL.md" || rel.starts_with("references/"), "{rel}");
        }
        // The frontmatter name must match the parent directory (spec rule).
        let skill = SKILL_FILES[0].1;
        assert!(skill.starts_with("---\nname: pane\n"), "frontmatter name must be `pane`");
        // Progressive disclosure: the body stays well under the 500-line cap.
        assert!(skill.lines().count() < 200, "SKILL.md grew past its budget");
    }

    #[test]
    fn plan_targets_the_shared_agent_skills_dir() {
        if let Some(dir) = target_dir() {
            assert!(dir.ends_with("skills/pane") || dir.ends_with("skills\\pane"), "{}", dir.display());
        }
    }
}
