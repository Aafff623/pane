#[cfg(test)]
mod tests {
    use crate::keyvault_v3::crypto::{recovery_code_extract, recovery_code_normalize};

    #[test]
    fn recovery_normalization_matches_the_shared_frontend_vectors() {
        let cases: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../tests/fixtures/recovery-normalization.json"
        )).unwrap();
        for case in cases {
            assert_eq!(
                recovery_code_normalize(case["input"].as_str().unwrap()),
                case["normalized"].as_str().unwrap(),
            );
        }
    }

    #[test]
    fn recovery_export_text_skips_invalid_shaped_lines() {
        let code = "0123456789ABCDEFGHJKMNPQRSTVWXYZ01";
        assert_eq!(recovery_code_extract(&format!("{}\n{code}\nKeep offline", "U".repeat(34))), Some(code.into()));
        assert_eq!(recovery_code_extract(&format!("{}ß", "1".repeat(32))), None);
    }
}
