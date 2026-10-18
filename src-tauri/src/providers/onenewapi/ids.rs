use base64::Engine;
use std::collections::HashSet;

/// 16 OS-random bytes, URL-safe base64 without padding.
pub fn new_id() -> Result<String, String> {
    let mut bytes = [0u8; 16];
    crate::platform::fill_random(&mut bytes)?;
    Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes))
}

pub fn new_id_avoiding(occupied: &HashSet<String>) -> Result<String, String> {
    for _ in 0..32 {
        let id = new_id()?;
        if !occupied.contains(&id) {
            return Ok(id);
        }
    }
    Err("could not allocate a unique id".into())
}

#[cfg(test)]
mod tests {
    use super::{new_id, new_id_avoiding};
    use std::collections::HashSet;

    #[test]
    fn ids_are_url_safe_unpadded_and_unique() {
        let a = new_id().unwrap();
        let b = new_id().unwrap();
        assert_ne!(a, b);
        for id in [&a, &b] {
            assert_eq!(id.len(), 22);
            assert!(!id.contains('='));
            assert!(id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
        }
    }

    #[test]
    fn avoiding_retries_until_free() {
        let id = new_id().unwrap();
        let occupied = HashSet::from([id.clone()]);
        let next = new_id_avoiding(&occupied).unwrap();
        assert_ne!(next, id);
    }
}
