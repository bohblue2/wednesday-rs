use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use wednesday_model::{
    deserialization,
    enums::AggressorSide,
    events::MarketEvent,
    identifiers::{Exchange, ExchangeId},
    instruments::Instrument,
    trade::PublicTrade,
};

use crate::transformer::iterator::MarketIter;

use super::message::BybitPayload;

pub type BybitTrade = BybitPayload<Vec<BybitTradeInner>>;

/// ### Raw Payload Examples
/// See docs: <https://bybit-exchange.github.io/docs/v5/websocket/public/trade>
/// Spot Side::Buy Trade
///```json
/// {
///     "T": 1672304486865,
///     "s": "BTCUSDT",
///     "S": "Buy",
///     "v": "0.001",
///     "p": "16578.50",
///     "L": "PlusTick",
///     "i": "20f43950-d8dd-5b31-9112-a178eb6023af",
///     "BT": false
/// }
/// ```
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BybitTradeInner {
    #[serde(alias = "T", deserialize_with = "deserialization::de_u64_epoch_ms_as_datetime_utc")]
    pub exchange_ts: DateTime<Utc>,
    #[serde(rename = "s")]
    pub market: String,
    #[serde(rename = "S")]
    pub side: AggressorSide,
    #[serde(alias = "v", deserialize_with = "deserialization::de_str")]
    pub amount: f64,
    #[serde(alias = "p", deserialize_with = "deserialization::de_str")]
    pub price: f64,
    #[serde(rename = "i")]
    pub id: String,
}

impl From<(ExchangeId, Instrument, BybitTrade)> for MarketIter<PublicTrade> {
    fn from((exchange_id, instrument, trades): (ExchangeId, Instrument, BybitTrade)) -> Self {
        Self(
            trades
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod de {
        use wednesday_model::{deserialization::datetime_utc_from_epoch_duration, error::SocketError, identifiers::SubscriptionId};

        use super::*;
        use std::time::Duration;

        #[test]
        fn test_bybit_trade() {
            struct TestCase {
                input: &'static str,
                expected: Result<BybitTradeInner, SocketError>,
            }

            let tests = vec![
                // TC0: input BybitTradeInner is deserialised
                TestCase {
                    input: r#"
                        {
                            "T": 1672304486865,
                            "s": "BTCUSDT",
                            "S": "Buy",
                            "v": "0.001",
                            "p": "16578.50",
                            "L": "PlusTick",
                            "i": "20f43950-d8dd-5b31-9112-a178eb6023af",
                            "BT": false
                        }
                    "#,
                    expected: Ok(BybitTradeInner {
                        exchange_ts: datetime_utc_from_epoch_duration(Duration::from_millis(1672304486865)),
                        market: "BTCUSDT".to_string(),
                        side: AggressorSide::Buy,
                        amount: 0.001,
                        price: 16578.50,
                        id: "20f43950-d8dd-5b31-9112-a178eb6023af".to_string(),
                    }),
                },
                // TC1: input BybitTradeInner is deserialised
                TestCase {
                    input: r#"
                        {
                            "T": 1672304486865,
                            "s": "BTCUSDT",
                            "S": "Sell",
                            "v": "0.001",
                            "p": "16578.50",
                            "L": "PlusTick",
                            "i": "20f43950-d8dd-5b31-9112-a178eb6023af",
                            "BT": false
                        }
                    "#,
                    expected: Ok(BybitTradeInner {
                        exchange_ts: datetime_utc_from_epoch_duration(Duration::from_millis(1672304486865)),
                        market: "BTCUSDT".to_string(),
                        side: AggressorSide::Sell,
                        amount: 0.001,
                        price: 16578.50,
                        id: "20f43950-d8dd-5b31-9112-a178eb6023af".to_string(),
                    }),
                },
                // TC2: input BybitTradeInner is unable to be deserialised
                TestCase {
                    input: r#"
                        {
                            "T": 1672304486865,
                            "s": "BTCUSDT",
                            "S": "Unknown",
                            "v": "0.001",
                            "p": "16578.50",
                            "L": "PlusTick",
                            "i": "20f43950-d8dd-5b31-9112-a178eb6023af",
                            "BT": false
                        }
                    "#,
                    expected: Err(SocketError::Unsupported {
                        entity: "",
                        item: "".to_string(),
                    }),
                },
            ];

            for (index, test) in tests.into_iter().enumerate() {
                let actual = serde_json::from_str::<BybitTradeInner>(test.input);
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

        #[test]
        fn test_bybit_trade_payload() {
            struct TestCase {
                input: &'static str,
                expected: Result<BybitTrade, SocketError>,
            }

            let tests = vec![
                // TC0: input BybitTrade is deserialised
                TestCase {
                    input: r#"
                        {
                        "topic": "publicTrade.BTCUSDT",
                        "type": "snapshot",
                        "ts": 1672304486868,
                            "data": [
                                {
                                    "T": 1672304486865,
                                    "s": "BTCUSDT",
                                    "S": "Buy",
                                    "v": "0.001",
                                    "p": "16578.50",
                                    "L": "PlusTick",
                                    "i": "20f43950-d8dd-5b31-9112-a178eb6023af",
                                    "BT": false
                                },
                                {
                                    "T": 1672304486865,
                                    "s": "BTCUSDT",
                                    "S": "Sell",
                                    "v": "0.001",
                                    "p": "16578.50",
                                    "L": "PlusTick",
                                    "i": "20f43950-d8dd-5b31-9112-a178eb6023af",
                                    "BT": false
                                }
                            ]
                        }
                    "#,
                    expected: Ok(BybitTrade {
                        subscription_id: SubscriptionId("publicTrade|BTCUSDT".to_string()),
                        r#type: "snapshot".to_string(),
                        exchange_ts: datetime_utc_from_epoch_duration(Duration::from_millis(1672304486868)),
                        data: vec![
                            BybitTradeInner {
                                exchange_ts: datetime_utc_from_epoch_duration(Duration::from_millis(1672304486865)),
                                market: "BTCUSDT".to_string(),
                                side: AggressorSide::Buy,
                                amount: 0.001,
                                price: 16578.50,
                                id: "20f43950-d8dd-5b31-9112-a178eb6023af".to_string(),
                            },
                            BybitTradeInner {
                                exchange_ts: datetime_utc_from_epoch_duration(Duration::from_millis(1672304486865)),
                                market: "BTCUSDT".to_string(),
                                side: AggressorSide::Sell,
                                amount: 0.001,
                                price: 16578.50,
                                id: "20f43950-d8dd-5b31-9112-a178eb6023af".to_string(),
                            },
                        ],
                    }),
                },
                // TC1: input BybitTrade is invalid w/ no subscription_id
                TestCase {
                    input: r#"
                        {
                            "data": [
                                {
                                    "T": 1672304486865,
                                    "s": "BTCUSDT",
                                    "S": "Unknown",
                                    "v": "0.001",
                                    "p": "16578.50",
                                    "L": "PlusTick",
                                    "i": "20f43950-d8dd-5b31-9112-a178eb6023af",
                                    "BT": false
                                }
                            ]
                        }
                    "#,
                    expected: Err(SocketError::Unsupported {
                        entity: "",
                        item: "".to_string(),
                    }),
                },
                // TC2: input BybitTrade is invalid w/ invalid subscription_id format
                TestCase {
                    input: r#"
                        {
                        "topic": "publicTrade.BTCUSDT.should_not_be_present",
                        "type": "snapshot",
                        "ts": 1672304486868,
                            "data": [
                                {
                                    "T": 1672304486865,
                                    "s": "BTCUSDT",
                                    "S": "Buy",
                                    "v": "0.001",
                                    "p": "16578.50",
                                    "L": "PlusTick",
                                    "i": "20f43950-d8dd-5b31-9112-a178eb6023af",
                                    "BT": false
                                },
                                {
                                    "T": 1672304486865,
                                    "s": "BTCUSDT",
                                    "S": "Sell",
                                    "v": "0.001",
                                    "p": "16578.50",
                                    "L": "PlusTick",
                                    "i": "20f43950-d8dd-5b31-9112-a178eb6023af",
                                    "BT": false
                                }
                            ]
                        }
                    "#,
                    expected: Err(SocketError::Unsupported {
                        entity: "",
                        item: "".to_string(),
                    }),
                },
            ];

            for (index, test) in tests.into_iter().enumerate() {
                let actual = serde_json::from_str::<BybitTrade>(test.input);
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
