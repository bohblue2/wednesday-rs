use wednesday_model::identifiers::Identifier;

use crate::subscriber::subscription::{
    kind::{OrderBooksL2, PublicTrades},
    Subscription,
};

use super::Bybit;

pub struct BybitChannel(pub &'static str);

/// Topic:
/// `orderbook.{depth}.{symbol}`
///
/// Example:
/// `orderbook.1.BTCUSDT`
impl BybitChannel {
    pub const TRADES: Self = Self("publicTrade");
    pub const ORDER_BOOK_L2: Self = Self("orderbook.50");
}

impl<Server> Identifier<BybitChannel> for Subscription<Bybit<Server>, PublicTrades> {
    fn id(&self) -> BybitChannel {
        BybitChannel::TRADES
    }
}

impl<Server> Identifier<BybitChannel> for Subscription<Bybit<Server>, OrderBooksL2> {
    fn id(&self) -> BybitChannel {
        BybitChannel::ORDER_BOOK_L2
    }
}

impl AsRef<str> for BybitChannel {
    fn as_ref(&self) -> &str {
        self.0
    }
}
