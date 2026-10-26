//! Per-account token-refresh lock (#63-05). The registry is keyed by
//! (family, account); production callers hold the lock across the token
//! exchange's network await, so it is a `tokio::sync::Mutex` — a std guard
//! held across an await would make the refresh future non-Send and could
//! stall the executor. No cross-process file lock: Pane has no shared
//! credential directory.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

#[derive(Debug, PartialEq, Eq)]
pub enum RefreshDecision {
    /// Another task already refreshed this account; reuse its result.
    Reuse,
    /// Still stale after the wait, or nobody refreshed it: do the work.
    Refresh,
}

/// Double-check after acquiring the per-account lock. A generation move plus
/// a valid token means the refresh_token may already have been rotated and
/// consumed by the other task — refreshing again would burn it.
pub fn decide_after_lock(
    observed_generation: u64,
    current_generation: u64,
    token_valid: bool,
) -> RefreshDecision {
    if current_generation > observed_generation && token_valid {
        RefreshDecision::Reuse
    } else {
        RefreshDecision::Refresh
    }
}

pub fn lock_key(family: &str, account_id: &str) -> String {
    format!("{}::{account_id}", family.trim().to_lowercase())
}

fn registry() -> &'static Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>> {
    static REGISTRY: OnceLock<Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn account_lock(family: &str, account_id: &str) -> Arc<tokio::sync::Mutex<()>> {
    let key = lock_key(family, account_id);
    registry()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .entry(key)
        .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn same_account_shares_one_lock() {
        assert!(Arc::ptr_eq(
            &account_lock("codex", "acct-1"),
            &account_lock("CODEX", "acct-1")
        ));
        assert!(!Arc::ptr_eq(
            &account_lock("codex", "acct-1"),
            &account_lock("codex", "acct-2")
        ));
    }

    #[test]
    fn double_check_reuses_a_rotated_valid_token() {
        assert_eq!(decide_after_lock(4, 5, true), RefreshDecision::Reuse);
        assert_eq!(decide_after_lock(5, 5, true), RefreshDecision::Refresh);
        assert_eq!(decide_after_lock(4, 5, false), RefreshDecision::Refresh);
        assert_eq!(decide_after_lock(5, 4, true), RefreshDecision::Refresh);
    }

    #[test]
    fn two_waiters_trigger_exactly_one_refresh() {
        let lock = account_lock("test", "concurrent");
        let refreshes = Arc::new(AtomicUsize::new(0));
        let generation = Arc::new(AtomicUsize::new(1));
        let mut handles = Vec::new();
        for observed in [1u64, 1u64] {
            let lock = lock.clone();
            let refreshes = refreshes.clone();
            let generation = generation.clone();
            handles.push(std::thread::spawn(move || {
                // Plain std threads (no tokio runtime here) → blocking_lock.
                let _guard = lock.blocking_lock();
                let current = generation.load(Ordering::SeqCst) as u64;
                let token_valid = current > observed;
                if decide_after_lock(observed, current, token_valid) == RefreshDecision::Refresh {
                    refreshes.fetch_add(1, Ordering::SeqCst);
                    generation.fetch_add(1, Ordering::SeqCst);
                }
            }));
        }
        for handle in handles {
            handle.join().unwrap();
        }
        assert_eq!(refreshes.load(Ordering::SeqCst), 1);
        assert_eq!(generation.load(Ordering::SeqCst), 2);
    }
}
