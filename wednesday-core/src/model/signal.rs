use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use wednesday_model::{
    identifiers::{Exchange, Market},
    instruments::Instrument,
};

use super::{decision::Decision, market_meta::MarketMeta};

#[derive(Debug, Clone)]
pub struct Signal {
    pub datetime: DateTime<Utc>,
    pub exchange: Exchange,
    pub instrument: Instrument,
    pub signals: HashMap<Decision, SignalStrength>,
    pub market_meta: MarketMeta,
}

#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct SignalStrength(pub f64);

#[derive(Clone, Eq, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct SignalForceExit {
    pub datetime: DateTime<Utc>,
    pub exchange: Exchange,
    pub instrument: Instrument,
}

impl SignalForceExit {
    pub const FORCED_EXIT_SIGNAL: &'static str = "SignalForcedExit";

    pub fn new<E, I>(exchange: E, instrument: I) -> Self
    where
        E: Into<Exchange>,
        I: Into<Instrument>,
    {
        Self {
            datetime: Utc::now(),
            exchange: exchange.into(),
            instrument: instrument.into(),
        }
    }
}

impl<M> From<M> for SignalForceExit
where
    M: Into<Market>,
{
    fn from(market: M) -> Self {
        let market = market.into();
        Self::new(market.exchange, market.instrument)
    }
}
