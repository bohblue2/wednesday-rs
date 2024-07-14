use crate::{exchange::connector::Connector, subscriber::subscription::SubscriptionKind};

use super::market::MarketStream;

pub trait StreamSelector<Kind> 
where
    Self: Connector,
    Kind: SubscriptionKind,
{
    type Stream: MarketStream<Self, Kind>;
}