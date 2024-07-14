use std::collections::HashMap;

use chrono::Utc;
use ta::{indicators::RelativeStrengthIndex, Next};
use tracing::debug;
use wednesday_model::events::{DataKind, MarketEvent};

use crate::model::{decision::Decision, market_meta::MarketMeta, signal::{Signal, SignalStrength}};

use super::SignalGenerator;

pub struct TickReactStrategyConfig {
    pub rsi_period: usize,
}

pub struct TickReactStrategy {
    rsi: RelativeStrengthIndex,
}

impl SignalGenerator for TickReactStrategy {
    fn generate_signal(&mut self, market: &MarketEvent<DataKind>) -> Option<Signal> {
        let last_trade_price = match &market.kind {
            DataKind::Bar(candle) => return None,
            DataKind::PublicTrade(trade) => { debug!("PublicTrade: {:?}", trade.price); trade.price }
            _ => return None
        };

        let signals = self.generate_signals_map(last_trade_price);

        Some(Signal {
            datetime: Utc::now(),
            exchange: market.exchange.clone(),
            instrument: market.instrument.clone(),
            market_meta: MarketMeta {
                close: last_trade_price,
                timestamp: market.exchange_ts
            },
            signals: signals,
        })
    }
}

impl TickReactStrategy {
    pub fn new(config: TickReactStrategyConfig) -> Self {
        let rsi = RelativeStrengthIndex::new(config.rsi_period).unwrap();
        Self {
            rsi: rsi
        }
    }

    pub fn generate_signals_map(&self, rsi: f64) -> HashMap<Decision, SignalStrength> {
        let mut signals = HashMap::new();

        if rsi < 57375.0 {
            signals.insert(Decision::Long, self.calculate_signal_strength());
        }
        if rsi > 57376.0 {
            signals.insert(Decision::Short, self.calculate_signal_strength());
        }
        signals
    }

    fn calculate_signal_strength(&self) -> SignalStrength {
        SignalStrength(1.0)
    }
}