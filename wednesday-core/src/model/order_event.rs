use chrono::{DateTime, Utc};
use wednesday_model::{enums::OrderType, identifiers::Exchange, instruments::Instrument};

use super::{decision::Decision, market_meta::MarketMeta, portfolio_error::PortfolioError};

#[derive(Debug, Clone)]
pub struct OrderEvent {
    pub timestamp: DateTime<Utc>,
    pub exchange: Exchange,
    pub instrument: Instrument,
    pub market_meta: MarketMeta,
    pub decision: Decision,
    pub quantity: f64,
    pub order_type: OrderType,
}

impl OrderEvent {
    pub const ORGANIC_ORDER: &'static str = "Order";
    pub const FORCED_EXIT_ORDER: &'static str = "OrderForcedExit";

    pub fn builder() -> OrderEventBuilder {
        OrderEventBuilder::new()
    }
}

/// Builder to construct OrderEvent instances.
#[derive(Debug, Default)]
pub struct OrderEventBuilder {
    pub time: Option<DateTime<Utc>>,
    pub exchange: Option<Exchange>,
    pub instrument: Option<Instrument>,
    pub market_meta: Option<MarketMeta>,
    pub decision: Option<Decision>,
    pub quantity: Option<f64>,
    pub order_type: Option<OrderType>,
}

impl OrderEventBuilder {
    pub fn new() -> Self { Self::default() }
    pub fn time(self, value: DateTime<Utc>) -> Self { Self { time: Some(value), ..self } }
    pub fn exchange(self, value: Exchange) -> Self { Self { exchange: Some(value), ..self } }
    pub fn instrument(self, value: Instrument) -> Self { Self { instrument: Some(value), ..self } }
    pub fn market_meta(self, value: MarketMeta) -> Self { Self { market_meta: Some(value), ..self } }
    pub fn decision(self, value: Decision) -> Self { Self { decision: Some(value), ..self } }
    pub fn quantity(self, value: f64) -> Self { Self { quantity: Some(value), ..self } }
    pub fn order_type(self, value: OrderType) -> Self { Self { order_type: Some(value), ..self } }

    pub fn build(self) -> Result<OrderEvent, PortfolioError> {
        Ok(OrderEvent {
            timestamp: self.time.ok_or(PortfolioError::BuilderIncomplete("time"))?,
            exchange: self
                .exchange
                .ok_or(PortfolioError::BuilderIncomplete("exchange"))?,
            instrument: self
                .instrument
                .ok_or(PortfolioError::BuilderIncomplete("instrument"))?,
            market_meta: self
                .market_meta
                .ok_or(PortfolioError::BuilderIncomplete("market_meta"))?,
            decision: self
                .decision
                .ok_or(PortfolioError::BuilderIncomplete("decision"))?,
            quantity: self
                .quantity
                .ok_or(PortfolioError::BuilderIncomplete("quantity"))?,
            order_type: self
                .order_type
                .ok_or(PortfolioError::BuilderIncomplete("order_type"))?,
        })
    }
}