pub mod builder;
pub mod exchange;
pub mod market;
pub mod parser;
pub mod protocol;
pub mod selector;

use std::collections::HashMap;

use tokio::sync::mpsc;
use tokio_stream::{wrappers::UnboundedReceiverStream, StreamMap};
use wednesday_model::identifiers::ExchangeId;

use crate::subscriber::subscription::SubscriptionKind;

use self::builder::{multiple::MultiStreamBuilder, StreamBuilder};

#[derive(Debug)]
pub struct Streams<T> {
    pub streams: HashMap<ExchangeId, mpsc::UnboundedReceiver<T>>,
}

impl<T> Streams<T> {
    pub fn builder<Kind>() -> StreamBuilder<Kind>
    where
        Kind: SubscriptionKind,
    {
        StreamBuilder::<Kind>::new()
    }

    // NOTE: This is a temporary solution to the problem of creating multiple streams
    pub fn builder_multi() -> MultiStreamBuilder<T> {
        MultiStreamBuilder::<T>::new()
    }

    pub fn select(&mut self, exchange: ExchangeId) -> Option<mpsc::UnboundedReceiver<T>> {
        self.streams.remove(&exchange)
    }

    pub async fn join(self) -> mpsc::UnboundedReceiver<T>
    where
        T: Send + 'static,
    {
        let (joined_tx, joined_rx) = mpsc::unbounded_channel();

        for mut exchange_rx in self.streams.into_values() {
            let joined_tx = joined_tx.clone();
            tokio::spawn(async move {
                while let Some(event) = exchange_rx.recv().await {
                    let _ = joined_tx.send(event);
                }
            });
        }
        joined_rx
    }

    pub async fn join_map(self) -> StreamMap<ExchangeId, UnboundedReceiverStream<T>> {
        self.streams.into_iter().fold(StreamMap::new(), |mut map, (exchange, rx)| {
            map.insert(exchange, UnboundedReceiverStream::new(rx));
            map
        })
    }
}
