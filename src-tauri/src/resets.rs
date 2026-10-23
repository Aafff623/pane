//! Pure reset-event detection. A large usage drop is not enough on its own:
//! the reset timestamp must have passed or moved forward, and the account/
//! credential/source identity must be unchanged.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Reading {
    pub used_percent: f64,
    pub resets_at: Option<i64>,
    pub period_ms: Option<i64>,
}

pub fn is_reset(previous: Reading, current: Reading, same_identity: bool) -> bool {
    if !same_identity || !previous.used_percent.is_finite() || !current.used_percent.is_finite() {
        return false;
    }
    if previous.used_percent - current.used_percent < 10.0 {
        return false;
    }
    match (previous.resets_at, current.resets_at) {
        (Some(old), Some(new)) => {
            let moved = new - old;
            let window_shift = current.period_ms.map(|period| period / 10).unwrap_or(0);
            moved >= 10 * 60_000 || moved >= window_shift
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(used: f64, reset: Option<i64>, period: Option<i64>) -> Reading {
        Reading { used_percent: used, resets_at: reset, period_ms: period }
    }

    #[test]
    fn a_drop_with_a_moved_reset_is_a_reset() {
        assert!(is_reset(read(90.0, Some(1_000_000), Some(3_600_000)), read(10.0, Some(1_000_000 + 20 * 60_000), Some(3_600_000)), true));
    }

    #[test]
    fn a_drop_without_reset_movement_is_not_a_reset() {
        assert!(!is_reset(read(90.0, Some(1_000_000), Some(3_600_000)), read(10.0, Some(1_000_000 + 60_000), Some(3_600_000)), true));
    }

    #[test]
    fn identity_change_vetoes_the_event() {
        assert!(!is_reset(read(90.0, Some(1_000_000), Some(3_600_000)), read(10.0, Some(1_200_000), Some(3_600_000)), false));
    }

    #[test]
    fn small_drops_do_not_qualify() {
        assert!(!is_reset(read(50.0, Some(1_000_000), Some(3_600_000)), read(45.0, Some(1_200_000), Some(3_600_000)), true));
    }
}
