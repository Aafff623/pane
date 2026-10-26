//! Structured command errors (#62-05). The public IPC contract remains a
//! String, but high-frequency failures can now carry a machine-readable code,
//! context and suggestion without leaking URLs or credentials.

use serde::Serialize;
use std::fmt;

#[derive(Debug)]
pub enum AppError {
    Io { path: String, source: std::io::Error },
    Localized { code: &'static str, en: String, zh: String },
}

impl fmt::Display for AppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => write!(formatter, "io {path}: {source}"),
            Self::Localized { en, .. } => formatter.write_str(en),
        }
    }
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Localized { .. } => None,
        }
    }
}

impl AppError {
    pub fn localized(code: &'static str, en: impl Into<String>, zh: impl Into<String>) -> Self {
        Self::Localized { code, en: en.into(), zh: zh.into() }
    }

    pub fn display(&self, locale: &str) -> String {
        match self {
            Self::Localized { en, zh, .. } if locale == "zh" => zh.clone(),
            Self::Localized { en, .. } => en.clone(),
            Self::Io { path, source } => format!("{path}: {source}"),
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::Io { .. } => "io",
            Self::Localized { code, .. } => code,
        }
    }
}

impl From<AppError> for String {
    fn from(error: AppError) -> Self {
        error.to_string()
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Structured<'a> {
    code: &'a str,
    context: &'a str,
    suggestion: &'a str,
}

/// Structured payload for IPC callers that can parse JSON; ordinary commands
/// keep returning the plain sentence.
pub fn structured(code: &str, context: &str, suggestion: &str) -> String {
    serde_json::to_string(&Structured { code, context, suggestion })
        .unwrap_or_else(|_| format!("{code}: {context}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn localized_errors_choose_the_requested_language() {
        let error = AppError::localized("port_busy", "Port 6736 is in use", "端口 6736 被占用");
        assert_eq!(error.display("zh"), "端口 6736 被占用");
        assert_eq!(error.display("en"), "Port 6736 is in use");
        assert_eq!(error.code(), "port_busy");
    }

    #[test]
    fn io_errors_keep_the_path_and_source() {
        let error = AppError::Io {
            path: "C:\\Pane\\config.json".into(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "missing"),
        };
        let text: String = error.into();
        assert!(text.contains("config.json"));
        assert!(text.contains("missing"));
    }

    #[test]
    fn structured_payload_is_json() {
        let value: serde_json::Value =
            serde_json::from_str(&structured("rate_limited", "claude", "wait 30s")).unwrap();
        assert_eq!(value["code"], "rate_limited");
        assert_eq!(value["suggestion"], "wait 30s");
    }
}
