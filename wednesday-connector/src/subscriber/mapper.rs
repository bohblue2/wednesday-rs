use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use wednesday_model::identifiers::{Identifier, SubscriptionId};

use crate::exchange::connector::Connector;

use super::subscription::{ExchangeSubscription, Map, Subscription, SubscriptionKind, SubscriptionMeta};



/// Defines how to map a collection of Barter [`Subscription`]s into exchange specific
/// [`SubscriptionMeta`], containing subscription payloads that are sent to the exchange.
pub trait SubscriptionMapper {
    fn map<Exchange, Kind>(subscriptions: &[Subscription<Exchange, Kind>]) -> SubscriptionMeta
    where
        Exchange: Connector,
        Kind: SubscriptionKind,
        Subscription<Exchange, Kind>: Identifier<Exchange::Channel> + Identifier<Exchange::Market>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub struct WsSubscriptionMapper;

impl SubscriptionMapper for WsSubscriptionMapper {
    fn map<Exchange, Kind>(subscriptions: &[Subscription<Exchange, Kind>]) -> SubscriptionMeta
    where
        Exchange: Connector,
        Kind: SubscriptionKind,
        Subscription<Exchange, Kind>: Identifier<Exchange::Channel> + Identifier<Exchange::Market>,
        ExchangeSubscription<Exchange::Channel, Exchange::Market>: Identifier<SubscriptionId>,
    {
        let mut instrument_map = Map(HashMap::with_capacity(subscriptions.len()));

        let exchange_subscriptions = subscriptions
            .iter()
            .map(|subscription| {
                // Translate Barter Subscription to exchange specific subscription
                let exchange_subscription = ExchangeSubscription::new(subscription);

                // Determine the SubscriptionId associated with this exchange specific subscription
                let subscription_id = exchange_subscription.id();

                instrument_map
                    .0
                    .insert(subscription_id, subscription.instrument.clone());

                exchange_subscription
            })
            .collect::<Vec<ExchangeSubscription<Exchange::Channel, Exchange::Market>>>();
        
        let subscriptions = Exchange::requests(exchange_subscriptions);

        SubscriptionMeta {
            instrument_map,
            subscriptions,
        }
    }
}