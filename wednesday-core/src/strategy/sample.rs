use std::collections::HashMap;

use chrono::Utc;
use ta::{indicators::RelativeStrengthIndex, Next};
use wednesday_model::events::{DataKind, MarketEvent};

use crate::model::{decision::Decision, market_meta::MarketMeta, signal::{Signal, SignalStrength}};

use super::SignalGenerator;

pub struct StrategyConfig {
    pub rsi_period: usize,
}

pub struct RsiStrategy {
    rsi: RelativeStrengthIndex,
}

impl SignalGenerator for RsiStrategy {
    fn generate_signal(&mut self, market: &MarketEvent<DataKind>) -> Option<Signal> {
        let bar_close = match &market.kind {
            DataKind::Bar(candle) => candle.close,
            _ => return None 
        };
        let rsi = self.rsi.next(bar_close);
        let signals = self.generate_signals_map(rsi);

        if signals.is_empty() { return None }

        Some(Signal {
            datetime: Utc::now(),
            exchange: market.exchange.clone(),
            instrument: market.instrument.clone(),
            market_meta: MarketMeta {
                close: bar_close,
                timestamp: market.exchange_ts
            },
            signals: signals,
        })
    }
}

impl RsiStrategy {
    pub fn new(config: StrategyConfig) -> Self {
        let rsi = RelativeStrengthIndex::new(config.rsi_period).unwrap();
        Self {
            rsi: rsi
        }
    }

    pub fn generate_signals_map(&self, rsi: f64) -> HashMap<Decision, SignalStrength> {
        let mut signals = HashMap::new();

        if rsi < 40.0 {
            signals.insert(Decision::Long, self.calculate_signal_strength());
        }
        if rsi > 60.0 {
            signals.insert(Decision::Short, self.calculate_signal_strength());
        }
        signals
    }

    fn calculate_signal_strength(&self) -> SignalStrength {
        SignalStrength(1.0)
    }
}