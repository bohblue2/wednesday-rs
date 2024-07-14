use serde::{Deserialize, Serialize};
use wednesday_model::identifiers::Identifier;

use crate::subscriber::subscription::Subscription;

use super::Binance;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct BinanceMarket(pub String);


impl<Server, Kind> Identifier<BinanceMarket> for Subscription<Binance<Server>, Kind> {
    fn id(&self) -> BinanceMarket {
        BinanceMarket(format!("{}{}", self.instrument.base_currency, self.instrument.quote_currency).to_uppercase())
    }
}

impl AsRef<str> for BinanceMarket {
    fn as_ref(&self) -> &str {
        &self.0
    }
}