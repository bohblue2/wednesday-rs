use async_trait::async_trait;
use futures::SinkExt;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};
use wednesday_model::{error::SocketError, identifiers::Identifier, instruments::Instrument};

use crate::{exchange::connector::Connector, protocol::http::websocket::{connect, WsClient}, subscriber::{mapper::WsSubscriptionMapper, subscription::{Map, Subscription, SubscriptionKind, SubscriptionMeta}, Subscriber}};
use crate::subscriber::validator::SubscriptionValidator;
use crate::subscriber::mapper::SubscriptionMapper;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub struct WsSubscriber;

#[async_trait]
impl Subscriber for WsSubscriber {
    type SubscriptionMapper = WsSubscriptionMapper;
    
    async fn subscribe<Exchange, Kind>(
        subscriptions: &[Subscription<Exchange, Kind>],
    ) -> Result<(WsClient, Map<Instrument>), SocketError>
    where
        Exchange: Connector + Send + Sync,
        Kind: SubscriptionKind + Send + Sync,
        Subscription<Exchange, Kind>: Identifier<Exchange::Channel> + Identifier<Exchange::Market>,
    {
        let exchange = Exchange::ID;
        let url = Exchange::url()?.to_string();
        debug!(%exchange, %url, ?subscriptions, "subscribing to WebSocket");

        let mut websocket = connect(url).await?;
        debug!(%exchange, ?subscriptions, "WebSocket connection established");

        let SubscriptionMeta {
            instrument_map,
            subscriptions,
        } = Self::SubscriptionMapper::map::<Exchange, Kind>(subscriptions);

        for subscription in subscriptions {
            debug!(%exchange, payload = ?subscription, "sending exchange to subscription");
            websocket.send(subscription).await?;
        }
        debug!(%exchange, "sent subscriptions to WebSocket");
        debug!(%exchange, "validating subscriptions");
        let map =
            Exchange::SubscriptionValidator::validate::<Exchange, Kind>(instrument_map, &mut websocket)
            .await?;
        
        info!(%exchange, "subscribed to WebSocket");
        Ok((websocket, map))
    }
}

