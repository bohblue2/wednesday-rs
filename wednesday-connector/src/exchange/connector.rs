use serde::{de::DeserializeOwned, Deserialize, Serialize};
use url::Url;
use wednesday_model::{error::SocketError, identifiers::ExchangeId, instruments::Instrument};
use std::{fmt::Debug, time::Duration};

use crate::{protocol::http::websocket::{PingInterval, WsMessage}, subscriber::{subscription::{ExchangeSubscription, Map}, validator::{SubscriptionValidator, Validator}, Subscriber}};

pub const DEFAULT_SUBSCRIPTION_TIMEOUT: Duration = Duration::from_secs(10);

pub trait Connector 
where
    Self: Clone + Default + Debug + for<'de> Deserialize<'de> + Serialize + Sized,
{
    const ID: ExchangeId;
    type Channel: AsRef<str>;
    type Market: AsRef<str>;
    type Subscriber: Subscriber;
    type SubscriptionValidator: SubscriptionValidator;
    type SubscriptionResponse: Validator + Debug + DeserializeOwned;

    fn url() -> Result<Url, SocketError>;

    fn ping_interval() -> Option<PingInterval> { None }

    fn expected_responses(map: &Map<Instrument>) -> usize { map.0.len() }

    fn subscription_timeout() -> Duration { DEFAULT_SUBSCRIPTION_TIMEOUT }

    fn requests(
        exchange_subscriptions: Vec<ExchangeSubscription<Self::Channel, Self::Market>>
    ) -> Vec<WsMessage>;
}

pub trait ExchangeServer: Default + Debug + Clone + Send {
    const ID: ExchangeId;
    fn ws_url() -> &'static str;
}