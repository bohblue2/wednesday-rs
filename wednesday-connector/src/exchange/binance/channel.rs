
use wednesday_model::identifiers::Identifier;

use crate::subscriber::subscription::{kind::{OrderBooksL2, PublicTrades}, Subscription};

use super::Binance;

pub struct BinanceChannel(pub &'static str);

impl BinanceChannel {
    pub const TRADES: Self = Self("@trade");
    pub const ORDER_BOOK_L2: Self = Self("@depth@100ms");
    pub const LIQUIDATIONS: Self = Self("@forceOrder");
}

impl<Server> Identifier<BinanceChannel> for Subscription<Binance<Server>, PublicTrades> {
    fn id(&self) -> BinanceChannel {
        BinanceChannel::TRADES
    }
}

impl<Server> Identifier<BinanceChannel> for Subscription<Binance<Server>, OrderBooksL2> {
    fn id(&self) -> BinanceChannel {
        BinanceChannel::ORDER_BOOK_L2
    }
}

// impl<Server> Identifier<BinanceChannel> for Subscription<Binance<Server>, Liquidations> {
//     fn id(&self) -> BinanceChannel {
//         BinanceChannel::LIQUIDATIONS
//     }
// }



impl AsRef<str> for BinanceChannel {
    fn as_ref(&self) -> &str {
        self.0
    }
}