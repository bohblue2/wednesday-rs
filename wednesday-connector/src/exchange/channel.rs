use tokio::sync::mpsc;

#[derive(Debug)]
pub struct ExchangeChannel<T> {
    pub tx: mpsc::UnboundedSender<T>,
    pub rx: mpsc::UnboundedReceiver<T>,
}

impl<T> ExchangeChannel<T> {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self { tx, rx }
    }
}

impl<T> Default for ExchangeChannel<T> {
    fn default() -> Self {
        Self::new()
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::{exchange::coinbase::Coinbase, subscription::trade::PublicTrades};
//     use barter_integration::model::instrument::kind::InstrumentKind;

//     #[test]
//     fn test_validate() {
//         struct TestCase {
//             input: Vec<Subscription<Coinbase, PublicTrades>>,
//             expected: Result<Vec<Subscription<Coinbase, PublicTrades>>, SocketError>,
//         }

//         let cases = vec![
//             TestCase {
//                 // TC0: Invalid Vec<Subscription> w/ empty vector
//                 input: vec![],
//                 expected: Err(SocketError::Subscribe("".to_string())),
//             },
//             TestCase {
//                 // TC1: Valid Vec<Subscription> w/ valid Coinbase Spot sub
//                 input: vec![Subscription::from((
//                     Coinbase,
//                     "base",
//                     "quote",
//                     InstrumentKind::Spot,
//                     PublicTrades,
//                 ))],
//                 expected: Ok(vec![Subscription::from((
//                     Coinbase,
//                     "base",
//                     "quote",
//                     InstrumentKind::Spot,
//                     PublicTrades,
//                 ))]),
//             },
//             TestCase {
//                 // TC2: Invalid StreamBuilder w/ invalid Coinbase FuturePerpetual sub
//                 input: vec![Subscription::from((
//                     Coinbase,
//                     "base",
//                     "quote",
//                     InstrumentKind::Perpetual,
//                     PublicTrades,
//                 ))],
//                 expected: Err(SocketError::Subscribe("".to_string())),
//             },
//         ];

//         for (index, test) in cases.into_iter().enumerate() {
//             let actual = validate(&test.input);

//             match (actual, test.expected) {
//                 (Ok(_), Ok(_)) => {
//                     // Test passed
//                 }
//                 (Err(_), Err(_)) => {
//                     // Test passed
//                 }
//                 (actual, expected) => {
//                     // Test failed
//                     panic!("TC{index} failed because actual != expected. \nActual: {actual:?}\nExpected: {expected:?}\n");
//                 }
//             }
//         }
//     }
// }
