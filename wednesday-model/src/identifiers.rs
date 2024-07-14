use std::{
    borrow::Cow,
    fmt::{self, Debug, Display},
};

use serde::{Deserialize, Deserializer, Serialize};

use crate::instruments::{Instrument, InstrumentKind, Symbol};

pub trait Identifier<T> {
    fn id(&self) -> T;
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Market {
    pub exchange: Exchange,
    #[serde(flatten)]
    pub instrument: Instrument,
}

impl<E, I> From<(E, I)> for Market
where
    E: Into<Exchange>,
    I: Into<Instrument>,
{
    fn from((exchange, instrument): (E, I)) -> Self {
        Self::new(exchange, instrument)
    }
}

impl<E, S> From<(E, S, S, InstrumentKind)> for Market
where
    E: Into<Exchange>,
    S: Into<Symbol>,
{
    fn from((exchange, base_currency, quote_currency, kind): (E, S, S, InstrumentKind)) -> Self {
        Self::new(exchange, (base_currency, quote_currency, kind))
    }
}

impl Market {
    pub fn new<E, I>(exchange: E, instrument: I) -> Self
    where
        E: Into<Exchange>,
        I: Into<Instrument>,
    {
        Self {
            exchange: exchange.into(),
            instrument: instrument.into(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct MarketId(pub String);

impl<'a, M> From<M> for MarketId
where
    M: Into<&'a Market>,
{
    fn from(market: M) -> Self {
        let market = market.into();
        Self::new(&market.exchange, &market.instrument)
    }
}

impl Debug for MarketId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MarketId({})", self.0)
    }
}

impl Display for MarketId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<'de> Deserialize<'de> for MarketId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(MarketId)
    }
}

impl MarketId {
    pub fn new(exchange: &Exchange, instrument: &Instrument) -> Self {
        Self(
            format!(
                "{}:{}/{}-{}",
                exchange, instrument.base_currency, instrument.quote_currency, instrument.kind
            )
            .to_lowercase(),
        )
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Exchange(Cow<'static, str>);

impl<E> From<E> for Exchange
where
    E: Into<Cow<'static, str>>,
{
    fn from(exchange: E) -> Self {
        Self(exchange.into())
    }
}

impl Debug for Exchange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Exchange({})", self.0)
    }
}

impl Display for Exchange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SubscriptionId(pub String);

impl<S> From<S> for SubscriptionId
where
    S: Into<String>,
{
    fn from(subscription_id: S) -> Self {
        Self(subscription_id.into())
    }
}

impl Debug for SubscriptionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SubscriptionId({})", self.0)
    }
}

impl Display for SubscriptionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for SubscriptionId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
#[serde(rename = "exchange", rename_all = "snake_case")]
pub enum ExchangeId {
    BinanceFuturesUsd,
    BinanceSpot,
    Bitfinex,
    Bitmex,
    BybitSpot,
    BybitPerpetualsUsd,
    Coinbase,
    GateioSpot,
    GateioFuturesUsd,
    GateioFuturesBtc,
    GateioPerpetualsBtc,
    GateioPerpetualsUsd,
    GateioOptions,
    Kraken,
    Okx,
}

impl From<ExchangeId> for Exchange {
    fn from(exchange_id: ExchangeId) -> Self {
        Exchange::from(exchange_id.as_str())
    }
}

impl Display for ExchangeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl ExchangeId {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExchangeId::BinanceSpot => "binance_spot",
            ExchangeId::BinanceFuturesUsd => "binance_futures_usd",
            ExchangeId::Bitfinex => "bitfinex",
            ExchangeId::Bitmex => "bitmex",
            ExchangeId::BybitSpot => "bybit_spot",
            ExchangeId::BybitPerpetualsUsd => "bybit_perpetuals_usd",
            ExchangeId::Coinbase => "coinbase",
            ExchangeId::GateioSpot => "gateio_spot",
            ExchangeId::GateioFuturesUsd => "gateio_futures_usd",
            ExchangeId::GateioFuturesBtc => "gateio_futures_btc",
            ExchangeId::GateioPerpetualsUsd => "gateio_perpetuals_usd",
            ExchangeId::GateioPerpetualsBtc => "gateio_perpetuals_btc",
            ExchangeId::GateioOptions => "gateio_options",
            ExchangeId::Kraken => "kraken",
            ExchangeId::Okx => "okx",
        }
    }

    #[allow(clippy::match_like_matches_macro)]
    pub fn supports(&self, instrument_kind: InstrumentKind) -> bool {
        use ExchangeId::*;
        use InstrumentKind::*;

        match (self, instrument_kind) {
            // Spot
            (BinanceFuturesUsd | Bitmex | BybitPerpetualsUsd | GateioPerpetualsUsd | GateioPerpetualsBtc, CryptoSpot) => false,
            (_, CryptoSpot) => true,

            // Future
            (GateioFuturesUsd | GateioFuturesBtc | Okx, CryptoFuture(_)) => true,
            (_, CryptoFuture(_)) => false,

            // Future Perpetual Swaps
            (BinanceFuturesUsd | Bitmex | Okx | BybitPerpetualsUsd | GateioPerpetualsUsd | GateioPerpetualsBtc, CryptoPerpetual) => true,

            // Unimplemented
            (_, CryptoPerpetual) => false,
            (BinanceFuturesUsd, Stock) => todo!(),
            (BinanceSpot, Stock) => todo!(),
            (Bitfinex, Stock) => todo!(),
            (Bitmex, Stock) => todo!(),
            (BybitSpot, Stock) => todo!(),
            (BybitPerpetualsUsd, Stock) => todo!(),
            (Coinbase, Stock) => todo!(),
            (GateioSpot, Stock) => todo!(),
            (GateioFuturesUsd, Stock) => todo!(),
            (GateioFuturesBtc, Stock) => todo!(),
            (GateioPerpetualsBtc, Stock) => todo!(),
            (GateioPerpetualsUsd, Stock) => todo!(),
            (GateioOptions, Stock) => todo!(),
            (Kraken, Stock) => todo!(),
            (Okx, Stock) => todo!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::instruments::{Instrument, InstrumentKind, Symbol};

    use super::*;
    use serde::de::Error;

    #[test]
    fn test_market_from_tuple() {
        let exchange = Exchange::from("NASDAQ");
        let instrument = Instrument::from((Symbol::from("AAPL"), Symbol::from("USD"), InstrumentKind::Stock));
        let market: Market = (exchange.clone(), instrument.clone()).into();
        assert_eq!(market.exchange, exchange);
        assert_eq!(market.instrument, instrument);
    }

    #[test]
    fn test_market_from_tuple_with_currencies() {
        let exchange = Exchange::from("NYSE");
        let base_currency = Symbol::new("BTC");
        let quote_currency = Symbol::new("USD");
        let kind = InstrumentKind::CryptoSpot;
        let instrument = Instrument::from((base_currency, quote_currency, kind));

        let market: Market = (exchange.clone(), instrument.clone()).into();
        assert_eq!(market.exchange, exchange);
        assert_eq!(market.instrument, instrument);
    }

    #[test]
    fn test_market_id_from_market() {
        let exchange = Exchange::from("NYSE");
        let instrument = Instrument::from((Symbol::new("AAPL"), Symbol::new("USD"), InstrumentKind::Stock));
        let market = Market::new(exchange.clone(), instrument.clone());
        let market_id: MarketId = MarketId::from(&market);
        assert_eq!(market_id, MarketId::new(&exchange, &instrument));
    }

    #[test]
    fn test_de_market() {
        struct TestCase {
            input: &'static str,
            expected: Result<Market, serde_json::Error>,
        }

        let cases = vec![
            TestCase {
                // TC0: Valid Binance btc_usd Spot Market
                input: r##"{ "exchange": "binance", "base_currency": "btc", "quote_currency": "usd", "instrument_kind": "crypto_spot" }"##,
                expected: Ok(Market {
                    exchange: Exchange::from("binance"),
                    instrument: Instrument::from(("btc", "usd", InstrumentKind::CryptoSpot)),
                }),
            },
            TestCase {
                // TC1: Valid Ftx btc_usd FuturePerpetual Market
                input: r##"{ "exchange": "ftx_old", "base_currency": "btc", "quote_currency": "usd", "instrument_kind": "crypto_perpetual" }"##,
                expected: Ok(Market {
                    exchange: Exchange::from("ftx_old"),
                    instrument: Instrument::from(("btc", "usd", InstrumentKind::CryptoPerpetual)),
                }),
            },
            TestCase {
                // TC3: Invalid Market w/ numeric exchange
                input: r##"{ "exchange": 100, "base_currency": "btc", "quote_currency": "usd", "instrument_kind": "crypto_perpetual" }"##,
                expected: Err(serde_json::Error::custom("")),
            },
        ];

        for (index, test) in cases.into_iter().enumerate() {
            let actual = serde_json::from_str::<Market>(test.input);

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
