use std::{marker::PhantomData, time::Duration, vec};

use std::fmt::Debug;
use tokio::time;
use url::Url;
use wednesday_model::{error::SocketError, identifiers::ExchangeId, instruments::Instrument};

use crate::{
    protocol::http::websocket::{PingInterval, WsMessage},
    stream::{protocol::websocket::ExchangeWsStream, selector::StreamSelector},
    subscriber::{
        protocol::websocket::WsSubscriber,
        subscription::{
            kind::{OrderBooksL2, PublicTrades},
            ExchangeSubscription, Map,
        },
        validator::WsSubscriptionValidator,
    },
    transformer::{stateful::MultiBookTransformer, stateless::StatelessTransformerWithPong},
};

use self::{
    channel::BybitChannel,
    market::BybitMarket,
    model::{l2::BybitBookUpdater, trade::BybitTrade},
    subscription::BybitSubscriptionResponse,
};

use super::connector::{self, Connector, ExchangeServer};

pub mod channel;
pub mod linear;
pub mod market;
pub mod model;
pub mod spot;
pub mod subscription;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Bybit<Server> {
    server: PhantomData<Server>,
}

impl<Server> Connector for Bybit<Server>
where
    Server: ExchangeServer,
{
    const ID: ExchangeId = Server::ID;
    type Channel = BybitChannel;
    type Market = BybitMarket;
    type Subscriber = WsSubscriber;
    type SubscriptionValidator = WsSubscriptionValidator;
    type SubscriptionResponse = BybitSubscriptionResponse;

    fn url() -> Result<Url, SocketError> {
        Url::parse(Server::ws_url()).map_err(SocketError::UrlParse)
    }

    fn requests(exchange_subscriptions: Vec<ExchangeSubscription<Self::Channel, Self::Market>>) -> Vec<WsMessage> {
        let stream_names = exchange_subscriptions
            .into_iter()
            .map(|sub| format!("{}.{}", sub.channel.as_ref(), sub.market.as_ref(),))
            .collect::<Vec<String>>();

        vec![WsMessage::Text(
            serde_json::json!({
                "op": "subscribe",
                "args": stream_names
            })
            .to_string(),
        )]
    }

    fn ping_interval() -> Option<PingInterval> {
        Some(PingInterval {
            interval: time::interval(Duration::from_millis(5_000)),
            ping: || {
                WsMessage::Text(
                    serde_json::json!({
                        "op": "ping"
                    })
                    .to_string(),
                )
            },
        })
    }

    fn expected_responses(_map: &Map<Instrument>) -> usize {
        1
    }

    fn subscription_timeout() -> std::time::Duration {
        connector::DEFAULT_SUBSCRIPTION_TIMEOUT
    }
}

impl<Server> StreamSelector<PublicTrades> for Bybit<Server>
where
    Server: ExchangeServer + Debug + Send + Sync,
{
    type Stream = ExchangeWsStream<StatelessTransformerWithPong<Self, PublicTrades, BybitTrade, BybitSubscriptionResponse>>;
}

impl<Server> StreamSelector<OrderBooksL2> for Bybit<Server>
where
    Server: ExchangeServer + Debug + Send + Sync,
{
    type Stream = ExchangeWsStream<MultiBookTransformer<Self, OrderBooksL2, BybitBookUpdater>>;
}

impl<'de, Server> serde::Deserialize<'de> for Bybit<Server>
where
    Server: ExchangeServer,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let input = <&str as serde::Deserialize>::deserialize(deserializer)?;
        let expected = Self::ID.as_str();

        if input == Self::ID.as_str() {
            Ok(Self::default())
        } else {
            Err(serde::de::Error::invalid_value(serde::de::Unexpected::Str(input), &expected))
        }
    }
}

impl<Server> serde::Serialize for Bybit<Server>
where
    Server: ExchangeServer,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let exchange_id = Self::ID.as_str();
        serializer.serialize_str(exchange_id)
    }
}
