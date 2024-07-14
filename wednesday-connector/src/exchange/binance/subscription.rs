use serde::{Deserialize, Serialize};
use wednesday_model::error::SocketError;

use crate::subscriber::validator::Validator;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Deserialize, Serialize)]
pub struct BinanceSubscriptionResponse {
    result: Option<Vec<String>>,
    id: u32,
}

impl Validator for BinanceSubscriptionResponse {
    fn validate(self) -> Result<Self, SocketError>
    where
        Self: Sized,
    {
        if self.result.is_none() {
            Ok(self)
        } else {
            Err(SocketError::Subscribe("receive failure subscription response: ".to_owned()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod de {
        use super::*;

        #[test]
        fn test_binance_sub_response() {
            struct TestCase {
                input: &'static str,
                expected: Result<BinanceSubscriptionResponse, SocketError>,
            }

            let cases = vec![
                TestCase {
                    // TC0: input response is Subscribed
                    input: r#"{"id":1,"result":null}"#,
                    expected: Ok(BinanceSubscriptionResponse { result: None, id: 1 }),
                },
                TestCase {
                    // TC1: input response is failed subscription
                    input: r#"{"result": [], "id": 1}"#,
                    expected: Ok(BinanceSubscriptionResponse {
                        result: Some(vec![]),
                        id: 1,
                    }),
                },
            ];

            for (index, test) in cases.into_iter().enumerate() {
                let actual = serde_json::from_str::<BinanceSubscriptionResponse>(test.input);
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

    #[test]
    fn test_validate_binance_sub_response() {
        struct TestCase {
            input_response: BinanceSubscriptionResponse,
            is_valid: bool,
        }

        let cases = vec![
            TestCase {
                // TC0: input response is successful subscription
                input_response: BinanceSubscriptionResponse { result: None, id: 1 },
                is_valid: true,
            },
            TestCase {
                // TC1: input response is failed subscription
                input_response: BinanceSubscriptionResponse {
                    result: Some(vec![]),
                    id: 1,
                },
                is_valid: false,
            },
        ];

        for (index, test) in cases.into_iter().enumerate() {
            let actual = test.input_response.validate().is_ok();
            assert_eq!(actual, test.is_valid, "TestCase {} failed", index);
        }
    }
}
