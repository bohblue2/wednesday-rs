use core::fmt::Debug;
use std::marker::PhantomData;

use crate::protocol::http::websocket::{PingInterval, WsMessage};
use crate::stream::protocol::websocket::ExchangeWsStream;
use crate::stream::selector::StreamSelector;
use crate::subscriber::protocol::websocket::WsSubscriber;
use crate::subscriber::subscription::kind::PublicTrades;
use crate::subscriber::subscription::{ExchangeSubscription, Map};
use crate::subscriber::validator::WsSubscriptionValidator;
use crate::transformer::stateless::StatelessTransformer;
use url::Url;
use wednesday_model::error::SocketError;
use wednesday_model::identifiers::ExchangeId;
use wednesday_model::instruments::Instrument;

use self::market::BinanceMarket;
use self::{channel::BinanceChannel, spot::trade::BinanceSpotTrade, subscription::BinanceSubscriptionResponse};

use super::connector::{Connector, ExchangeServer};

pub mod book;
pub mod channel;
mod futures;
pub mod market;
pub mod spot;
pub mod subscription;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Binance<Server> {
    server: PhantomData<Server>,
}

impl<Server> Connector for Binance<Server>
where
    Server: ExchangeServer,
{
    const ID: ExchangeId = Server::ID;
    type Channel = BinanceChannel;
    type Market = BinanceMarket;
    type Subscriber = WsSubscriber;
    type SubscriptionValidator = WsSubscriptionValidator;
    type SubscriptionResponse = BinanceSubscriptionResponse;

    fn url() -> Result<Url, SocketError> {
        Url::parse(Server::ws_url()).map_err(|e| SocketError::UrlParse(e))
    }

    fn requests(exchange_subscriptions: Vec<ExchangeSubscription<Self::Channel, Self::Market>>) -> Vec<WsMessage> {
        let stream_names = exchange_subscriptions
            .into_iter()
            .map(|sub| format!("{}{}", sub.market.as_ref().to_lowercase(), sub.channel.as_ref()))
            .collect::<Vec<String>>();

        vec![WsMessage::Text(
            serde_json::json!({
                "method": "SUBSCRIBE",
                "params": stream_names,
                "id": 1
            })
            .to_string(),
        )]
    }

    fn ping_interval() -> Option<PingInterval> {
        None
    }

    fn expected_responses(_: &Map<Instrument>) -> usize {
        1
    }

    fn subscription_timeout() -> std::time::Duration {
        crate::exchange::connector::DEFAULT_SUBSCRIPTION_TIMEOUT
    }
}

impl<Server> StreamSelector<PublicTrades> for Binance<Server>
where
    Server: ExchangeServer + Debug + Send + Sync,
{
    type Stream = ExchangeWsStream<StatelessTransformer<Self, PublicTrades, BinanceSpotTrade>>;
}

impl<'de, Server> serde::Deserialize<'de> for Binance<Server>
where
    Server: ExchangeServer,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let input = <String as serde::Deserialize>::deserialize(deserializer)?;
        let expected = Self::ID.as_str();

        if input.as_str() == Self::ID.as_str() {
            Ok(Self::default())
        } else {
            Err(serde::de::Error::invalid_value(
                serde::de::Unexpected::Str(input.as_str()),
                &expected,
            ))
        }
    }
}
impl<Server> serde::Serialize for Binance<Server>
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
