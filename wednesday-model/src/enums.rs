

use std::fmt::{self, Display};

use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum AggressorSide {
    #[serde(rename = "buy", alias = "BUY", alias = "Buy", alias = "b", alias = "B")]
    Buy,
    #[serde(rename = "sell", alias = "SELL", alias = "Sell", alias = "s", alias = "S")]
    Sell,
    None,
}

impl Display for AggressorSide {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                AggressorSide::Buy => "Buy",
                AggressorSide::Sell => "Sell",
                AggressorSide::None => "None",
            }
        )
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub enum OrderSide {
    Buy,
    Sell,
    None,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub enum OrderType {
    Limit,
    Market,
    Cancel,
    None,
}

pub const PRICE: i32 = 0;
pub const QUANTITY: i32 = 1;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Deserialize, Serialize)]
pub enum BookSide {
    #[serde(alias = "buy", alias = "BUY", alias = "b", alias = "bid")]
    Bid,
    #[serde(alias = "sell", alias = "SELL", alias = "s", alias = "ask")]
    Ask,
}

impl Display for BookSide {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                BookSide::Bid => "Bid",
                BookSide::Ask => "Ask",
            }
        )
    }
}
