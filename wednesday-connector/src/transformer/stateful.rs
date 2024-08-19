use std::marker::PhantomData;
use std::fmt::Debug;

use async_trait::async_trait;
use serde::Deserialize;
use tokio::sync::mpsc;
use wednesday_model::{
    error::DataError,
    events::MarketEvent,
    identifiers::{Identifier, SubscriptionId},
    instruments::Instrument,
    orderbook::OrderBook,
};

use crate::{
    exchange::connector::Connector,
    protocol::http::websocket::WsMessage,
    subscriber::subscription::{Map, SubscriptionKind},
};

use super::{
    iterator::MarketIter,
    updater::{InstrumentOrderBook, OrderBookUpdater},
    ExchangeTransformer, Transformer,
};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct MultiBookTransformer<Exchange, Kind, Updater> {
    // Map of instrument order books, NOTE: Shouldn't we change the map variable name?
    pub book_map: Map<InstrumentOrderBook<Updater>>,
    phantom: PhantomData<(Exchange, Kind)>,
}

impl<Exchange, Kind, Updater> Transformer for MultiBookTransformer<Exchange, Kind, Updater>
where
    Exchange: Connector,
    Kind: SubscriptionKind<Event = OrderBook>,
    Updater: OrderBookUpdater<OrderBook = Kind::Event>,
    Updater::Update: Identifier<Option<SubscriptionId>> + for<'de> Deserialize<'de> + Debug,
{
    type Error = DataError;
    type Input = Updater::Update;
    type Output = MarketEvent<Kind::Event>;
    type OutputIter = Vec<Result<Self::Output, Self::Error>>;
    type Pong = ();

    fn transform(&mut self, update: Self::Input) -> Self::OutputIter {
        let subscription_id = match update.id() {
            Some(subscription_id) => subscription_id,
            None => return vec![],
        };

        let book = match self.book_map.find_mut(&subscription_id) {
            Ok(book) => book,
            Err(unidentifiable) => return vec![Err(DataError::Socket(unidentifiable))],
        };

        let InstrumentOrderBook { instrument, book, updater } = book;

        // Apply update (snapshot or delta) to OrderBook & generate Market<OrderBook> snapshot
        match updater.update(book, update) {
            Ok(Some(book)) => MarketIter::<OrderBook>::from((Exchange::ID, instrument.clone(), book)).0,
            // NOTE: Shouldn't we return an error here?
            Ok(None) => vec![],
            Err(error) => vec![Err(error)],
        }
    }
}

#[async_trait]
impl<Exchange, Kind, Updater> ExchangeTransformer<Exchange, Kind> for MultiBookTransformer<Exchange, Kind, Updater>
where
    Exchange: Connector + Send,
    Kind: SubscriptionKind<Event = OrderBook> + Send,
    Updater: OrderBookUpdater<OrderBook = Kind::Event> + Send,
    Updater::Update: Identifier<Option<SubscriptionId>> + for<'de> Deserialize<'de> + Debug,
{
    async fn new(ws_sink_tx: mpsc::UnboundedSender<WsMessage>, map: Map<Instrument>) -> Result<Self, DataError> {
        let (subscription_ids, init_book_requests): (Vec<_>, Vec<_>) = map
            .0
            .into_iter()
            .map(|(subscription_id, instrument)| (subscription_id, Updater::init::<Exchange, Kind>(ws_sink_tx.clone(), instrument)))
            .unzip();

        let init_order_books = futures::future::join_all(init_book_requests)
            .await
            .into_iter()
            .collect::<Result<Vec<InstrumentOrderBook<Updater>>, DataError>>()?;

        let book_map = subscription_ids
            .into_iter()
            .zip(init_order_books.into_iter())
            .collect::<Map<InstrumentOrderBook<Updater>>>();

        Ok(Self {
            book_map,
            phantom: PhantomData::default(),
        })
    }
}
