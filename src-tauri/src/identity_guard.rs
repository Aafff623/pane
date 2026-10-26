//! Quota-response identity guard (#63-02). A provider only compares when
//! both sides carry an identity; a missing field is never treated as a
//! mismatch (older endpoints and relay cards legitimately omit it). Credential
//! fingerprints are derived, never logged as raw secrets.

use sha2::{Digest, Sha256};

#[derive(Debug, PartialEq, Eq)]
pub enum IdentityVerdict {
    Match,
    Mismatch,
    Missing,
}

pub fn check_identity(expected: Option<&str>, actual: Option<&str>) -> IdentityVerdict {
    let expected = expected.map(str::trim).filter(|value| !value.is_empty());
    let actual = actual.map(str::trim).filter(|value| !value.is_empty());
    match (expected, actual) {
        (Some(expected), Some(actual)) if expected == actual => IdentityVerdict::Match,
        (Some(_), Some(_)) => IdentityVerdict::Mismatch,
        _ => IdentityVerdict::Missing,
    }
}

/// sha256 prefix + length + expiry: enough to tell two credentials apart in a
/// log without containing any part of the secret itself.
pub fn credential_fingerprint(secret: &str, expires_at: Option<&str>) -> String {
    let digest = Sha256::digest(secret.as_bytes());
    let prefix: String = digest[..4].iter().map(|byte| format!("{byte:02x}")).collect();
    format!(
        "{prefix}:len={}:exp={}",
        secret.chars().count(),
        expires_at.unwrap_or("-")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_comparison_is_three_valued() {
        assert_eq!(check_identity(Some("u-1"), Some("u-1")), IdentityVerdict::Match);
        assert_eq!(check_identity(Some("u-1"), Some("u-2")), IdentityVerdict::Mismatch);
        assert_eq!(check_identity(None, Some("u-2")), IdentityVerdict::Missing);
        assert_eq!(check_identity(Some(" "), Some("u-2")), IdentityVerdict::Missing);
        assert_eq!(check_identity(Some("u-1"), None), IdentityVerdict::Missing);
    }

    #[test]
    fn fingerprint_is_stable_and_never_contains_the_secret() {
        let secret = "sk-live-abcdef123456";
        let first = credential_fingerprint(secret, Some("2026-11-01T00:00:00Z"));
        let second = credential_fingerprint(secret, Some("2026-11-01T00:00:00Z"));
        assert_eq!(first, second);
        assert!(!first.contains(secret));
        assert!(first.contains(&format!("len={}", secret.len())));
        assert!(first.contains("exp=2026-11-01T00:00:00Z"));
        assert_ne!(
            first,
            credential_fingerprint("sk-live-abcdef123457", Some("2026-11-01T00:00:00Z"))
        );
    }
}
