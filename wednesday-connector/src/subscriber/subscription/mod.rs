pub mod kind;
pub mod private;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::{self, Debug, Display};
use wednesday_model::error::SocketError;
use wednesday_model::identifiers::{Identifier, SubscriptionId};
use wednesday_model::instruments::{Instrument, InstrumentKind};

use crate::protocol::http::websocket::WsMessage;

pub trait SubscriptionKind
where
    Self: Debug + Clone,
{
    type Event: Debug;
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Subscription<Exchange, Kind> {
    pub exchange: Exchange,
    #[serde(flatten)]
    pub instrument: Instrument,
    #[serde(alias = "type")]
    pub kind: Kind,
}

impl<Exchange, Kind> Display for Subscription<Exchange, Kind>
where
    Exchange: Display,
    Kind: Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}_{}{}", self.exchange, self.kind, self.instrument)
    }
}

impl<Exchange, S, Kind> From<(Exchange, S, S, InstrumentKind, Kind)> for Subscription<Exchange, Kind>
where
    S: Into<String>,
{
    fn from((exchange, base_currency, quote_currency, instrument_kind, kind): (Exchange, S, S, InstrumentKind, Kind)) -> Self {
        Self::new(exchange, (base_currency, quote_currency, instrument_kind), kind)
    }
}

impl<Exchange, Kind> Subscription<Exchange, Kind> {
    pub fn new<I>(exchange: Exchange, instrument: I, kind: Kind) -> Self
    where
        I: Into<Instrument>,
    {
        Self {
            exchange,
            instrument: instrument.into(),
            kind,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Ord, PartialOrd, Hash, Deserialize)]
pub struct ExchangeSubscription<Channel, Market> {
    pub channel: Channel,
    pub market: Market,
}

impl<Channel, Market> Identifier<SubscriptionId> for ExchangeSubscription<Channel, Market>
where
    Channel: AsRef<str>,
    Market: AsRef<str>,
{
    fn id(&self) -> SubscriptionId {
        SubscriptionId::from(format!("{}|{}", self.channel.as_ref(), self.market.as_ref()))
    }
}

impl<Channel, Market> ExchangeSubscription<Channel, Market>
where
    Channel: AsRef<str>,
    Market: AsRef<str>,
{
    pub fn new<Exchange, Kind>(subscription: &Subscription<Exchange, Kind>) -> Self
    where
        Subscription<Exchange, Kind>: Identifier<Channel> + Identifier<Market>,
    {
        Self {
            channel: subscription.id(),
            market: subscription.id(),
        }
    }
}

impl<Channel, Market> From<(Channel, Market)> for ExchangeSubscription<Channel, Market>
where
    Channel: AsRef<str>,
    Market: AsRef<str>,
{
    fn from((channel, market): (Channel, Market)) -> Self {
        Self { channel, market }
    }
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct SubscriptionMeta {
    pub instrument_map: Map<Instrument>,
    pub subscriptions: Vec<WsMessage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Map<T>(pub HashMap<SubscriptionId, T>);

impl<T> FromIterator<(SubscriptionId, T)> for Map<T> {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = (SubscriptionId, T)>,
    {
        Self(iter.into_iter().collect::<HashMap<SubscriptionId, T>>())
    }
}

impl<T> Map<T> {
    /// Find the `T` associated with the provided [`SubscriptionId`].
    pub fn find(&self, id: &SubscriptionId) -> Result<T, SocketError>
    where
        T: Clone,
    {
        self.0.get(id).cloned().ok_or_else(|| SocketError::Unidentifiable(id.clone()))
    }

    /// Find the mutable reference to `T` associated with the provided [`SubscriptionId`].
    pub fn find_mut(&mut self, id: &SubscriptionId) -> Result<&mut T, SocketError> {
        self.0.get_mut(id).ok_or_else(|| SocketError::Unidentifiable(id.clone()))
    }
}
