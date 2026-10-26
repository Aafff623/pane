//! FetchStrategy pipeline skeleton (#65-04). The app still calls providers
//! directly; this module fixes the semantics a later migration must use, so
//! the eight failure classes and the single delayed retry cannot drift per
//! family while the migration happens in batches.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FetchFailureKind {
    AuthExpired,
    MissingCredential,
    PermissionDenied,
    RateLimited,
    ProviderUnavailable,
    ParseFailure,
    NetworkFailure,
    ApiFailure,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchFailure {
    pub kind: FetchFailureKind,
    pub retry_after_ms: Option<i64>,
}

impl FetchFailure {
    pub fn new(kind: FetchFailureKind) -> Self {
        Self { kind, retry_after_ms: None }
    }

    pub fn with_retry_after(mut self, ms: Option<i64>) -> Self {
        self.retry_after_ms = ms;
        self
    }

    pub fn is_transient(&self) -> bool {
        matches!(
            self.kind,
            FetchFailureKind::RateLimited
                | FetchFailureKind::ProviderUnavailable
                | FetchFailureKind::NetworkFailure
                | FetchFailureKind::ApiFailure
        )
    }
}

/// One delayed retry only when the vendor asked for a short wait (≤10 s) and
/// this is the first attempt. Longer hints belong to the refresh gate's
/// bench, not to a tight in-request retry.
pub fn retry_once_allowed(failure: &FetchFailure, attempt: u32) -> bool {
    attempt == 0
        && failure.is_transient()
        && failure
            .retry_after_ms
            .is_some_and(|ms| (0..=10_000).contains(&ms))
}

pub trait FetchStrategy {
    type Output;

    fn is_available(&self) -> bool;

    fn fetch(&self) -> Result<Self::Output, FetchFailure>;

    /// Whether the chain may try the next strategy after this failure.
    fn should_fallback(&self, failure: &FetchFailure) -> bool {
        matches!(
            failure.kind,
            FetchFailureKind::AuthExpired
                | FetchFailureKind::MissingCredential
                | FetchFailureKind::PermissionDenied
                | FetchFailureKind::ProviderUnavailable
                | FetchFailureKind::NetworkFailure
                | FetchFailureKind::ApiFailure
        )
    }

    /// Preserve why a fallback was needed; the UI shows this instead of
    /// pretending the backup path was the first choice.
    fn diagnostic(&self, failure: &FetchFailure) -> Option<String> {
        Some(format!("{:?}", failure.kind))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Unavailable;
    impl FetchStrategy for Unavailable {
        type Output = ();
        fn is_available(&self) -> bool {
            false
        }
        fn fetch(&self) -> Result<(), FetchFailure> {
            Err(FetchFailure::new(FetchFailureKind::MissingCredential))
        }
    }

    struct Primary;
    impl FetchStrategy for Primary {
        type Output = &'static str;
        fn is_available(&self) -> bool {
            true
        }
        fn fetch(&self) -> Result<&'static str, FetchFailure> {
            Err(FetchFailure::new(FetchFailureKind::ProviderUnavailable))
        }
        fn diagnostic(&self, failure: &FetchFailure) -> Option<String> {
            Some(format!("primary failed: {:?}", failure.kind))
        }
    }

    #[test]
    fn availability_skips_a_strategy_without_calling_it() {
        let strategy = Unavailable;
        assert!(!strategy.is_available());
    }

    #[test]
    fn fallback_and_diagnostic_semantics_are_explicit() {
        let primary = Primary;
        let failure = FetchFailure::new(FetchFailureKind::ProviderUnavailable);
        assert!(primary.should_fallback(&failure));
        assert_eq!(
            primary.diagnostic(&failure).as_deref(),
            Some("primary failed: ProviderUnavailable")
        );
        assert!(!primary.should_fallback(&FetchFailure::new(FetchFailureKind::ParseFailure)));
    }

    #[test]
    fn delayed_retry_is_one_attempt_and_short_only() {
        let short = FetchFailure::new(FetchFailureKind::RateLimited).with_retry_after(Some(5_000));
        assert!(retry_once_allowed(&short, 0));
        assert!(!retry_once_allowed(&short, 1));
        let long = FetchFailure::new(FetchFailureKind::RateLimited).with_retry_after(Some(30_000));
        assert!(!retry_once_allowed(&long, 0));
        assert!(!retry_once_allowed(
            &FetchFailure::new(FetchFailureKind::AuthExpired).with_retry_after(Some(1_000)),
            0
        ));
    }
}
