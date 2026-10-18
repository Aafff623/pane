use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UsageUnit {
    Tokens,
    Credits,
    Minutes,
    Requests,
}

/// An observed cumulative counter, never a balance or a percentage.
/// Scopes identify independent meters on the same card (for example key
/// counters); a missing scope is the card's primary meter.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UsageReading {
    pub unit: UsageUnit,
    pub used: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total: Option<f64>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub scope: String,
}

/// Vendor-reported daily buckets, distinct from cumulative meter readings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UsageDays {
    pub unit: UsageUnit,
    pub scope: String,
    pub days: std::collections::BTreeMap<String, f64>,
}

impl UsageReading {
    pub fn valid(&self) -> bool {
        self.used.is_finite()
            && self.used >= 0.0
            && self
                .total
                .is_none_or(|total| total.is_finite() && total >= 0.0)
    }
}
