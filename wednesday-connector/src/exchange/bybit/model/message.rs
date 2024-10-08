use chrono::{DateTime, Utc};
use serde::{
    de::{Error, Unexpected},
    Deserialize, Serialize,
};
use tracing::debug;
use wednesday_model::{
    deserialization, events::MarketEvent, identifiers::{Exchange, ExchangeId, Identifier, SubscriptionId}, instruments::Instrument, trade::PublicTrade
};

use crate::{exchange::bybit::subscription::BybitSubscriptionResponse, transformer::iterator::MarketIter};

use super::{l2::BybitOrderBookL2, trade::BybitTrade};

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BybitMessage {
    Response(BybitSubscriptionResponse),
    Trade(BybitTrade),
    OrderBook(BybitOrderBookL2)
}

impl Identifier<Option<SubscriptionId>> for BybitMessage {
    fn id(&self) -> Option<SubscriptionId> {
        match self {
            BybitMessage::Trade(trade) => trade.id(),
            BybitMessage::OrderBook(order_book) => order_book.id(),
            BybitMessage::Response(pong_response) => pong_response.id(),
        }
    }
}


impl From<(ExchangeId, Instrument, BybitMessage)> for MarketIter<PublicTrade> {
    fn from((exchange_id, instrument, message): (ExchangeId, Instrument, BybitMessage)) -> Self {
        match message {
            BybitMessage::Response(response) => {
                debug!("HandlingPOONG: {:?}", response);
                Self(vec![])
            },
            BybitMessage::Trade(trades) => {
                Self(trades
                        .data
                        .into_iter()
                        .map(|trade| {
                            Ok(MarketEvent {
                                exchange_ts: trade.exchange_ts,
                                local_ts: Utc::now(),
                                exchange: Exchange::from(exchange_id),
                                instrument: instrument.clone(),
                                kind: PublicTrade {
                                    id: trade.id,
                                    price: trade.price,
                                    quantity: trade.amount,
                                    aggressor_side: trade.side,
                                },
                            })
                        })
                        .collect(),
            )
            },
            BybitMessage::OrderBook(order_book) => { Self(vec![]) },
        }



    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Serialize, Deserialize)]
pub struct BybitPayload<T> {
    #[serde(alias = "topic", deserialize_with = "de_bybit_message_subscription_id")]
    pub subscription_id: SubscriptionId,
    #[serde(alias = "type")]
    pub r#type: String,
    #[serde(alias = "ts", deserialize_with = "deserialization::de_u64_epoch_ms_as_datetime_utc")]
    pub exchange_ts: DateTime<Utc>,
    pub data: T,
}

pub fn de_bybit_message_subscription_id<'de, D>(deserializer: D) -> Result<SubscriptionId, D::Error>
where
    D: serde::Deserializer<'de>,
{
    // parsing Example
    // - publicTrade.BTCUSDT
    // - orderbook.50.BTCUSDT
    let input = <&str as Deserialize>::deserialize(deserializer)?;
    let tokens: Vec<&str> = input.split(".").collect();

    let topic_type = tokens.get(0);
    let mut level: Option<&str> = None;
    let mut market: Option<&str> = None;

    if topic_type == Some(&"publicTrade") {
        market = Some(tokens[1]);
    } else if topic_type == Some(&"orderbook") {
        level = Some(tokens[1]);
        market = Some(tokens[2]);
    }

    match (topic_type, level, market) {
        (Some(&"publicTrade"), None, market) => {
            if tokens.len() > 2 {
                return Err(Error::invalid_value(
                    Unexpected::Str(input),
                    &"invalid message type expected pattern: <type>.<symbol>",
                ));
            }
            let subscription_id = format!("{}|{}", topic_type.unwrap(), market.unwrap_or_default());
            Ok(SubscriptionId::from(subscription_id))
        },
        (Some(&"orderbook"), level, market) => {
            let subscription_id = format!(
                "{}.{}|{}",
                topic_type.unwrap(),
                level.unwrap_or_default(),
                market.unwrap_or_default()
            );
            Ok(SubscriptionId::from(subscription_id))
        },
        _ => Err(Error::invalid_value(
            Unexpected::Str(input),
            &"invalid message type expected pattern: <type>.<symbol>",
        )),
    }
}

impl Identifier<Option<SubscriptionId>> for BybitOrderBookL2 {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.subscription_id.clone())
    }
}

impl Identifier<Option<SubscriptionId>> for BybitTrade {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.subscription_id.clone())
    }
}

#[cfg(test)]
mod tests {

    mod de {
        use wednesday_model::error::SocketError;

        use crate::exchange::bybit::subscription::{BybitReturnMessage, BybitSubscriptionResponse};

        #[test]
        fn test_bybit_pong() {
            struct TestCase {
                input: &'static str,
                expected: Result<BybitSubscriptionResponse, SocketError>,
            }

            let tests = vec![
                // TC0: input BybitResponse(Pong) is deserialised
                TestCase {
                    input: r#"
                        {
                            "success": true,
                            "ret_msg": "pong",
                            "conn_id": "0970e817-426e-429a-a679-ff7f55e0b16a",
                            "op": "ping"
                        }
                    "#,
                    expected: Ok(BybitSubscriptionResponse {
                        success: true,
                        ret_msg: BybitReturnMessage::Pong,
                        conn_id: "0970e817-426e-429a-a679-ff7f55e0b16a".to_owned(),
                        req_id: "".to_owned(),
                        op: "ping".to_owned(),
                    }),
                },
            ];

            for (index, test) in tests.into_iter().enumerate() {
                let actual = serde_json::from_str::<BybitSubscriptionResponse>(test.input);
                match (actual, test.expected) {
                    (Ok(actual), Ok(expected)) => {
                        assert_eq!(actual, expected, "TC{} failed", index)
                    },
                    (Err(_), Err(_)) => {
                        // Test passed
                    },
                    (actual, expected) => {
                        // Test failed
                        panic!("TC{index} failed because actual != expected. \nActual: {actual:?}\nExpected: {expected:?}\n");
                    },
                }
            }
        }
    }
}
