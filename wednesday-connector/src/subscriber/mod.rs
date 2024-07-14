use async_trait::async_trait;
use wednesday_model::{error::SocketError, identifiers::Identifier, instruments::Instrument};

use crate::{exchange::connector::Connector, protocol::http::websocket::WsClient};

use self::{mapper::SubscriptionMapper, subscription::{Map, Subscription, SubscriptionKind}};

pub mod protocol;
pub mod subscription;
pub mod mapper;
pub mod validator;

#[async_trait]
pub trait Subscriber {
    type SubscriptionMapper: SubscriptionMapper;

    async fn subscribe<Exchange, Kind>(
        subscriptions: &[Subscription<Exchange, Kind>],
    ) -> Result<(WsClient, Map<Instrument>), SocketError>
    where
        Exchange: Connector + Send + Sync,
        Kind: SubscriptionKind + Send + Sync,
        Subscription<Exchange, Kind>: Identifier<Exchange::Channel> + Identifier<Exchange::Market>;
}