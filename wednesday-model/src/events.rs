use std::fmt::Debug;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{bar::Bar, identifiers::Exchange, instruments::Instrument, orderbook::OrderBookL1, trade::PublicTrade};

// use super::orderbook::{OrderBookL1};
// use super::trade::Trade;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MarketEvent<T> {
    pub exchange_ts: DateTime<Utc>,
    pub local_ts: DateTime<Utc>,
    pub exchange: Exchange,
    pub instrument: Instrument,
    pub kind: T,
}

#[derive(Debug, Clone)]
pub enum DataKind {
    PublicTrade(PublicTrade),
    OrderBookL1(OrderBookL1),
    // OrderBook(Orderbook),
    Bar(Bar),
    // Liquidation(Liquidation)
}

impl From<MarketEvent<PublicTrade>> for MarketEvent<DataKind> {
    fn from(event: MarketEvent<PublicTrade>) -> Self {
        Self {
            exchange_ts: event.exchange_ts,
            local_ts: event.local_ts,
            exchange: event.exchange,
            instrument: event.instrument,
            kind: DataKind::PublicTrade(event.kind),
        }
    }
}

impl From<MarketEvent<OrderBookL1>> for MarketEvent<DataKind> {
    fn from(event: MarketEvent<OrderBookL1>) -> Self {
        Self {
            exchange_ts: event.exchange_ts,
            local_ts: event.local_ts,
            exchange: event.exchange,
            instrument: event.instrument,
            kind: DataKind::OrderBookL1(event.kind),
        }
    }
}

impl From<MarketEvent<Bar>> for MarketEvent<DataKind> {
    fn from(event: MarketEvent<Bar>) -> Self {
        Self {
            exchange_ts: event.exchange_ts,
            local_ts: event.local_ts,
            exchange: event.exchange,
            instrument: event.instrument,
            kind: DataKind::Bar(event.kind),
        }
    }
}

// / Events that occur when bartering. [`MarketEvent`], [`Signal`], [`OrderEvent`], and
// / [`FillEvent`] are vital to the [`Trader`](crate::engine::trader::Trader) event loop, dictating
// / the trading sequence. The [`PositionExit`] Event is a representation of work done by the
// / system, and is useful for analysing performance & reconciliations.
// #[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
// pub enum Event {
//     Market(MarketEvent<DataKind>),
//     Signal(Signal),
//     SignalForceExit(SignalForceExit),
//     OrderNew(OrderEvent),
//     OrderUpdate,
//     Fill(FillEvent),
//     PositionNew(Position),
//     PositionUpdate(PositionUpdate),
//     PositionExit(PositionExit),
//     Balance(Balance),
// }
