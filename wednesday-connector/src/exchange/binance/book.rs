use chrono::Utc;
use serde::{Deserialize, Deserializer, Serialize};
use wednesday_model::{deserialization, enums::{AggressorSide, BookSide}, events::{MarketEvent}, identifiers::{Exchange, ExchangeId, Identifier, SubscriptionId}, instruments::Instrument, orderbook::{Level, OrderBook, OrderBookSide}, trade::PublicTrade};
use crate::{exchange::binance::channel::BinanceChannel, subscriber::subscription::ExchangeSubscription};



#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct BinanceLevel {
    #[serde(deserialize_with = "deserialization::de_str")]
    pub price: f64,
    #[serde(deserialize_with = "deserialization::de_str")]
    pub amount: f64,
}

impl From<BinanceLevel> for Level {
    fn from(level: BinanceLevel) -> Self {
        Self {
            price: level.price,
            amount: level.amount,
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct BinanceOrderBookL2Snapshot {
    #[serde(rename = "lastUpdateId")]
    pub last_update_id: u64,
    pub bids: Vec<BinanceLevel>,
    pub asks: Vec<BinanceLevel>,
}

impl From<BinanceOrderBookL2Snapshot> for OrderBook {
    fn from(snapshot: BinanceOrderBookL2Snapshot) -> Self {
        Self {
            last_update_ts: Utc::now(),
            bids: OrderBookSide::new(BookSide::Bid, snapshot.bids),
            asks: OrderBookSide::new(BookSide::Ask, snapshot.asks),
        }
    }
}

// NOTE: This deserialization implementation has to be refactored.
pub fn de_ob_l2_subscription_id<'de, D>(deserializer: D) -> Result<SubscriptionId, D::Error>
where
    D: Deserializer<'de>,
{
    <&str as Deserialize>::deserialize(deserializer)
        .map(|market| ExchangeSubscription::from((BinanceChannel::ORDER_BOOK_L2, market)).id())
}

#[cfg(test)]
mod tests {
    mod de {
        use crate::exchange::binance::book::{BinanceLevel, BinanceOrderBookL2Snapshot};

        #[test]
        fn test_binance_level() {
            let input = r#"["4.00000200", "12.00000000"]"#;
            assert_eq!(
                serde_json::from_str::<BinanceLevel>(input).unwrap(),
                BinanceLevel {
                    price: 4.00000200,
                    amount: 12.0
                },
            )
        }


        #[test]
        fn test_binance_order_book_l2_snapshot() {
            struct TestCase {
                input: &'static str,
                expected: BinanceOrderBookL2Snapshot,
            }

            let tests = vec![
                TestCase {
                    // TC0: valid Spot BinanceOrderBookL2Snapshot
                    input: r#"
                    {
                        "lastUpdateId": 1027024,
                        "bids": [
                            [
                                "4.00000000",
                                "431.00000000"
                            ]
                        ],
                        "asks": [
                            [
                                "4.00000200",
                                "12.00000000"
                            ]
                        ]
                    }
                    "#,
                    expected: BinanceOrderBookL2Snapshot {
                        last_update_id: 1027024,
                        bids: vec![BinanceLevel {
                            price: 4.0,
                            amount: 431.0,
                        }],
                        asks: vec![BinanceLevel {
                            price: 4.00000200,
                            amount: 12.0,
                        }],
                    },
                },
                TestCase {
                    // TC1: valid FuturePerpetual BinanceOrderBookL2Snapshot
                    input: r#"
                    {
                        "lastUpdateId": 1027024,
                        "E": 1589436922972,
                        "T": 1589436922959,
                        "bids": [
                            [
                                "4.00000000",
                                "431.00000000"
                            ]
                        ],
                        "asks": [
                            [
                                "4.00000200",
                                "12.00000000"
                            ]
                        ]
                    }
                    "#,
                    expected: BinanceOrderBookL2Snapshot {
                        last_update_id: 1027024,
                        bids: vec![BinanceLevel {
                            price: 4.0,
                            amount: 431.0,
                        }],
                        asks: vec![BinanceLevel {
                            price: 4.00000200,
                            amount: 12.0,
                        }],
                    },
                },
            ];

            for (index, test) in tests.into_iter().enumerate() {
                assert_eq!(
                    serde_json::from_str::<BinanceOrderBookL2Snapshot>(test.input).unwrap(),
                    test.expected,
                    "TC{} failed",
                    index
                );
            }
        }
    }
}