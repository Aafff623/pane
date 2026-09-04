//! Antigravity credential slots — captured snapshots of the IDE's Google
//! OAuth token bundle, one per Google account the user wants to monitor.
//! The IDE itself keeps exactly one token (Windows Credential Manager,
//! `gemini:antigravity`), so "multiple accounts" means capturing that
//! bundle into a slot per account and refreshing each independently with
//! its own refresh token. Storage mirrors accounts.rs conventions and is
//! kept free of Tauri types.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// One captured Google account: the OAuth bundle Antigravity stores, plus
/// a display label. `refresh_token` is the long-lived piece — access
/// tokens are refreshed from it at query time.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct AgSlot {
    #[serde(default)]
    pub label: String,
    #[serde(rename = "refreshToken", default)]
    pub refresh_token: String,
    #[serde(rename = "accessToken", default)]
    pub access_token: String,
    /// RFC3339 expiry of the captured access token (advisory; slots
    /// refresh whenever the stored access is missing/expired).
    #[serde(default)]
    pub expires_at: Option<String>,
    /// Unix seconds when the slot was captured.
    #[serde(default)]
    pub captured_at: i64,
}

fn slots_file(base: &Path) -> PathBuf {
    base.join("antigravity-accounts.json")
}

/// Tolerates a UTF-8 BOM like every other Pane JSON store.
pub fn parse_slots(raw: &str) -> Vec<AgSlot> {
    serde_json::from_str(raw.trim_start_matches('\u{feff}')).unwrap_or_default()
}

pub fn load_slots_from(base: &Path) -> Vec<AgSlot> {
    let raw = std::fs::read_to_string(slots_file(base)).unwrap_or_default();
    let mut out = parse_slots(&raw);
    out.retain(|s| !s.refresh_token.trim().is_empty());
    out
}

pub fn save_slots_to(base: &Path, slots: &[AgSlot]) -> Result<(), String> {
    std::fs::create_dir_all(base).map_err(|e| format!("create config dir: {e}"))?;
    std::fs::write(
        slots_file(base),
        serde_json::to_string_pretty(slots).unwrap_or_default(),
    )
    .map_err(|e| format!("write antigravity slots: {e}"))
}

pub fn load_slots() -> Vec<AgSlot> {
    load_slots_from(&crate::providers::config_dir())
}

pub fn save_slots(slots: &[AgSlot]) -> Result<(), String> {
    save_slots_to(&crate::providers::config_dir(), slots)
}

fn fnv1a(bytes: &[u8], mut hash: u64) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x1000_0000_01b3);
    }
    hash
}

/// Stable card id for a slot: keyed on the refresh token (the identity
/// that survives access-token rotation). Same two-lane FNV scheme as
/// accounts.rs so ids look uniform and collisions stay vanishingly rare.
pub fn card_id_for_slot(slot: &AgSlot) -> String {
    let material = slot.refresh_token.trim().as_bytes().to_vec();
    let left = fnv1a(&material, 0xcbf2_9ce4_8422_2325);
    let mut second = b"pane-antigravity-slot-v1\0".to_vec();
    second.extend_from_slice(&material);
    let right = fnv1a(&second, 0x8422_2325_cbf2_9ce4);
    format!("antigravity@{left:016x}{right:016x}")
}

/// "ya29…abcd" style mask: a recognizable head, ellipsis, last 4 chars.
pub fn mask_token(token: &str) -> String {
    let token = token.trim();
    let chars: Vec<char> = token.chars().collect();
    if chars.len() < 4 {
        return "…".into();
    }
    let tail: String = chars[chars.len() - 4..].iter().collect();
    if chars.len() >= 12 {
        format!("{}…{tail}", chars[..4].iter().collect::<String>())
    } else {
        format!("…{tail}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slots_roundtrip_and_drop_empty_refresh() {
        let base = std::env::temp_dir().join(format!("pane-ag-slots-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let slots = vec![
            AgSlot {
                label: "pro-a".into(),
                refresh_token: "1//refresh-a".into(),
                access_token: "ya29.aaa".into(),
                expires_at: None,
                captured_at: 1,
            },
            AgSlot {
                label: "broken".into(),
                refresh_token: String::new(),
                access_token: String::new(),
                expires_at: None,
                captured_at: 2,
            },
        ];
        save_slots_to(&base, &slots).expect("write");
        let loaded = load_slots_from(&base);
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].label, "pro-a");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn slot_ids_are_stable_across_access_rotation() {
        let mut a = AgSlot {
            label: "a".into(),
            refresh_token: "1//same".into(),
            access_token: "ya29.old".into(),
            expires_at: None,
            captured_at: 0,
        };
        let id_before = card_id_for_slot(&a);
        a.access_token = "ya29.rotated".into();
        assert_eq!(card_id_for_slot(&a), id_before);
        a.refresh_token = "1//other".into();
        assert_ne!(card_id_for_slot(&a), id_before);
    }

    #[test]
    fn masking_shows_head_and_tail() {
        assert_eq!(mask_token("ya29.long-token-abcd"), "ya29…abcd");
        assert_eq!(mask_token("abc"), "…");
    }
}
