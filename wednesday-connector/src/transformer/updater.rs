use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use wednesday_model::{error::DataError, instruments::Instrument, orderbook::OrderBook};

use crate::protocol::http::websocket::WsMessage;

#[async_trait]
pub trait OrderBookUpdater
where
    Self: Sized,
{
    type OrderBook;
    type Update;

    async fn init<Exchange, Kind>(
        ws_sink_tx: mpsc::UnboundedSender<WsMessage>,
        instrument: Instrument,
    ) -> Result<InstrumentOrderBook<Self>, DataError>
    where
        Exchange: Send,
        Kind: Send;

    fn update(&mut self, book: &mut Self::OrderBook, update: Self::Update) -> Result<Option<Self::OrderBook>, DataError>;
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct InstrumentOrderBook<Updater> {
    pub instrument: Instrument,
    pub updater: Updater,
    pub book: OrderBook,
}
