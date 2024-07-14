use chrono::{DateTime, Utc};
use wednesday_model::{identifiers::Exchange, instruments::Instrument};

use super::{decision::Decision, execution_error::ExecutionError, fee::Fees, market_meta::MarketMeta};

#[derive(Debug, Clone)]
pub struct FillEvent {
    pub timestamp: DateTime<Utc>,
    pub exchange: Exchange,
    pub instrument: Instrument,
    pub market_meta: MarketMeta,
    pub decision: Decision,
    pub quantity: f64,
    pub fill_value_gross: f64,
    pub fees: Fees,
}

impl FillEvent {
    pub const EVENT_TYPE: &'static str = "FillEvent";

    pub fn builder() -> FillEventBuilder {
        FillEventBuilder::new()
    }
}

#[derive(Debug, Default)]
pub struct FillEventBuilder {
    pub time: Option<DateTime<Utc>>,
    pub exchange: Option<Exchange>,
    pub instrument: Option<Instrument>,
    pub market_meta: Option<MarketMeta>,
    pub decision: Option<Decision>,
    pub quantity: Option<f64>,
    pub fill_value_gross: Option<f64>,
    pub fees: Option<Fees>,
}

impl FillEventBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn time(self, value: DateTime<Utc>) -> Self {
        Self { time: Some(value), ..self }
    }
    pub fn exchange(self, value: Exchange) -> Self {
        Self {
            exchange: Some(value),
            ..self
        }
    }
    pub fn instrument(self, value: Instrument) -> Self {
        Self {
            instrument: Some(value),
            ..self
        }
    }
    pub fn market_meta(self, value: MarketMeta) -> Self {
        Self {
            market_meta: Some(value),
            ..self
        }
    }
    pub fn decision(self, value: Decision) -> Self {
        Self {
            decision: Some(value),
            ..self
        }
    }
    pub fn quantity(self, value: f64) -> Self {
        Self {
            quantity: Some(value),
            ..self
        }
    }
    pub fn fill_value_gross(self, value: f64) -> Self {
        Self {
            fill_value_gross: Some(value),
            ..self
        }
    }
    pub fn fees(self, value: Fees) -> Self {
        Self { fees: Some(value), ..self }
    }

    pub fn build(self) -> Result<FillEvent, ExecutionError> {
        let timestamp = self.time.ok_or(ExecutionError::BuilderIncomplete("time"))?;
        let exchange = self.exchange.ok_or(ExecutionError::BuilderIncomplete("exchange"))?;
        let instrument = self.instrument.ok_or(ExecutionError::BuilderIncomplete("instrument"))?;
        let market_meta = self.market_meta.ok_or(ExecutionError::BuilderIncomplete("market_meta"))?;
        let decision = self.decision.ok_or(ExecutionError::BuilderIncomplete("decision"))?;
        let quantity = self.quantity.ok_or(ExecutionError::BuilderIncomplete("quantity"))?;
        let fill_value_gross = self.fill_value_gross.ok_or(ExecutionError::BuilderIncomplete("fill_value_gross"))?;
        let fees = self.fees.ok_or(ExecutionError::BuilderIncomplete("fees"))?;

        Ok(FillEvent {
            timestamp,
            exchange,
            instrument,
            market_meta,
            decision,
            quantity,
            fill_value_gross,
            fees,
        })
    }
}
