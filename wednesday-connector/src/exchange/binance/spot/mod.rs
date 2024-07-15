use wednesday_model::identifiers::ExchangeId;

use crate::{
    exchange::connector::ExchangeServer,
    stream::{protocol::ws_stream::ExchangeWsStream, selector::StreamSelector},
    subscriber::subscription::kind::OrderBooksL2,
    transformer::stateful::MultiBookTransformer,
};

use self::l2::BinanceSpotBookUpdater;

use super::Binance;

pub mod l2;
pub mod trade;

pub const WEBSOCKET_BASE_URL_BINANCE_SPOT: &str = "wss://stream.binance.com:9443/ws";

pub type BinanceSpot = Binance<BinanceServerSpot>;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct BinanceServerSpot;

// 여기에 order execution 관련 코드 추가
impl ExchangeServer for BinanceServerSpot {
    const ID: ExchangeId = ExchangeId::BinanceSpot;

    fn ws_url() -> &'static str {
        WEBSOCKET_BASE_URL_BINANCE_SPOT
    }
}

impl StreamSelector<OrderBooksL2> for BinanceSpot {
    type Stream = ExchangeWsStream<MultiBookTransformer<Self, OrderBooksL2, BinanceSpotBookUpdater>>;
}
