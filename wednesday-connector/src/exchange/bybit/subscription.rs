use serde::{Deserialize, Serialize};
use tracing::debug;
use wednesday_model::{error::SocketError, identifiers::{Identifier, SubscriptionId}};

use crate::subscriber::validator::Validator;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct BybitSubscriptionResponse {
    pub success: bool,
    #[serde(default)]
    pub ret_msg: BybitReturnMessage,
    #[serde(default)]
    pub conn_id: String,
    #[serde(default)]
    pub req_id: String,
    #[serde(default)]
    pub op: String,
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub enum BybitReturnMessage {
    #[serde(alias = "", alias = "None")]
    Empty,
    #[serde(alias = "pong")]
    Pong,
    #[serde(alias = "subscribe")]
    Subscribe,
}

impl Default for BybitReturnMessage {
    fn default() -> Self {
        Self::Empty
    }
}

impl Identifier<Option<SubscriptionId>> for BybitSubscriptionResponse {
    fn id(&self) -> Option<SubscriptionId> {
        Some(SubscriptionId::from(self.req_id.clone()))
    }
}

impl Validator for BybitSubscriptionResponse {
    fn validate(self) -> Result<Self, SocketError>
    where
        Self: Sized,
    {
        println!("reg msg: {:?}", self.ret_msg);
        match self.ret_msg {
            BybitReturnMessage::Pong => Ok(self),
            BybitReturnMessage::Empty => {
                debug!("Received a response from the exchange: {:?}", self);
                if self.op == "subscribe" && self.success {
                    Ok(self)
                } else {
                    Err(SocketError::Subscribe("received failure subsciption response".to_owned()))
                }
            },
            BybitReturnMessage::Subscribe => {
                if self.success {
                    Ok(self)
                } else {
                    Err(SocketError::Subscribe("received failure subsciption response".to_owned()))
                }
            },
            _ => Err(SocketError::Subscribe("received unknown subsciption response".to_owned())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod de {
        use super::*;

        #[test]
        fn test_bybit_sub_response() {
            struct TestCase {
                input: &'static str,
                expected: Result<BybitSubscriptionResponse, SocketError>,
            }

            let cases = vec![
                TestCase {
                    // TC0: input response is Subscribed
                    input: r#"
                        {
                            "success": true,
                            "ret_msg": "subscribe",
                            "conn_id": "2324d924-aa4d-45b0-a858-7b8be29ab52b",
                            "req_id": "10001",
                            "op": "subscribe"
                        }
                    "#,
                    expected: Ok(BybitSubscriptionResponse {
                        success: true,
                        ret_msg: BybitReturnMessage::Subscribe,
                        conn_id: "2324d924-aa4d-45b0-a858-7b8be29ab52b".to_string(),
                        req_id: "10001".to_string(),
                        op: "subscribe".to_string(),
                    }),
                },
                TestCase {
                    // TC1: input response is failed subscription
                    input: r#"
                        {
                            "success": false,
                            "conn_id": "",
                            "op": ""
                        }
                    "#,
                    expected: Ok(BybitSubscriptionResponse {
                        success: false,
                        ret_msg: BybitReturnMessage::Empty,
                        conn_id: "".to_string(),
                        req_id: "".to_string(),
                        op: "".to_string(),
                    }),
                },
            ];

            for (index, test) in cases.into_iter().enumerate() {
                let actual = serde_json::from_str::<BybitSubscriptionResponse>(test.input);
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
    fn test_validate_bybit_sub_response() {
        struct TestCase {
            input_response: BybitSubscriptionResponse,
            is_valid: bool,
        }

        let cases = vec![
            TestCase {
                // TC0: input response is successful subscription
                input_response: BybitSubscriptionResponse {
                    success: true,
                    ret_msg: BybitReturnMessage::Empty,
                    conn_id: String::new(),
                    req_id: String::new(),
                    op: String::new(),
                },
                is_valid: false,
            },
            TestCase {
                // TC1: input response is successful subscription
                input_response: BybitSubscriptionResponse {
                    success: true,
                    ret_msg: BybitReturnMessage::Subscribe,
                    conn_id: String::new(),
                    req_id: String::new(),
                    op: String::new(),
                },
                is_valid: true,
            },
            TestCase {
                // TC2: input response is failed subscription
                input_response: BybitSubscriptionResponse {
                    success: false,
                    ret_msg: BybitReturnMessage::Empty,
                    conn_id: String::new(),
                    req_id: String::new(),
                    op: String::new(),
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
