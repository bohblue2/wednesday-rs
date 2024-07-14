pub mod multiple;

use std::{collections::HashMap, pin::Pin};
use std::fmt::Debug;

use futures::Future;
use tracing::debug;
use wednesday_model::error::DataError;
use wednesday_model::events::MarketEvent;
use wednesday_model::identifiers::{ExchangeId, Identifier};

use crate::exchange::channel::ExchangeChannel;
use crate::stream::market::consume;
use crate::subscriber::subscription::{Subscription, SubscriptionKind};
use crate::subscriber::validator::validate;

use super::selector::StreamSelector;
use super::Streams;

pub type SubscribeFuture = Pin<Box<dyn Future<Output = Result<(), DataError>>>>;

#[derive(Default)]
pub struct StreamBuilder<Kind>
where
    Kind: SubscriptionKind,
{
    pub channels: HashMap<ExchangeId, ExchangeChannel<MarketEvent<Kind::Event>>>,
    pub futures: Vec<SubscribeFuture>,
}

impl<Kind> Debug for StreamBuilder<Kind>
where
    Kind: SubscriptionKind
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StreamBuilder<SubscriptionKind>")
            .field("channels", &self.channels)
            .field("num_futures", &self.futures.len())
            .finish()
    }
}

impl<Kind> StreamBuilder<Kind>
where
    Kind: SubscriptionKind,
{
    pub fn new() -> Self {
        Self {
            channels: HashMap::new(),
            futures: Vec::new(),
        }
    }

    // Note: This part is definitely needed a refactoring.
    pub fn subscribe<
        SubscriptionIter, 
        SubscriptionItem, 
        Exchange>
    (mut self, subscriptions: SubscriptionIter) -> Self
    where
        SubscriptionIter: IntoIterator<Item = SubscriptionItem>,
        SubscriptionItem: Into<Subscription<Exchange, Kind>>,
        Exchange: StreamSelector<Kind> + Ord + Send + Sync + 'static,
        Kind: Ord + Send + Sync + 'static,
        Kind::Event: Send,
        Subscription<Exchange, Kind>: Identifier<Exchange::Channel> + Identifier<Exchange::Market>,

    {
        let mut subscriptions = 
            subscriptions.into_iter()
                .map(|subscription| subscription.into())
                .collect::<Vec<Subscription<Exchange, Kind>>>();

        let exchange_tx = self.channels
            .entry(Exchange::ID) 
            .or_default()
            .tx
            .clone();
        
        self.futures.push(
            Box::pin(async move {
                debug!("Validating subscriptions before subscribing.");
                validate(&subscriptions)?;
                
                subscriptions.sort();
                subscriptions.dedup();

                debug!("Spawning task to consume subscriptions for exchange: {:?}", Exchange::ID);
                tokio::spawn(consume::<Exchange, Kind>(subscriptions, exchange_tx));

                Ok(())
            }));
        
        self
    }

    pub async fn init(self) -> Result<Streams<MarketEvent<Kind::Event>>, DataError> {
        // Await Stream initialization perpetual and ensure success
        futures::future::join_all(self.futures).await;
        
        Ok(Streams {
            streams: self
                .channels
                .into_iter()    
                .map(|(exchange_id, channel)| (exchange_id, channel.rx))
                .collect(),
        })
    }
}

