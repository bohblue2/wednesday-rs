use std::cmp::Ordering;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::debug;
use wednesday_macro::{DeSubscriptionKind, SerSubscriptionKind};

use crate::{calculator::{mid_price, volume_weighted_mid_price}, enums::BookSide, events::{MarketEvent}, identifiers::{Exchange, ExchangeId}, instruments::Instrument};



#[derive(Debug, PartialEq, Clone)]
pub struct OrderBookL1 {
    pub last_update_ts: DateTime<Utc>,
    pub best_bid: Level, 
    pub best_ask: Level,
}

impl OrderBookL1 {
    pub fn mid_price(&self) -> f64 {
        mid_price(self.best_bid.price, self.best_ask.price)
    }

    pub fn volume_weighed_mid_price(&self) -> f64 {
        volume_weighted_mid_price(self.best_bid, self.best_ask)
    }
}

#[derive(Debug, PartialEq, Clone, Copy, Default, Deserialize, Serialize)]
pub struct Level {
    pub price: f64,
    pub amount: f64,
}

impl Level {
    pub fn new<T>(price: T, amount: T) -> Self
    where
        T: Into<f64>,
    {
        Self {
            price: price.into(),
            amount: amount.into(),
        }
    }

    pub fn eq_price(&self, price: f64) -> bool {
        (price - self.price).abs() < f64::EPSILON
    }
    
    pub fn eq_amount(&self, amount: f64) -> bool {
        (amount - self.amount).abs() < f64::EPSILON
    }
}

impl<T> From<(T, T)> for Level
where
    T: Into<f64>,
{
    fn from((price, amount): (T, T)) -> Self {
        Self::new(price, amount)
    }
}

impl Ord for Level {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other)
            .unwrap_or_else(|| panic!("{:?}.partial._cmp({:?}) impossible", self, other))
    }
}

impl PartialOrd for Level {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.price.partial_cmp(&other.price)? {
            Ordering::Equal => self.amount.partial_cmp(&other.amount),
            ordering => Some(ordering),
        }
    }
}

// No need to implement this, since `PartialEq` is already implemented 
impl Eq for Level {}



#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct OrderBook {
    pub last_update_ts: DateTime<Utc>,
    pub bids: OrderBookSide,
    pub asks: OrderBookSide,
}

impl OrderBook {
    pub fn snapshot(&mut self) -> Self {
        self.bids.sort();
        self.asks.sort();
        self.clone()
    }

    pub fn mid_price(&self) -> Option<f64> {
        match (self.bids.levels.first(), self.asks.levels.first()) {
            (Some(best_bid), Some(best_ask)) => Some(mid_price(best_bid.price, best_ask.price)),
            (Some(best_bid), None) => Some(best_bid.price),
            (None, Some(best_ask)) => Some(best_ask.price),
            (None, None) => None,
        }
    }

    pub fn volume_weighed_mid_price(&self) -> Option<f64> {
        match (self.bids.levels.first(), self.asks.levels.first()) {
            (Some(best_bid), Some(best_ask)) => {
                Some(volume_weighted_mid_price(*best_bid, *best_ask))
            }
            (Some(best_bid), None) => Some(best_bid.price),
            (None, Some(best_ask)) => Some(best_ask.price),
            (None, None) => None,
        }
    }
}

impl Default for OrderBook {
    fn default() -> Self {
        Self {
            last_update_ts: Utc::now(),
            bids: OrderBookSide::new(BookSide::Bid, Vec::<Level>::new()),
            asks: OrderBookSide::new(BookSide::Ask, Vec::<Level>::new()),
        }
    }
}

#[derive(Debug, PartialEq, Clone, PartialOrd, Ord, Eq, Serialize, Deserialize)]
pub struct OrderBookSide {
    side: BookSide,
    pub levels: Vec<Level>
}

impl OrderBookSide {
    pub fn new<Iter, L>(side: BookSide, levels: Iter) -> Self
    where
        Iter: IntoIterator<Item = L>,
        L: Into<Level>,
    {
        Self {
            side,
            levels: levels.into_iter().map(L::into).collect()
        }
    }

    pub fn upsert<Iter, L>(&mut self, levels: Iter)
    where
        Iter: IntoIterator<Item = L>,
        L: Into<Level>,
    {
        levels
            .into_iter()
            .for_each(|level| self.upsert_single(level))
    }

    pub fn upsert_single<L>(&mut self, new_level: L)
    where
        L: Into<Level>,
    {
        let new_level = new_level.into();
        
        match self
            .levels
            .iter_mut()
            .enumerate()
            .find(|(_index, level)| level.eq_price(new_level.price))
        {
            Some((index, _)) if new_level.amount == 0.0 => {
                self.levels.remove(index);
            }

            Some((_, level)) => {
                *level = new_level;
            }

            None if new_level.amount > 0.0 => self.levels.push(new_level),

            _ => {
                // {"message":"Level to remove not found","new_level":"Level { price: 61067.39, amount: 0.0 } 
                // 만약 수정하려고 하는 가격 범위가 현재 미드 프라이스 기준으로 너무 멀리 100틱 이상 떨어져 있으면 무시.
                debug!(
                    ?new_level,
                    side = %self.side,
                    "Level to remove not found",
                );
            }
        };
    }

    pub fn sort(&mut self) {
        self.levels.sort_unstable();

        if let BookSide::Bid = self.side {
            self.levels.reverse();
        }
    }
}




#[cfg(test)]
mod tests {
    use super::*;

    mod order_book_l1 {
        use super::*;

        #[test]
        fn test_mid_price() {
            struct TestCase {
                input: OrderBookL1,
                expected: f64,
            }

            let tests = vec![
                TestCase {
                    // TC0
                    input: OrderBookL1 {
                        last_update_ts: Default::default(),
                        best_bid: Level::new(100, 999999),
                        best_ask: Level::new(200, 1),
                    },
                    expected: 150.0,
                },
                TestCase {
                    // TC1
                    input: OrderBookL1 {
                        last_update_ts: Default::default(),
                        best_bid: Level::new(50, 1),
                        best_ask: Level::new(250, 999999),
                    },
                    expected: 150.0,
                },
                TestCase {
                    // TC2
                    input: OrderBookL1 {
                        last_update_ts: Default::default(),
                        best_bid: Level::new(10, 999999),
                        best_ask: Level::new(250, 999999),
                    },
                    expected: 130.0,
                },
            ];

            for (index, test) in tests.into_iter().enumerate() {
                assert_eq!(test.input.mid_price(), test.expected, "TC{index} failed")
            }
        }

        #[test]
        fn test_volume_weighted_mid_price() {
            struct TestCase {
                input: OrderBookL1,
                expected: f64,
            }

            let tests = vec![
                TestCase {
                    // TC0: volume the same so should be equal to non-weighted mid price
                    input: OrderBookL1 {
                        last_update_ts: Default::default(),
                        best_bid: Level::new(100, 100),
                        best_ask: Level::new(200, 100),
                    },
                    expected: 150.0,
                },
                TestCase {
                    // TC1: volume affects mid-price
                    input: OrderBookL1 {
                        last_update_ts: Default::default(),
                        best_bid: Level::new(100, 600),
                        best_ask: Level::new(200, 1000),
                    },
                    expected: 137.5,
                },
                TestCase {
                    // TC2: volume the same and price the same
                    input: OrderBookL1 {
                        last_update_ts: Default::default(),
                        best_bid: Level::new(1000, 999999),
                        best_ask: Level::new(1000, 999999),
                    },
                    expected: 1000.0,
                },
            ];

            for (index, test) in tests.into_iter().enumerate() {
                assert_eq!(
                    test.input.volume_weighed_mid_price(),
                    test.expected,
                    "TC{index} failed"
                )
            }
        }
    }

    mod order_book {
        use super::*;
        use crate::calculator::mid_price;

        #[test]
        fn test_mid_price() {
            struct TestCase {
                input: OrderBook,
                expected: Option<f64>,
            }

            let tests = vec![
                TestCase {
                    // TC0: no levels so 0.0 mid-price
                    input: OrderBook {
                        last_update_ts: Default::default(),
                        bids: OrderBookSide {
                            side: BookSide::Bid,
                            levels: vec![],
                        },
                        asks: OrderBookSide {
                            side: BookSide::Ask,
                            levels: vec![],
                        },
                    },
                    expected: None,
                },
                TestCase {
                    // TC1: no asks in the book so take best bid price
                    input: OrderBook {
                        last_update_ts: Default::default(),
                        bids: OrderBookSide {
                            side: BookSide::Bid,
                            levels: vec![Level::new(100.0, 100.0), Level::new(50.0, 100.0)],
                        },
                        asks: OrderBookSide {
                            side: BookSide::Ask,
                            levels: vec![],
                        },
                    },
                    expected: Some(100.0),
                },
                TestCase {
                    // TC2: no bids in the book so take ask price
                    input: OrderBook {
                        last_update_ts: Default::default(),
                        bids: OrderBookSide {
                            side: BookSide::Bid,
                            levels: vec![],
                        },
                        asks: OrderBookSide {
                            side: BookSide::Ask,
                            levels: vec![Level::new(50.0, 100.0), Level::new(100.0, 100.0)],
                        },
                    },
                    expected: Some(50.0),
                },
                TestCase {
                    // TC3: best bid and ask amount is the same, so regular mid-price
                    input: OrderBook {
                        last_update_ts: Default::default(),
                        bids: OrderBookSide {
                            side: BookSide::Bid,
                            levels: vec![Level::new(100.0, 100.0), Level::new(50.0, 100.0)],
                        },
                        asks: OrderBookSide {
                            side: BookSide::Ask,
                            levels: vec![Level::new(200.0, 100.0), Level::new(300.0, 100.0)],
                        },
                    },
                    expected: Some(150.0),
                },
            ];

            for (index, test) in tests.into_iter().enumerate() {
                assert_eq!(test.input.mid_price(), test.expected, "TC{index} failed")
            }
        }

        #[test]
        fn test_volume_weighted_mid_price() {
            struct TestCase {
                input: OrderBook,
                expected: Option<f64>,
            }

            let tests = vec![
                TestCase {
                    // TC0: no levels so 0.0 mid-price
                    input: OrderBook {
                        last_update_ts: Default::default(),
                        bids: OrderBookSide {
                            side: BookSide::Bid,
                            levels: vec![],
                        },
                        asks: OrderBookSide {
                            side: BookSide::Ask,
                            levels: vec![],
                        },
                    },
                    expected: None,
                },
                TestCase {
                    // TC1: no asks in the book so take best bid price
                    input: OrderBook {
                        last_update_ts: Default::default(),
                        bids: OrderBookSide {
                            side: BookSide::Bid,
                            levels: vec![Level::new(100.0, 100.0), Level::new(50.0, 100.0)],
                        },
                        asks: OrderBookSide {
                            side: BookSide::Ask,
                            levels: vec![],
                        },
                    },
                    expected: Some(100.0),
                },
                TestCase {
                    // TC2: no bids in the book so take ask price
                    input: OrderBook {
                        last_update_ts: Default::default(),
                        bids: OrderBookSide {
                            side: BookSide::Bid,
                            levels: vec![],
                        },
                        asks: OrderBookSide {
                            side: BookSide::Ask,
                            levels: vec![Level::new(50.0, 100.0), Level::new(100.0, 100.0)],
                        },
                    },
                    expected: Some(50.0),
                },
                TestCase {
                    // TC3: best bid and ask amount is the same, so regular mid-price
                    input: OrderBook {
                        last_update_ts: Default::default(),
                        bids: OrderBookSide {
                            side: BookSide::Bid,
                            levels: vec![Level::new(100.0, 100.0), Level::new(50.0, 100.0)],
                        },
                        asks: OrderBookSide {
                            side: BookSide::Ask,
                            levels: vec![Level::new(200.0, 100.0), Level::new(300.0, 100.0)],
                        },
                    },
                    expected: Some(150.0),
                },
                TestCase {
                    // TC4: valid volume weighted mid-price
                    input: OrderBook {
                        last_update_ts: Default::default(),
                        bids: OrderBookSide {
                            side: BookSide::Bid,
                            levels: vec![Level::new(100.0, 3000.0), Level::new(50.0, 100.0)],
                        },
                        asks: OrderBookSide {
                            side: BookSide::Ask,
                            levels: vec![Level::new(200.0, 1000.0), Level::new(300.0, 100.0)],
                        },
                    },
                    expected: Some(175.0),
                },
            ];

            for (index, test) in tests.into_iter().enumerate() {
                assert_eq!(
                    test.input.volume_weighed_mid_price(),
                    test.expected,
                    "TC{index} failed"
                )
            }
        }
    }

    mod order_book_side {
        use super::*;

        #[test]
        fn test_upsert_single() {
            struct TestCase {
                book_side: OrderBookSide,
                new_level: Level,
                expected: OrderBookSide,
            }

            let tests = vec![
                TestCase {
                    // TC0: Level exists & new value is 0 => remove Level
                    book_side: OrderBookSide::new(
                        BookSide::Bid,
                        vec![Level::new(80, 1), Level::new(90, 1), Level::new(100, 1)],
                    ),
                    new_level: Level::new(100, 0),
                    expected: OrderBookSide::new(
                        BookSide::Bid,
                        vec![Level::new(80, 1), Level::new(90, 1)],
                    ),
                },
                TestCase {
                    // TC1: Level exists & new value is > 0 => replace Level
                    book_side: OrderBookSide::new(
                        BookSide::Bid,
                        vec![Level::new(80, 1), Level::new(90, 1), Level::new(100, 1)],
                    ),
                    new_level: Level::new(100, 10),
                    expected: OrderBookSide::new(
                        BookSide::Bid,
                        vec![Level::new(80, 1), Level::new(90, 1), Level::new(100, 10)],
                    ),
                },
                TestCase {
                    // TC2: Level does not exist & new value > 0 => insert new Level
                    book_side: OrderBookSide::new(
                        BookSide::Bid,
                        vec![Level::new(80, 1), Level::new(90, 1), Level::new(100, 1)],
                    ),
                    new_level: Level::new(110, 1),
                    expected: OrderBookSide::new(
                        BookSide::Bid,
                        vec![
                            Level::new(80, 1),
                            Level::new(90, 1),
                            Level::new(100, 1),
                            Level::new(110, 1),
                        ],
                    ),
                },
                TestCase {
                    // TC3: Level does not exist & new value is 0 => no change
                    book_side: OrderBookSide::new(
                        BookSide::Bid,
                        vec![Level::new(80, 1), Level::new(90, 1), Level::new(100, 1)],
                    ),
                    new_level: Level::new(110, 0),
                    expected: OrderBookSide::new(
                        BookSide::Bid,
                        vec![Level::new(80, 1), Level::new(90, 1), Level::new(100, 1)],
                    ),
                },
            ];

            for (index, mut test) in tests.into_iter().enumerate() {
                test.book_side.upsert_single(test.new_level);
                assert_eq!(test.book_side, test.expected, "TC{} failed", index);
            }
        }

        #[test]
        fn test_sort_bids() {
            struct TestCase {
                input: OrderBookSide,
                expected: OrderBookSide,
            }

            let tests = vec![
                TestCase {
                    // TC0: sorted correctly from reverse sorted
                    input: OrderBookSide::new(
                        BookSide::Bid,
                        vec![
                            Level::new(80, 1),
                            Level::new(90, 1),
                            Level::new(100, 1),
                            Level::new(110, 1),
                            Level::new(120, 1),
                        ],
                    ),
                    expected: OrderBookSide::new(
                        BookSide::Bid,
                        vec![
                            Level::new(120, 1),
                            Level::new(110, 1),
                            Level::new(100, 1),
                            Level::new(90, 1),
                            Level::new(80, 1),
                        ],
                    ),
                },
                TestCase {
                    // TC1: sorted correctly from partially sorted
                    input: OrderBookSide::new(
                        BookSide::Bid,
                        vec![
                            Level::new(120, 1),
                            Level::new(90, 1),
                            Level::new(80, 1),
                            Level::new(110, 1),
                            Level::new(100, 1),
                        ],
                    ),
                    expected: OrderBookSide::new(
                        BookSide::Bid,
                        vec![
                            Level::new(120, 1),
                            Level::new(110, 1),
                            Level::new(100, 1),
                            Level::new(90, 1),
                            Level::new(80, 1),
                        ],
                    ),
                },
                TestCase {
                    // TC1: sorted correctly from already sorted
                    input: OrderBookSide::new(
                        BookSide::Bid,
                        vec![
                            Level::new(120, 1),
                            Level::new(110, 1),
                            Level::new(100, 1),
                            Level::new(90, 1),
                            Level::new(80, 1),
                        ],
                    ),
                    expected: OrderBookSide::new(
                        BookSide::Bid,
                        vec![
                            Level::new(120, 1),
                            Level::new(110, 1),
                            Level::new(100, 1),
                            Level::new(90, 1),
                            Level::new(80, 1),
                        ],
                    ),
                },
            ];

            for (index, mut test) in tests.into_iter().enumerate() {
                test.input.sort();
                assert_eq!(test.input, test.expected, "TC{} failed", index);
            }
        }

        #[test]
        fn test_sort_asks() {
            struct TestCase {
                input: OrderBookSide,
                expected: OrderBookSide,
            }

            let tests = vec![
                TestCase {
                    // TC0: sorted correctly from already sorted
                    input: OrderBookSide::new(
                        BookSide::Ask,
                        vec![
                            Level::new(80, 1),
                            Level::new(90, 1),
                            Level::new(100, 1),
                            Level::new(110, 1),
                            Level::new(120, 1),
                        ],
                    ),
                    expected: OrderBookSide::new(
                        BookSide::Ask,
                        vec![
                            Level::new(80, 1),
                            Level::new(90, 1),
                            Level::new(100, 1),
                            Level::new(110, 1),
                            Level::new(120, 1),
                        ],
                    ),
                },
                TestCase {
                    // TC1: sorted correctly from partially sorted
                    input: OrderBookSide::new(
                        BookSide::Ask,
                        vec![
                            Level::new(120, 1),
                            Level::new(90, 1),
                            Level::new(80, 1),
                            Level::new(110, 1),
                            Level::new(100, 1),
                        ],
                    ),
                    expected: OrderBookSide::new(
                        BookSide::Ask,
                        vec![
                            Level::new(80, 1),
                            Level::new(90, 1),
                            Level::new(100, 1),
                            Level::new(110, 1),
                            Level::new(120, 1),
                        ],
                    ),
                },
                TestCase {
                    // TC1: sorted correctly from reverse sorted
                    input: OrderBookSide::new(
                        BookSide::Ask,
                        vec![
                            Level::new(120, 1),
                            Level::new(110, 1),
                            Level::new(100, 1),
                            Level::new(90, 1),
                            Level::new(80, 1),
                        ],
                    ),
                    expected: OrderBookSide::new(
                        BookSide::Ask,
                        vec![
                            Level::new(80, 1),
                            Level::new(90, 1),
                            Level::new(100, 1),
                            Level::new(110, 1),
                            Level::new(120, 1),
                        ],
                    ),
                },
            ];

            for (index, mut test) in tests.into_iter().enumerate() {
                test.input.sort();
                assert_eq!(test.input, test.expected, "TC{} failed", index);
            }
        }
    }

    mod level {
        use super::*;

        #[test]
        fn test_partial_ord() {
            struct TestCase {
                input_one: Level,
                input_two: Level,
                expected: Option<Ordering>,
            }

            let tests = vec![
                TestCase {
                    // TC0: Input One has higher price and higher quantity -> Greater
                    input_one: Level::new(100, 100),
                    input_two: Level::new(10, 10),
                    expected: Some(Ordering::Greater),
                },
                TestCase {
                    // TC1: Input One has higher price but same quantity -> Greater
                    input_one: Level::new(100, 100),
                    input_two: Level::new(10, 100),
                    expected: Some(Ordering::Greater),
                },
                TestCase {
                    // TC2: Input One has higher price but lower quantity -> Greater
                    input_one: Level::new(100, 10),
                    input_two: Level::new(10, 100),
                    expected: Some(Ordering::Greater),
                },
                TestCase {
                    // TC3: Input One has same price and higher quantity -> Greater
                    input_one: Level::new(10, 200),
                    input_two: Level::new(10, 100),
                    expected: Some(Ordering::Greater),
                },
                TestCase {
                    // TC4: Input One has same price and same quantity -> Equal
                    input_one: Level::new(100, 100),
                    input_two: Level::new(100, 100),
                    expected: Some(Ordering::Equal),
                },
                TestCase {
                    // TC5: Input One has same price but lower quantity -> Less
                    input_one: Level::new(10, 50),
                    input_two: Level::new(10, 100),
                    expected: Some(Ordering::Less),
                },
                TestCase {
                    // TC6: Input One has lower price but higher quantity -> Less
                    input_one: Level::new(10, 100),
                    input_two: Level::new(100, 50),
                    expected: Some(Ordering::Less),
                },
                TestCase {
                    // TC7: Input One has lower price and same quantity -> Less
                    input_one: Level::new(50, 100),
                    input_two: Level::new(100, 100),
                    expected: Some(Ordering::Less),
                },
                TestCase {
                    // TC8: Input One has lower price and lower quantity -> Less
                    input_one: Level::new(50, 50),
                    input_two: Level::new(100, 100),
                    expected: Some(Ordering::Less),
                },
            ];

            for (index, test) in tests.into_iter().enumerate() {
                let actual = test.input_one.partial_cmp(&test.input_two);
                match (actual, test.expected) {
                    (None, None) => {
                        // Test passed
                    }
                    (Some(actual), Some(expected)) => {
                        assert_eq!(actual, expected, "TC{} failed", index)
                    }
                    (actual, expected) => {
                        // Test failed
                        panic!("TC{index} failed because actual != expected. \nActual: {actual:?}\nExpected: {expected:?}\n");
                    }
                }
            }
        }

        #[test]
        fn test_eq_price() {
            struct TestCase {
                level: Level,
                input_level: Level,
                expected: bool,
            }

            let tests = vec![
                TestCase {
                    // TC0: Input Level has higher price
                    level: Level::new(50, 100),
                    input_level: Level::new(100, 100),
                    expected: false,
                },
                TestCase {
                    // TC1: Input Level an equal price
                    level: Level::new(50, 100),
                    input_level: Level::new(50, 100),
                    expected: true,
                },
                TestCase {
                    // TC2: Input Level has lower price
                    level: Level::new(50, 100),
                    input_level: Level::new(10, 100),
                    expected: false,
                },
            ];

            for (index, test) in tests.into_iter().enumerate() {
                let actual = test.level.eq_price(test.input_level.price);
                assert_eq!(actual, test.expected, "TC{} failed", index);
            }
        }
    }
}