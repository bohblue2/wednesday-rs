use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use wednesday_model::{deserialization, enums::AggressorSide, events::{MarketEvent}, identifiers::{Exchange, ExchangeId, Identifier, SubscriptionId}, instruments::Instrument, trade::PublicTrade};

use crate::{exchange::binance::channel::BinanceChannel, subscriber::subscription::ExchangeSubscription, transformer::iterator::MarketIter};

/// Binance real-time trade message.
///
/// Note:
/// For [`BinanceFuturesUsd`](super::futures::BinanceFuturesUsd) this real-time stream is
/// undocumented.
///
/// See discord: <https://discord.com/channels/910237311332151317/923160222711812126/975712874582388757>
///
/// ### Raw Payload Examples
/// See docs: <https://binance-docs.github.io/apidocs/spot/en/#trade-streams>
/// #### Spot Side::Buy Trade
/// ```json
/// {
///     "e":"trade",
///     "E":1649324825173,
///     "s":"ETHUSDT",
///     "t":1000000000,
///     "p":"10000.19",
///     "q":"0.239000",
///     "b":10108767791,
///     "a":10108764858,
///     "T":1749354825200,
///     "m":false,
///     "M":true
/// }
/// ```
///
/// #### FuturePerpetual Side::Sell Trade
/// ```json
/// {
///     "e": "trade",
///     "E": 1649839266194,
///     "T": 1749354825200,
///     "s": "ETHUSDT",
///     "t": 1000000000,
///     "p":"10000.19",
///     "q":"0.239000",
///     "X": "MARKET",
///     "m": true
/// }
/// ```
#[derive(Clone, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
pub struct BinanceSpotTrade {
    #[serde(alias = "s", deserialize_with = "de_trade_subscription_id")]
    pub subscription_id: SubscriptionId,
    #[serde(
        alias = "T",
        deserialize_with = "deserialization::de_u64_epoch_ms_as_datetime_utc"
    )]
    pub time: DateTime<Utc>,
    #[serde(alias = "t")]
    pub id: u64,
    #[serde(alias = "p", deserialize_with = "deserialization::de_str")]
    pub price: f64,
    #[serde(alias = "q", deserialize_with = "deserialization::de_str")]
    pub amount: f64,
    #[serde(alias = "m", deserialize_with = "de_side_from_buyer_is_maker")]
    pub side: AggressorSide,
}

impl Identifier<Option<SubscriptionId>> for BinanceSpotTrade {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.subscription_id.clone())
    }
}

impl From<(ExchangeId, Instrument, BinanceSpotTrade)> for MarketIter<PublicTrade> {
    fn from((exchange_id, instrument, trade): (ExchangeId, Instrument, BinanceSpotTrade)) -> Self {
        Self(vec![Ok(MarketEvent {
            exchange_ts: trade.time,
            local_ts: Utc::now(),
            exchange: Exchange::from(exchange_id),
            instrument,
            kind: PublicTrade {
                id: trade.id.to_string(), 
                price: trade.price,
                quantity: trade.amount,
                aggressor_side: trade.side,
            },
        })])
    }
}

/// Deserialize a [`BinanceSpotTrade`] "s" (eg/ "BTCUSDT") as the associated [`SubscriptionId`]
/// (eg/ "@trade|BTCUSDT").
pub fn de_trade_subscription_id<'de, D>(deserializer: D) -> Result<SubscriptionId, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    <&str as Deserialize>::deserialize(deserializer)
        .map(|market| ExchangeSubscription::from((BinanceChannel::TRADES, market)).id())
}

/// Deserialize a [`BinanceSpotTrade`] "buyer_is_maker" boolean field to a Barter [`Side`].
///
/// Variants:
/// buyer_is_maker => Side::Sell
/// !buyer_is_maker => Side::Buy
pub fn de_side_from_buyer_is_maker<'de, D>(deserializer: D) -> Result<AggressorSide, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    Deserialize::deserialize(deserializer).map(|buyer_is_maker| {
        if buyer_is_maker {
            AggressorSide::Sell
        } else {
            AggressorSide::Buy
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    mod de {
        use super::*;
        use deserialization::datetime_utc_from_epoch_duration;
        use serde::de::Error;
        use wednesday_model::error::SocketError;
        use std::time::Duration;

        #[test]
        fn test_binance_trade() {
            struct TestCase {
                input: &'static str,
                expected: Result<BinanceSpotTrade, SocketError>,
            }

            let tests = vec![
                TestCase {
                    // TC0: Spot trade valid
                    input: r#"
                    {
                        "e":"trade","E":1649324825173,"s":"ETHUSDT","t":1000000000,
                        "p":"10000.19","q":"0.239000","b":10108767791,"a":10108764858,
                        "T":1749354825200,"m":false,"M":true
                    }
                    "#,
                    expected: Ok(BinanceSpotTrade {
                        subscription_id: SubscriptionId::from("@trade|ETHUSDT"),
                        time: datetime_utc_from_epoch_duration(Duration::from_millis(
                            1749354825200,
                        )),
                        id: 1000000000,
                        price: 10000.19,
                        amount: 0.239000,
                        side: AggressorSide::Buy,
                    }),
                },
                TestCase {
                    // TC1: Spot trade malformed w/ "yes" is_buyer_maker field
                    input: r#"{
                        "e":"trade","E":1649324825173,"s":"ETHUSDT","t":1000000000,
                        "p":"10000.19000000","q":"0.239000","b":10108767791,"a":10108764858,
                        "T":1649324825173,"m":"yes","M":true
                    }"#,
                    expected: Err(SocketError::DeserializingJson {
                        error: serde_json::Error::custom("").to_string(),
                        payload: "".to_owned(),
                    }),
                },
                TestCase {
                    // TC2: FuturePerpetual trade w/ type MARKET
                    input: r#"
                    {
                        "e": "trade","E": 1649839266194,"T": 1749354825200,"s": "ETHUSDT",
                        "t": 1000000000,"p":"10000.19","q":"0.239000","X": "MARKET","m": true
                    }
                    "#,
                    expected: Ok(BinanceSpotTrade {
                        subscription_id: SubscriptionId::from("@trade|ETHUSDT"),
                        time: datetime_utc_from_epoch_duration(Duration::from_millis(
                            1749354825200,
                        )),
                        id: 1000000000,
                        price: 10000.19,
                        amount: 0.239000,
                        side: AggressorSide::Sell,
                    }),
                },
                TestCase {
                    // TC3: FuturePerpetual trade w/ type LIQUIDATION
                    input: r#"
                    {
                        "e": "trade","E": 1649839266194,"T": 1749354825200,"s": "ETHUSDT",
                        "t": 1000000000,"p":"10000.19","q":"0.239000","X": "LIQUIDATION","m": false
                    }
                    "#,
                    expected: Ok(BinanceSpotTrade {
                        subscription_id: SubscriptionId::from("@trade|ETHUSDT"),
                        time: datetime_utc_from_epoch_duration(Duration::from_millis(
                            1749354825200,
                        )),
                        id: 1000000000,
                        price: 10000.19,
                        amount: 0.239000,
                        side: AggressorSide::Buy,
                    }),
                },
                TestCase {
                    // TC4: FuturePerpetual trade w/ type LIQUIDATION
                    input: r#"{
                        "e": "trade","E": 1649839266194,"T": 1749354825200,"s": "ETHUSDT",
                        "t": 1000000000,"p":"10000.19","q":"0.239000","X": "INSURANCE_FUND","m": false
                    }"#,
                    expected: Ok(BinanceSpotTrade {
                        subscription_id: SubscriptionId::from("@trade|ETHUSDT"),
                        time: datetime_utc_from_epoch_duration(Duration::from_millis(
                            1749354825200,
                        )),
                        id: 1000000000,
                        price: 10000.19,
                        amount: 0.239000,
                        side: AggressorSide::Buy,
                    }),
                },
            ];

            for (index, test) in tests.into_iter().enumerate() {
                let actual = serde_json::from_str::<BinanceSpotTrade>(test.input);
                match (actual, test.expected) {
                    (Ok(actual), Ok(expected)) => {
                        assert_eq!(actual, expected, "TC{} failed", index)
                    }
                    (Err(_), Err(_)) => {
                        // Test passed
                    }
                    (actual, expected) => {
                        // Test failed
                        panic!("TC{index} failed because actual != expected. \nActual: {actual:?}\nExpected: {expected:?}\n");
                    }
                }
            }
        }
    }
}
