use chrono::Utc;
use wednesday_model::{
    error::DataError,
    events::MarketEvent,
    identifiers::{Exchange, ExchangeId},
    instruments::Instrument,
    orderbook::OrderBook,
};

#[derive(Debug)]
pub struct MarketIter<T>(pub Vec<Result<MarketEvent<T>, DataError>>);

impl<T> FromIterator<Result<MarketEvent<T>, DataError>> for MarketIter<T> {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = Result<MarketEvent<T>, DataError>>,
    {
        Self(iter.into_iter().collect())
    }
}

impl From<(ExchangeId, Instrument, OrderBook)> for MarketIter<OrderBook> {
    fn from((exchange_id, instrument, order_book): (ExchangeId, Instrument, OrderBook)) -> Self {
        Self(vec![Ok(MarketEvent {
            exchange_ts: order_book.last_update_ts,
            local_ts: Utc::now(),
            exchange: Exchange::from(exchange_id),
            instrument,
            kind: order_book,
        })])
    }
}
