use wednesday_macro::{DeSubscriptionKind, SerSubscriptionKind};
use wednesday_model::{bar::Bar, orderbook::{OrderBook, OrderBookL1}, trade::PublicTrade};

use super::SubscriptionKind;

#[derive(Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Debug, Hash, DeSubscriptionKind, SerSubscriptionKind)]
pub struct OrderBooksL1;

impl SubscriptionKind for OrderBooksL1 {
    type Event = OrderBookL1;
}
#[derive(Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Debug, Hash, DeSubscriptionKind, SerSubscriptionKind)]
pub struct OrderBooksL2;
impl SubscriptionKind for OrderBooksL2 {
    type Event = OrderBook;
}

#[derive(Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Debug, Hash, DeSubscriptionKind, SerSubscriptionKind)]
pub struct OrderBooksL3;
impl SubscriptionKind for OrderBooksL3 {
    type Event = OrderBook;
}

#[derive(Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Debug, Hash, SerSubscriptionKind, DeSubscriptionKind)]
pub struct PublicTrades;

impl SubscriptionKind for PublicTrades {
    type Event = PublicTrade;
}


#[derive(Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Debug, Hash, SerSubscriptionKind, DeSubscriptionKind)]
pub struct Bars;

impl SubscriptionKind for Bars {
    type Event = Bar;
}