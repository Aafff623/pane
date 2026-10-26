//! Adaptive refresh cadence (#63-03): tighten the background loop only when a
//! quota is nearly exhausted or about to reset. Backoff always outranks
//! acceleration — a provider in a rate-limit cooldown must not be polled more
//! often just because another card is low.

pub const FAST_INTERVAL_MS: i64 = 60_000;
pub const LOW_REMAINING_PERCENT: f64 = 20.0;
pub const NEAR_RESET_MS: i64 = 30 * 60 * 1000;

/// `base_ms` is the user's configured cadence. `remaining_percent` is the
/// lowest remaining share across healthy cards; `ms_to_reset` is the nearest
/// future reset. A backoff anywhere ends acceleration for this tick.
pub fn adaptive_interval_ms(
    base_ms: i64,
    remaining_percent: Option<f64>,
    ms_to_reset: Option<i64>,
    backoff_until_ms: i64,
    now_ms: i64,
) -> i64 {
    let base = base_ms.max(FAST_INTERVAL_MS);
    if backoff_until_ms > now_ms {
        return base;
    }
    let low = remaining_percent.is_some_and(|percent| percent < LOW_REMAINING_PERCENT);
    let near_reset = ms_to_reset.is_some_and(|ms| ms > 0 && ms <= NEAR_RESET_MS);
    if low || near_reset {
        FAST_INTERVAL_MS
    } else {
        base
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urgency_uses_the_documented_thresholds() {
        let base = 5 * 60_000;
        assert_eq!(adaptive_interval_ms(base, Some(19.9), None, 0, 0), FAST_INTERVAL_MS);
        assert_eq!(adaptive_interval_ms(base, Some(20.0), None, 0, 0), base);
        assert_eq!(adaptive_interval_ms(base, None, Some(NEAR_RESET_MS), 0, 0), FAST_INTERVAL_MS);
        assert_eq!(adaptive_interval_ms(base, None, Some(NEAR_RESET_MS + 1), 0, 0), base);
        assert_eq!(adaptive_interval_ms(base, Some(90.0), None, 0, 0), base);
    }

    #[test]
    fn an_active_backoff_blocks_acceleration() {
        let base = 5 * 60_000;
        assert_eq!(
            adaptive_interval_ms(base, Some(2.0), Some(60_000), 10_000, 5_000),
            base
        );
        // Once the cooldown has expired, urgency may speed the loop again.
        assert_eq!(
            adaptive_interval_ms(base, Some(2.0), Some(60_000), 5_000, 10_000),
            FAST_INTERVAL_MS
        );
    }

    #[test]
    fn cadence_never_drops_below_one_minute() {
        assert_eq!(adaptive_interval_ms(1_000, None, None, 0, 0), FAST_INTERVAL_MS);
    }
}
