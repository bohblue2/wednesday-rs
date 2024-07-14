use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Copy)]
pub struct MarketMeta {
    pub close: f64,
    pub timestamp: DateTime<Utc>,
}

impl Default for MarketMeta {
    fn default() -> Self {
        Self {
            close: 0.0,
            timestamp: Utc::now(),
        }
    }
}