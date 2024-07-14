use std::fmt::{self, Display, Debug};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InstrumentKind {
    Stock,
    CryptoSpot,
    CryptoFuture(FuturesContract),
    CryptoPerpetual,
}

impl Default for InstrumentKind {
    fn default() -> Self {
        Self::CryptoSpot
    }
}

impl Display for InstrumentKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                InstrumentKind::Stock => "stock".to_string(),
                InstrumentKind::CryptoSpot => "spot".to_string(),
                InstrumentKind::CryptoFuture(futures_contract) => 
                    format!("future_{}-UTC", futures_contract.expiration.date_naive()),
                InstrumentKind::CryptoPerpetual => "perpetual".to_string(),
            }
        )
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Deserialize, Serialize)]
pub struct FuturesContract {
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub expiration: DateTime<Utc>,
}


#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct Symbol(String);

impl Debug for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for Symbol {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for Symbol {
    fn deserialize<D>(deserializer: D) -> Result<Symbol, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer).map(Symbol::new)
    }
}

impl<S> From<S> for Symbol
where 
    S: Into<String>,
{
    fn from(input: S) -> Self {
        Self::new(input)
    }
}

impl Symbol {
    pub fn new<S>(symbol: S) -> Self
    where
        S: Into<String>,
    {
        Symbol(symbol.into().to_lowercase())
    }
}

// 일단 크립토만
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct Instrument {
    pub base_currency: Symbol,
    pub quote_currency: Symbol,
    #[serde(rename = "instrument_kind")]
    pub kind: InstrumentKind,
}

impl Display for Instrument {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}/{}, {})", self.base_currency, self.quote_currency, self.kind)
    }
}

impl<S> From<(S, S, InstrumentKind)> for Instrument
where
    S: Into<Symbol>,
{
    fn from(
        (base_currency, quote_curreny, kind): (S, S, InstrumentKind)
    ) -> Self {
        Instrument {
            base_currency: base_currency.into(),
            quote_currency: quote_curreny.into(),
            kind: kind,
        }
    }
}

impl Instrument {
    pub fn new<S>(base_currency: S, quote_currency: S, kind: InstrumentKind) -> Self
    where
        S: Into<Symbol>,
    {
        Instrument {
            base_currency: base_currency.into(),
            quote_currency: quote_currency.into(),
            kind,
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;
    use crate::instruments::FuturesContract;

    #[test]
    fn test_de_instrument() {
        struct TestCase {
            input: &'static str,
            expected: Result<Instrument, serde_json::Error>
        }

        let cases = vec![
            TestCase {
                input: r#"{
                    "base_currency": "btc", 
                    "quote_currency": "usd", 
                    "instrument_kind": "crypto_spot" 
                }"#,
                expected: Ok(Instrument::from(("btc", "usd", InstrumentKind::CryptoSpot))),
            },
            TestCase {
                // TC1: Valid Future
                input: r#"{
                    "base_currency": "btc",
                    "quote_currency": "usd",
                    "instrument_kind": {"crypto_future": {"expiration": 1703980800000}}
                }"#,
                expected: Ok(Instrument::new(
                    "btc",
                    "usd",
                    InstrumentKind::CryptoFuture(FuturesContract {
                        expiration: Utc.timestamp_millis_opt(1703980800000).unwrap(),
                    }),
                )),
            },
            TestCase {
                // TC2: Valid FuturePerpetual
                input: r#"{
                    "base_currency": "btc", 
                    "quote_currency": "usd", 
                    "instrument_kind": "crypto_perpetual"
                }"#,
                expected: Ok(Instrument::from(("btc", "usd", InstrumentKind::CryptoPerpetual))),
            }
        ];

        for (i, test_case) in cases.iter().enumerate() {
            let actual = serde_json::from_str::<Instrument>(test_case.input);
            match (&actual, &test_case.expected) {
                (Ok(act_inst), Ok(exp_inst)) => {
                    assert_eq!(act_inst, exp_inst, "Test case {} failed", i);
                }
                (Err(act_err), Err(exp_err)) => {
                    assert_eq!(act_err.to_string(), exp_err.to_string(), "Test case {} failed", i);
                }
                _ => {
                    panic!("Test case {} failed: expected {:?}, got {:?}", i, test_case.expected, actual);
                }
            }
        }
    }
}