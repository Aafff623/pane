//! Read-only local sign-in discovery for the auth center. Provider modules
//! already know how to spot the official clients' credential stores (the
//! same probes the Customize gear panel uses); this file maps those probes
//! to auth-center groups so a machine that is already signed in shows up
//! without a second login. Nothing is copied or persisted here.
//!
//! Kept free of Tauri types so the parse-tests harness compiles it via
//! #[path], same convention as auth_center.rs.

#[derive(Clone, Debug, PartialEq)]
pub struct LocalLogin {
    /// Card id the usage snapshot is cached against.
    pub id: &'static str,
    pub label: String,
    pub email: Option<String>,
    /// Which local store the row came from, shown under the label.
    pub hint: String,
    /// CN editions are separate products sharing a group id — always list
    /// them even when Pane also manages an account in the same group.
    pub always_listed: bool,
}

fn row(
    id: &'static str,
    label: &str,
    hint: Option<String>,
    email: Option<String>,
    always_listed: bool,
) -> Option<LocalLogin> {
    hint.map(|hint| LocalLogin {
        id,
        label: label.to_string(),
        email,
        hint,
        always_listed,
    })
}

/// The local sign-ins that belong to one auth-center group. Called once per
/// family, so a row never needs to carry its group id.
pub fn candidates(group: &str) -> Vec<LocalLogin> {
    match group {
        "qoder" => [
            row(
                "qoder",
                "Qoder",
                crate::providers::qoder::local_credential_hint(),
                None,
                false,
            ),
            row(
                "qodercn",
                "Qoder CN",
                crate::providers::qodercn::local_credential_hint(),
                None,
                true,
            ),
        ]
        .into_iter()
        .flatten()
        .collect(),
        "trae" => [
            row(
                "trae",
                "Trae",
                crate::providers::trae::local_credential_hint(),
                None,
                false,
            ),
            row(
                "traecn",
                "Trae CN",
                crate::providers::traecn::local_credential_hint(),
                None,
                true,
            ),
        ]
        .into_iter()
        .flatten()
        .collect(),
        "codex" => {
            let Some(hint) = crate::providers::codex::local_credential_hint() else {
                return Vec::new();
            };
            let email = crate::providers::codex::default_account_info()
                .and_then(|(_, email)| email)
                .filter(|e| !e.trim().is_empty());
            let label = email.as_deref().unwrap_or("Codex CLI");
            vec![LocalLogin {
                id: "codex",
                label: label.to_string(),
                email,
                hint,
                always_listed: false,
            }]
        }
        "cursor" | "copilot" | "kiro" | "antigravity" => {
            let (id, label, hint) = match group {
                "cursor" => ("cursor", "Cursor", crate::providers::cursor::local_credential_hint()),
                "copilot" => (
                    "copilot",
                    "GitHub Copilot",
                    crate::providers::copilot::local_credential_hint(),
                ),
                "kiro" => ("kiro", "Kiro", crate::providers::kiro::local_credential_hint()),
                _ => (
                    "antigravity",
                    "Antigravity",
                    crate::providers::antigravity::local_credential_hint(),
                ),
            };
            row(id, label, hint, None, false).into_iter().collect()
        }
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_have_card_ids_and_hints() {
        for group in ["qoder", "trae", "codex", "cursor", "copilot", "kiro", "antigravity"] {
            for row in candidates(group) {
                assert!(!row.id.is_empty());
                assert!(!row.label.is_empty());
                assert!(!row.hint.is_empty());
            }
        }
    }

    #[test]
    fn families_without_local_probes_list_nothing() {
        for group in ["", "zed", "windsurf", "grok"] {
            assert!(candidates(group).is_empty(), "{group} should have no local rows");
        }
    }

    #[test]
    fn cn_editions_are_marked_always_visible() {
        let all: Vec<LocalLogin> = ["qoder", "trae"].into_iter().flat_map(candidates).collect();
        for row in all.iter().filter(|row| row.id.ends_with("cn")) {
            assert!(row.always_listed, "{} must always be listed", row.id);
        }
    }
}
