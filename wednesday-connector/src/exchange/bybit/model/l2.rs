use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Deserializer, Serialize};
use tokio::sync::mpsc::UnboundedSender;
use wednesday_model::{
    deserialization,
    error::DataError,
    identifiers::{Identifier, SubscriptionId},
    instruments::Instrument,
    orderbook::{Level, OrderBook},
};

use crate::{
    exchange::bybit::channel::BybitChannel,
    protocol::http::websocket::WsMessage,
    subscriber::subscription::ExchangeSubscription,
    transformer::updater::{InstrumentOrderBook, OrderBookUpdater},
};

use super::message::BybitPayload;

pub type BybitOrderBookL2 = BybitPayload<BybitOrderBookL2Delta>;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct BybitLevel {
    #[serde(deserialize_with = "deserialization::de_str")]
    pub price: f64,
    #[serde(deserialize_with = "deserialization::de_str")]
    pub amount: f64,
}

impl From<BybitLevel> for Level {
    fn from(value: BybitLevel) -> Self {
        Self {
            price: value.price,
            amount: value.amount,
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct BybitOrderBookL2Delta {
    #[serde(alias = "s")]
    pub symbol: String,
    #[serde(alias = "u")]
    pub last_update_id: u64,
    #[serde(alias = "seq")]
    pub sequence: u64,
    #[serde(alias = "a", deserialize_with = "crate::exchange::bybit::model::l2::de_ob_l2_levels")]
    pub asks: Vec<BybitLevel>,
    #[serde(alias = "b", deserialize_with = "crate::exchange::bybit::model::l2::de_ob_l2_levels")]
    pub bids: Vec<BybitLevel>,
}

// impl Identifier<Option<SubscriptionId>> for BybitOrderBookL2Delta {
//     fn id(&self) -> Option<SubscriptionId> {
//         Some(self.subscription_id.clone())
//     }
// }

// NOTE: This deserialization implementation has to be refactored.
pub fn de_ob_l2_subscription_id<'de, D>(deserializer: D) -> Result<SubscriptionId, D::Error>
where
    D: Deserializer<'de>,
{
    <&str as Deserialize>::deserialize(deserializer).map(|market| ExchangeSubscription::from((BybitChannel::ORDER_BOOK_L2, market)).id())
}

pub fn de_ob_l2_levels<'de, D>(deserializer: D) -> Result<Vec<BybitLevel>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw_levels: Vec<[String; 2]> = Deserialize::deserialize(deserializer)?;
    let levels = raw_levels
        .into_iter()
        .map(|[price, amount]| BybitLevel {
            price: price.parse().unwrap(),
            amount: amount.parse().unwrap(),
        })
        .collect();
    Ok(levels)
}

// See docs: https://bybit-exchange.github.io/docs/v5/websocket/public/orderbook
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct BybitBookUpdater {
    pub updates_processed: u64,
    pub last_update_id: u64,
}

impl BybitBookUpdater {
    pub fn new(last_update_id: u64) -> Self {
        Self {
            updates_processed: 0,
            last_update_id,
        }
    }
}

#[async_trait]
impl OrderBookUpdater for BybitBookUpdater {
    type OrderBook = OrderBook;
    type Update = BybitOrderBookL2;

    async fn init<Exchange, Kind>(_: UnboundedSender<WsMessage>, instrument: Instrument) -> Result<InstrumentOrderBook<Self>, DataError>
    where
        Exchange: Send,
        Kind: Send,
    {
        // Bybit does not provide a snapshot for the order book (does not provide a snapshot via REST API)
        // Therefore, a Dummy InstrumentOrderBook is returned for now
        Ok(InstrumentOrderBook {
            instrument,
            updater: Self::new(0),
            book: OrderBook::default(),
        })
    }

    fn update(&mut self, book: &mut Self::OrderBook, update: Self::Update) -> Result<Option<Self::OrderBook>, DataError> {
        if update.data.last_update_id <= self.last_update_id {
            return Ok(None);
        }
        self.updates_processed += 1;
        self.last_update_id = update.data.last_update_id;
        book.last_update_ts = Utc::now();
        book.bids.upsert(update.data.bids);
        book.asks.upsert(update.data.asks);
        Ok(Some(book.snapshot()))
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;
    use wednesday_model::deserialization::datetime_utc_from_epoch_duration;

    use super::*;

    #[test]
    fn test_bybit_futures_order_book_l2_deltas() {
        let input = r#"
        {
            "topic": "orderbook.50.BTCUSDT",
            "type": "delta",
            "ts": 1687940967466,
            "data": {
                "s": "BTCUSDT",
                "b": [
                    ["30247.20", "30.028"],
                    ["30245.40", "0.224"],
                    ["30242.10", "1.593"],
                    ["30240.30", "1.305"],
                    ["30240.00", "0"]
                ],
                "a": [
                    ["30248.70", "0"],
                    ["30249.30", "0.892"],
                    ["30249.50", "1.778"],
                    ["30249.60", "0"],
                    ["30251.90", "2.947"],
                    ["30252.20", "0.659"],
                    ["30252.50", "4.591"]
                ],
                "u": 177400507,
                "seq": 66544703342
            },
            "cts": 1687940967464
        }
        "#;
        let parsed: BybitOrderBookL2 = serde_json::from_str(input).unwrap();
        assert_eq!(
            parsed,
            BybitOrderBookL2 {
                subscription_id: SubscriptionId::from("orderbook.50|BTCUSDT"),
                r#type: "delta".to_string(),
                exchange_ts: datetime_utc_from_epoch_duration(Duration::from_millis(1687940967466)),
                data: BybitOrderBookL2Delta {
                    symbol: "BTCUSDT".to_string(),
                    last_update_id: 177400507,
                    sequence: 66544703342,
                    bids: vec![
                        BybitLevel {
                            price: 30247.20,
                            amount: 30.028
                        },
                        BybitLevel {
                            price: 30245.40,
                            amount: 0.224
                        },
                        BybitLevel {
                            price: 30242.10,
                            amount: 1.593
                        },
                        BybitLevel {
                            price: 30240.30,
                            amount: 1.305
                        },
                        BybitLevel {
                            price: 30240.00,
                            amount: 0.0
                        },
                    ],
                    asks: vec![
                        BybitLevel {
                            price: 30248.70,
                            amount: 0.0
                        },
                        BybitLevel {
                            price: 30249.30,
                            amount: 0.892
                        },
                        BybitLevel {
                            price: 30249.50,
                            amount: 1.778
                        },
                        BybitLevel {
                            price: 30249.60,
                            amount: 0.0
                        },
                        BybitLevel {
                            price: 30251.90,
                            amount: 2.947
                        },
                        BybitLevel {
                            price: 30252.20,
                            amount: 0.659
                        },
                        BybitLevel {
                            price: 30252.50,
                            amount: 4.591
                        },
                    ],
                }
            }
        );
    }
    mod bybit_futures_book_updater {
        use chrono::Utc;
        use wednesday_model::{
            enums::BookSide,
            identifiers::SubscriptionId,
            orderbook::{Level, OrderBook, OrderBookSide},
        };

        use crate::exchange::bybit::model::l2::{
            tests::{BybitPayload, DataError, OrderBookUpdater},
            BybitBookUpdater, BybitLevel, BybitOrderBookL2Delta,
        };

        #[test]
        fn update() {
            struct TestCase {
                updater: BybitBookUpdater,
                book: OrderBook,
                input_update: BybitPayload<BybitOrderBookL2Delta>,
                expected: Result<Option<OrderBook>, DataError>,
            }

            let time = Utc::now();

            let tests = vec![
                TestCase {
                    updater: BybitBookUpdater {
                        updates_processed: 0,
                        last_update_id: 0,
                    },
                    book: OrderBook {
                        last_update_ts: time,
                        bids: OrderBookSide::new(
                            BookSide::Bid,
                            vec![Level {
                                price: 30247.20,
                                amount: 30.028,
                            }],
                        ),
                        asks: OrderBookSide::new(
                            BookSide::Ask,
                            vec![Level {
                                price: 30248.70,
                                amount: 0.0,
                            }],
                        ),
                    },
                    input_update: BybitPayload {
                        subscription_id: SubscriptionId::from("orderbook.50|BTCUSDT"),
                        r#type: "delta".to_string(),
                        exchange_ts: time,
                        data: BybitOrderBookL2Delta {
                            symbol: "BTCUSDT".to_string(),
                            last_update_id: 177400507,
                            sequence: 66544703342,
                            bids: vec![
                                BybitLevel {
                                    price: 30247.20,
                                    amount: 30.028,
                                },
                                BybitLevel {
                                    price: 30245.40,
                                    amount: 0.224,
                                },
                                BybitLevel {
                                    price: 30242.10,
                                    amount: 1.593,
                                },
                                BybitLevel {
                                    price: 30240.30,
                                    amount: 1.305,
                                },
                                BybitLevel {
                                    price: 30240.00,
                                    amount: 0.0,
                                },
                            ],
                            asks: vec![
                                BybitLevel {
                                    price: 30248.70,
                                    amount: 0.0,
                                },
                                BybitLevel {
                                    price: 30249.30,
                                    amount: 0.892,
                                },
                                BybitLevel {
                                    price: 30249.50,
                                    amount: 1.778,
                                },
                                BybitLevel {
                                    price: 30249.60,
                                    amount: 0.0,
                                },
                                BybitLevel {
                                    price: 30251.90,
                                    amount: 2.947,
                                },
                                BybitLevel {
                                    price: 30252.20,
                                    amount: 0.659,
                                },
                                BybitLevel {
                                    price: 30252.50,
                                    amount: 4.591,
                                },
                            ],
                        },
                    },
                    expected: Ok(Some(OrderBook {
                        last_update_ts: time,
                        bids: OrderBookSide::new(
                            BookSide::Bid,
                            vec![
                                Level {
                                    price: 30247.20,
                                    amount: 30.028,
                                },
                                Level {
                                    price: 30245.40,
                                    amount: 0.224,
                                },
                                Level {
                                    price: 30242.10,
                                    amount: 1.593,
                                },
                                Level {
                                    price: 30240.30,
                                    amount: 1.305,
                                },
                            ],
                        ),
                        asks: OrderBookSide::new(
                            BookSide::Ask,
                            vec![
                                Level {
                                    price: 30249.30,
                                    amount: 0.892,
                                },
                                Level {
                                    price: 30249.50,
                                    amount: 1.778,
                                },
                                Level {
                                    price: 30251.90,
                                    amount: 2.947,
                                },
                                Level {
                                    price: 30252.20,
                                    amount: 0.659,
                                },
                                Level {
                                    price: 30252.50,
                                    amount: 4.591,
                                },
                            ],
                        ),
                    })),
                },
                TestCase {
                    updater: BybitBookUpdater {
                        updates_processed: 0,
                        last_update_id: 0,
                    },
                    book: OrderBook {
                        last_update_ts: time,
                        bids: OrderBookSide::new(BookSide::Bid, vec![Level::new(80, 1), Level::new(10, 1), Level::new(90, 1)]),
                        asks: OrderBookSide::new(BookSide::Ask, vec![Level::new(150, 1), Level::new(110, 1), Level::new(120, 1)]),
                    },
                    input_update: BybitPayload {
                        subscription_id: SubscriptionId::from("orderbook.50|BTCUSDT"),
                        r#type: "delta".to_string(),
                        exchange_ts: time,
                        data: BybitOrderBookL2Delta {
                            symbol: "BTCUSDT".to_string(),
                            last_update_id: 1,
                            sequence: 1,
                            bids: vec![BybitLevel { price: 80.0, amount: 0.0 }, BybitLevel { price: 90.0, amount: 10.0 }],
                            asks: vec![BybitLevel { price: 200.0, amount: 1.0 }, BybitLevel { price: 500.0, amount: 0.0 }],
                        },
                    },
                    expected: Ok(Some(OrderBook {
                        last_update_ts: time,
                        bids: OrderBookSide::new(BookSide::Bid, vec![Level::new(90, 10), Level::new(10, 1)]),
                        asks: OrderBookSide::new(
                            BookSide::Ask,
                            vec![Level::new(110, 1), Level::new(120, 1), Level::new(150, 1), Level::new(200, 1)],
                        ),
                    })),
                },
            ];

            for (index, mut test) in tests.into_iter().enumerate() {
                let actual = test.updater.update(&mut test.book, test.input_update);

                match (actual, test.expected) {
                    (Ok(Some(actual)), Ok(Some(expected))) => {
                        // Replace time with deterministic timestamp
                        let actual = OrderBook {
                            last_update_ts: time,
                            ..actual
                        };
                        assert_eq!(actual, expected, "TC{} failed", index)
                    },
                    (Ok(None), Ok(None)) => {
                        // Test passed
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
