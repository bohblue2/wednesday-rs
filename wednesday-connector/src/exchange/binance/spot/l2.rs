use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender;
use wednesday_model::{error::{DataError, SocketError}, identifiers::{Identifier, SubscriptionId}, instruments::Instrument, orderbook::OrderBook};

use crate::{exchange::binance::book::{BinanceLevel, BinanceOrderBookL2Snapshot}, protocol::http::websocket::WsMessage, transformer::updater::{InstrumentOrderBook, OrderBookUpdater}};

pub const REST_BOOK_L2_SNAPSHOT_URL_BINANCE_SPOT: &str = "https://api.binance.com/api/v3/depth";

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct BinanceSpotOrderBookL2Delta {
    #[serde(
        alias = "s",
        deserialize_with = "crate::exchange::binance::book::de_ob_l2_subscription_id"
    )]
    pub subscription_id: SubscriptionId,
    #[serde(alias = "U")]
    pub first_update_id: u64,
    #[serde(alias = "u")]
    pub last_update_id: u64,
    #[serde(alias = "b")]
    pub bids: Vec<BinanceLevel>,
    #[serde(alias = "a")]
    pub asks: Vec<BinanceLevel>,
}

impl Identifier<Option<SubscriptionId>> for BinanceSpotOrderBookL2Delta {
    fn id(&self) -> Option<SubscriptionId> {
        Some(self.subscription_id.clone())
    }
}

/// BinanceSpot: How To Manage A Local OrderBook Correctly
///
/// 1. Open a stream to wss://stream.binance.com:9443/ws/BTCUSDT@depth.
/// 2. Buffer the events you receive from the stream.
/// 3. Get a depth snapshot from <https://api.binance.com/api/v3/depth?symbol=BNBBTC&limit=1000>.
/// 4. -- *DIFFERENT FROM FUTURES* --
///    Drop any event where u is <= lastUpdateId in the snapshot.
/// 5. -- *DIFFERENT FROM FUTURES* --
///    The first processed event should have U <= lastUpdateId+1 AND u >= lastUpdateId+1.
/// 6. -- *DIFFERENT FROM FUTURES* --
///    While listening to the stream, each new event's U should be equal to the
///    previous event's u+1, otherwise initialize the process from step 3.
/// 7. The data in each event is the absolute quantity for a price level.
/// 8. If the quantity is 0, remove the price level.
///
/// Notes:
///  - Receiving an event that removes a price level that is not in your local order book can happen and is normal.
///  - Uppercase U => first_update_id
///  - Lowercase u => last_update_id,
///
/// See docs: <https://binance-docs.github.io/apidocs/spot/en/#how-to-manage-a-local-order-book-correctly>
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct BinanceSpotBookUpdater {
    pub updates_processed: u64,
    pub last_update_id: u64,
    pub prev_update_id: u64,
}

impl BinanceSpotBookUpdater {
    /// Construct a new BinanceSpot [`OrderBookUpdater`] using the provided last_update_id from
    /// a HTTP snapshot.
    pub fn new(last_update_id: u64) -> Self {
        Self {
            updates_processed: 0,
            last_update_id,
            prev_update_id: last_update_id,
        }
    }
    /// BinanceSpot: How To Manage A Local OrderBook Correctly: Step 5:
    /// "The first processed event should have U <= lastUpdateId+1 AND u >= lastUpdateId+1"
    ///
    /// See docs: <https://binance-docs.github.io/apidocs/spot/en/#how-to-manage-a-local-order-book-correctly>
    pub fn is_first_update(&self) -> bool {
        self.updates_processed == 0
    }

    /// BinanceSpot: How To Manage A Local OrderBook Correctly: Step 5:
    /// "The first processed event should have U <= lastUpdateId+1 AND u >= lastUpdateId+1"
    ///
    /// See docs: <https://binance-docs.github.io/apidocs/spot/en/#how-to-manage-a-local-order-book-correctly>
    pub fn validate_first_update(
        &self, 
        update: &BinanceSpotOrderBookL2Delta,
    ) -> Result<(), DataError> {
        let expected_next_id = self.last_update_id + 1;
        if update.first_update_id <= expected_next_id 
            && update.last_update_id >= expected_next_id 
        {
            Ok(())
        } else {
            Err(DataError::InvalidSequence {
                prev_last_update_id: self.last_update_id,
                first_update_id: update.first_update_id,
            })
        }

    }

    /// BinanceFuturesUsd: How To Manage A Local OrderBook Correctly: Step 6:
    /// "While listening to the stream, each new event's U should be equal to the
    ///  previous event's u+1, otherwise initialize the process from step 3."
    ///
    /// See docs: <https://binance-docs.github.io/apidocs/spot/en/#how-to-manage-a-local-order-book-correctly>
    pub fn validate_next_update(
        &self,
        update: &BinanceSpotOrderBookL2Delta,
    ) -> Result<(), DataError> {
        let expected_next_id = self.last_update_id + 1;
        if update.first_update_id == expected_next_id {
            Ok(())
        } else {
            Err(DataError::InvalidSequence {
                prev_last_update_id: self.last_update_id,
                first_update_id: update.first_update_id,
            })
        }
    }
}

#[async_trait]
impl OrderBookUpdater for BinanceSpotBookUpdater {
    type OrderBook = OrderBook;
    type Update = BinanceSpotOrderBookL2Delta;


    async fn init<Exchange, Kind>(
        _: UnboundedSender<WsMessage>,
        instrument: Instrument
    ) -> Result<InstrumentOrderBook<Self>, DataError>
    where
        Exchange: Send,
        Kind: Send,
    {
        let snapshot_url = format!(
            "{}?symbol={}{}&limit=100",
            REST_BOOK_L2_SNAPSHOT_URL_BINANCE_SPOT,
            instrument.base_currency.as_ref().to_uppercase(),
            instrument.quote_currency.as_ref().to_uppercase()
        );

        let snapshot = reqwest::get(&snapshot_url)
            .await
            .map_err(SocketError::Http)?
            .json::<BinanceOrderBookL2Snapshot>()
            .await
            .map_err(SocketError::Http)?;

        Ok(InstrumentOrderBook {
            instrument,
            updater: Self::new(snapshot.last_update_id),
            book: OrderBook::from(snapshot)
        })
    }

    fn update(
        &mut self,
        book: &mut Self::OrderBook,
        update: Self::Update,
    ) -> Result<Option<Self::OrderBook>, DataError> {
        // BinanceSpot: How To Manage A Local OrderBook Correctly
        // See Self's Rust Docs for more information on each numbered step
        // See docs: <https://binance-docs.github.io/apidocs/spot/en/#how-to-manage-a-local-order-book-correctly>

        // 4. Drop any event where u is <= lastUpdateId in the snapshot:
        if update.last_update_id <= self.last_update_id {
            return Ok(None);
        }

        if self.is_first_update() {
            self.validate_first_update(&update)?;
        } else {
            self.validate_next_update(&update)?;
        }

        book.last_update_ts = chrono::Utc::now();
        book.bids.upsert(update.bids);
        book.asks.upsert(update.asks);
        
        self.updates_processed += 1;
        self.prev_update_id = self.last_update_id;
        self.last_update_id = update.last_update_id;

        // NOTE:: 여기서 snapshot을 넘겨주는게 맞나 ? 그냥 Book 넘겨주는게 맞을지도.
        Ok(Some(book.snapshot()))
    }

}