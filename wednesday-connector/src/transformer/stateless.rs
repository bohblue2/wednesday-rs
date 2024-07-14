use std::marker::PhantomData;
use std::fmt::Debug;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::debug;
use wednesday_model::{error::DataError, events::{MarketEvent}, identifiers::{ExchangeId, Identifier, SubscriptionId}, instruments::Instrument, orderbook::OrderBook};

use crate::{exchange::connector::Connector, protocol::http::websocket::WsMessage, subscriber::subscription::{Map, SubscriptionKind}};

use super::{iterator::MarketIter, updater::{InstrumentOrderBook, OrderBookUpdater}, ExchangeTransformer, Transformer};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StatelessTransformer<Exchange, Kind, Input> {
    instrument_map: Map<Instrument>,
    phantom: PhantomData<(Exchange, Kind, Input)>,
}

#[async_trait]
impl<Exchange, Kind, Input> ExchangeTransformer<Exchange, Kind>
    for StatelessTransformer<Exchange, Kind, Input>
where
    Exchange: Connector + Send,
    Kind: SubscriptionKind + Send,
    Input: Identifier<Option<SubscriptionId>> + for<'de> Deserialize<'de>,
    MarketIter<Kind::Event>: From<(ExchangeId, Instrument, Input)>,
{
    async fn new(
        _: mpsc::UnboundedSender<WsMessage>,
        instrument_map: Map<Instrument>,
    ) -> Result<Self, DataError> {
        debug!(?instrument_map, "Creating StatelessTransformer, no WebSocket sink required");

        Ok(Self {
            instrument_map,
            phantom: PhantomData::default(),
        })
    }
}


impl<Exchange, Kind, Input> Transformer for StatelessTransformer<Exchange, Kind, Input>
where
    Exchange: Connector,
    Kind: SubscriptionKind,
    Input: Identifier<Option<SubscriptionId>> + for<'de> Deserialize<'de>,
    MarketIter<Kind::Event>: From<(ExchangeId, Instrument, Input)>,
{
    type Error = DataError;
    type Input = Input;
    type Output = MarketEvent<Kind::Event>;
    type OutputIter = Vec<Result<Self::Output, Self::Error>>;
    type Pong = ();

    fn transform(&mut self, input: Self::Input) -> Self::OutputIter {
        let subscription_id = match input.id() {
            Some(subscription_id) => subscription_id,
            None => return vec![],
        };

        match self.instrument_map.find(&subscription_id) {
            Ok(instrument) => MarketIter::<Kind::Event>::from((Exchange::ID, instrument, input)).0,
            Err(unidentifiable) => vec![Err(DataError::Socket(unidentifiable))],
        }
    }
}

pub struct StatelessTransformerWithPong<Exchange, Kind, Input, Pong> {
    inner: StatelessTransformer<Exchange, Kind, Input>,
    phantom: PhantomData<Pong>,
}

impl<Exchange, Kind, Input, Pong> Transformer for 
    StatelessTransformerWithPong<Exchange, Kind, Input, Pong>
where
    Exchange: Connector,
    Kind: SubscriptionKind,
    Input: Identifier<Option<SubscriptionId>> + for<'de> Deserialize<'de>,
    MarketIter<Kind::Event>: From<(ExchangeId, Instrument, Input)>,
    Pong: for<'de> Deserialize<'de> + Debug,
{
    type Error = DataError;
    type Input = Input;
    type Output = MarketEvent<Kind::Event>;
    type OutputIter = Vec<Result<Self::Output, Self::Error>>;
    type Pong = Pong;

    fn transform(&mut self, input: Self::Input) -> Self::OutputIter {
        self.inner.transform(input)
    }
}

#[async_trait]
impl<Exchange, Kind, Input, Pong> ExchangeTransformer<Exchange, Kind> for 
    StatelessTransformerWithPong<Exchange, Kind, Input, Pong> 
where
    Exchange: Connector + Send,
    Kind: SubscriptionKind + Send,
    Input: Identifier<Option<SubscriptionId>> + for<'de> Deserialize<'de>,
    MarketIter<Kind::Event>: From<(ExchangeId, Instrument, Input)>,
    Pong: for<'de> Deserialize<'de> + Send + Debug,
{
    async fn new(
        ws_sink_tx: mpsc::UnboundedSender<WsMessage>,
        instrument_map: Map<Instrument>,
    ) -> Result<Self, DataError> {
        debug!(?instrument_map, "Creating StatelessTransformerWithPong, no WebSocket sink required");
        
        Ok(Self {
            inner: StatelessTransformer::new(ws_sink_tx ,instrument_map).await?,
            phantom: PhantomData::default(),
        })
    }
}