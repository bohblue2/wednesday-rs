use wednesday_model::identifiers::ExchangeId;

use crate::{
    exchange::connector::ExchangeServer,
    stream::{protocol::websocket::ExchangeWsStream, selector::StreamSelector},
    subscriber::subscription::kind::OrderBooksL2,
    transformer::stateful::MultiBookTransformer,
};

use self::l2::BinanceFuturesBookUpdater;

use super::Binance;

pub mod l2;
pub mod trade;

pub const WEBSOCKET_BASE_URL_BINANCE_FUTURES_USD: &str = "wss://fstream.binance.com/ws";

pub type BinanceFuturesUsd = Binance<BinanceServerFuturesUsd>;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct BinanceServerFuturesUsd;

impl ExchangeServer for BinanceServerFuturesUsd {
    const ID: ExchangeId = ExchangeId::BinanceFuturesUsd;

    fn ws_url() -> &'static str {
        WEBSOCKET_BASE_URL_BINANCE_FUTURES_USD
    }
}

impl StreamSelector<OrderBooksL2> for BinanceFuturesUsd {
    type Stream = ExchangeWsStream<MultiBookTransformer<Self, OrderBooksL2, BinanceFuturesBookUpdater>>;
}
