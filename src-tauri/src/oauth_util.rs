//! Protocol-independent helpers shared by every browser sign-in flow.
//!
//! These are pure: percent-coding, base64url, PKCE S256, random tokens and
//! the loopback callback page. Vendor-specific state machines, endpoints and
//! token exchanges stay in their own modules — only the identical arithmetic
//! lives here.

use base64::Engine;
use rand_core::{OsRng, RngCore};

pub fn b64url(bytes: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

/// `n` CSPRNG bytes, base64url-encoded without padding.
pub fn random_b64url(n: usize) -> String {
    let mut bytes = vec![0u8; n];
    OsRng.fill_bytes(&mut bytes);
    b64url(&bytes)
}

/// 16 CSPRNG bytes rendered as a UUID-shaped string, matching the value
/// cockpit hands vendors that expect one.
pub fn random_uuid_like() -> String {
    let mut bytes = [0u8; 16];
    OsRng.fill_bytes(&mut bytes);
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

/// RFC 7636 S256: base64url(SHA-256(verifier)).
pub fn code_challenge(verifier: &str) -> String {
    use sha2::{Digest, Sha256};
    b64url(&Sha256::digest(verifier.as_bytes()))
}

pub fn pct_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

pub fn pct_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
                if let Ok(v) = u8::from_str_radix(hex, 16) {
                    out.push(v);
                    i += 3;
                    continue;
                }
                out.push(b'%');
                i += 1;
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The page a vendor's browser tab lands on after the loopback callback.
/// Same shell for every flow; only the outcome text differs.
pub fn callback_html(ok: bool) -> String {
    let (fg, text) = if ok {
        ("#4ade80", "Sign-in complete — you can close this tab and return to Pane.")
    } else {
        ("#f87171", "Sign-in failed — return to Pane and try again.")
    };
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>Pane</title></head>\
         <body style=\"font-family:sans-serif;background:#18181b;color:{fg};display:flex;\
         align-items:center;justify-content:center;height:100vh;margin:0\"><p>{text}</p></body></html>"
    )
}

/// First request line of a loopback callback → (code, state).
pub fn parse_callback_request(request_line: &str) -> Option<(String, String)> {
    let path = request_line.split_whitespace().nth(1)?;
    let query = path.split_once('?')?.1;
    let mut code = None;
    let mut state = None;
    for pair in query.split('&') {
        let Some((key, value)) = pair.split_once('=') else {
            continue;
        };
        match key {
            "code" => code = Some(pct_decode(value)),
            "state" => state = Some(pct_decode(value)),
            _ => {}
        }
    }
    Some((code?, state?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_challenge_matches_the_rfc_7636_vector() {
        assert_eq!(
            code_challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn percent_coding_round_trips_and_keeps_unreserved_bytes() {
        assert_eq!(pct_encode("a b/c?d=e&f"), "a%20b%2Fc%3Fd%3De%26f");
        assert_eq!(pct_decode("a%20b%2Fc%3Fd%3De%26f"), "a b/c?d=e&f");
        assert_eq!(pct_decode("a+b"), "a b");
        // A truncated or non-hex escape stays literal instead of panicking.
        assert_eq!(pct_decode("100%"), "100%");
        assert_eq!(pct_decode("%zz"), "%zz");
    }

    #[test]
    fn callback_parsing_needs_both_code_and_state() {
        let (code, state) =
            parse_callback_request("GET /auth/callback?code=ac%2F1&state=st-42 HTTP/1.1").unwrap();
        assert_eq!(code, "ac/1");
        assert_eq!(state, "st-42");
        assert!(parse_callback_request("GET /auth/callback HTTP/1.1").is_none());
        assert!(parse_callback_request("GET /auth/callback?code=only HTTP/1.1").is_none());
    }

    #[test]
    fn random_tokens_are_url_safe_and_the_uuid_shape_is_stable() {
        let token = random_b64url(24);
        assert!(!token.contains('=') && !token.contains('+') && !token.contains('/'));
        assert_eq!(random_b64url(16).len(), 22);
        let uuid = random_uuid_like();
        assert_eq!(uuid.len(), 36);
        assert_eq!(uuid.matches('-').count(), 4);
        assert_ne!(random_uuid_like(), uuid);
    }

    #[test]
    fn callback_page_reports_success_and_failure_distinctly() {
        let ok = callback_html(true);
        let bad = callback_html(false);
        assert!(ok.contains("Sign-in complete") && ok.contains("#4ade80"));
        assert!(bad.contains("Sign-in failed") && bad.contains("#f87171"));
    }
}
