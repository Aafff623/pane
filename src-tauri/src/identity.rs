//! Shared stable card identity derivation for account-like stores.
//!
//! The caller supplies the family, identity material and namespace. Keeping
//! the hashing policy here prevents account stores from drifting while their
//! persisted card ids remain byte-stable.

fn fnv1a(bytes: &[u8], mut hash: u64) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x1000_0000_01b3);
    }
    hash
}

pub(crate) fn stable_card_id(family: &str, namespace: &[u8], material: &[u8]) -> String {
    let left = fnv1a(material, 0xcbf2_9ce4_8422_2325);
    let mut second = namespace.to_vec();
    second.extend_from_slice(material);
    let right = fnv1a(&second, 0x8422_2325_cbf2_9ce4);
    format!("{family}@{left:016x}{right:016x}")
}

#[cfg(test)]
mod tests {
    use super::stable_card_id;

    #[test]
    fn stable_ids_preserve_pre_extraction_store_vectors() {
        for (family, namespace, material, expected) in [
            ("deepseek", b"pane-account-id-v1\0".as_slice(), b"synthetic-key\0".as_slice(),
             "deepseek@b4293a30657cd5223274c9b37a222d17"),
            ("antigravity", b"pane-antigravity-slot-v1\0", b"synthetic-refresh",
             "antigravity@8fa7e9ca737543f2a7b335d3c05740ef"),
            ("cursor", b"pane-cursor-account-v1\0", b"synthetic-access",
             "cursor@775078b60040b03d99fa5ed5eb27d38b"),
            ("copilot", b"pane-account-id-v1\0", b"synthetic-user\0",
             "copilot@82a49a8fd5850a1457549f8176af5deb"),
        ] {
            assert_eq!(stable_card_id(family, namespace, material), expected);
        }
    }
}
