pub mod iterator;
pub mod stateful;
pub mod stateless;
pub mod updater;

use async_trait::async_trait;
use serde::Deserialize;
use tokio::sync::mpsc;
use wednesday_model::{error::DataError, events::MarketEvent, instruments::Instrument};

use crate::{
    protocol::http::websocket::WsMessage,
    subscriber::subscription::{Map, SubscriptionKind},
};

pub trait Transformer {
    type Error;
    type Input: for<'de> Deserialize<'de>;
    type Output;
    type OutputIter: IntoIterator<Item = Result<Self::Output, Self::Error>>;
    type Pong: for<'de> Deserialize<'de>;
    fn transform(&mut self, input: Self::Input) -> Self::OutputIter;
}

#[async_trait]
pub trait ExchangeTransformer<Exchange, Kind>
where
    Self: Transformer<Output = MarketEvent<Kind::Event>, Error = DataError> + Sized,
    Kind: SubscriptionKind,
{
    async fn new(ws_sink_tx: mpsc::UnboundedSender<WsMessage>, instrument_map: Map<Instrument>) -> Result<Self, DataError>;
}
