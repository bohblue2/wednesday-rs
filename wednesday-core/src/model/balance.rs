use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::{timestamp, Uuid};

#[derive(Debug, Copy, PartialOrd, PartialEq, Clone, Serialize, Deserialize)]
pub struct Balance {
    pub timestamp: DateTime<Utc>,
    pub total: f64,
    pub available: f64,
}

impl Default for Balance {
    fn default() -> Self {
        Self {
            timestamp: Utc::now(),
            total: 0.0,
            available: 0.0,
        }
    }
}

pub type BalanceId = String;

impl Balance {
    pub fn new(timestamp: DateTime<Utc>, total: f64, available: f64) -> Self {
        Self {
            timestamp,
            total,
            available,
        }
    }

    pub fn balance_id(engine_id: Uuid) -> BalanceId {
        format!("{}_balance", engine_id)
    }
}
