use std::rc::Rc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio_stream::StreamExt;
use tracing::{debug, error, info};
use wednesday_model::{
    error::{DataError, SocketError},
    instruments::Instrument,
};

use crate::{
    exchange::connector::Connector,
    protocol::http::websocket::{WsClient, WsParser},
    stream::{parser::StreamParser, selector::StreamSelector},
};

use super::subscription::{Map, Subscription, SubscriptionKind};

pub trait Validator {
    fn validate(self) -> Result<Self, SocketError>
    where
        Self: Sized;
}

impl<Exchange, Kind> Validator for &Subscription<Exchange, Kind>
where
    Exchange: StreamSelector<Kind>,
    Kind: SubscriptionKind,
{
    fn validate(self) -> Result<Self, SocketError>
    where
        Self: Sized,
    {
        let exchange_id = Exchange::ID;

        if exchange_id.supports(self.instrument.kind) {
            Ok(self)
        } else {
            Err(SocketError::Unsupported {
                entity: exchange_id.as_str(),
                item: self.instrument.kind.to_string(),
            })
        }
    }
}

/// Defines how to validate that actioned market data
/// [`Subscription`](crate::subscription::Subscription)s were accepted by the exchange.
#[async_trait]
pub trait SubscriptionValidator {
    type Parser: StreamParser;

    async fn validate<Exchange, Kind>(instrument_map: Map<Instrument>, ws_client: &mut WsClient) -> Result<Map<Instrument>, SocketError>
    where
        Exchange: Connector + Send,
        Kind: SubscriptionKind + Send;
}

/// Standard [`SubscriptionValidator`] for [`WebSocket`]s suitable for most exchanges.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize, Serialize)]
pub struct WsSubscriptionValidator;

pub fn validate<Exchange, Kind>(subscriptions: &[Subscription<Exchange, Kind>]) -> Result<(), DataError>
where
    Exchange: StreamSelector<Kind>,
    Kind: SubscriptionKind,
{
    if subscriptions.is_empty() {
        return Err(DataError::Socket(SocketError::Subscribe(
            "StreamBuilder contains no Subscription to action".to_owned(),
        )));
    }

    subscriptions
        .iter()
        .map(|subscription| subscription.validate())
        .collect::<Result<Vec<_>, SocketError>>()?;

    Ok(())
}

///
#[async_trait]
impl SubscriptionValidator for WsSubscriptionValidator {
    type Parser = WsParser;

    async fn validate<Exchange, Kind>(instrument_map: Map<Instrument>, ws_client: &mut WsClient) -> Result<Map<Instrument>, SocketError>
    where
        Exchange: Connector + Send,
        Kind: SubscriptionKind + Send,
    {
        // Establish exchange specific subscription validation parameters
        let timeout = Exchange::subscription_timeout();
        let expected_responses = Exchange::expected_responses(&instrument_map);

        // Parameter to keep track of successful Subscription outcomes
        let mut success_responses = 0usize;

        loop {
            // Break if all Subscriptions were a success
            if success_responses == expected_responses {
                info!(exchange = %Exchange::ID, "validated exchange WebSocket subscriptions");
                break Ok(instrument_map);
            }

            tokio::select! {
                // If timeout reached, return SubscribeError
                _ = tokio::time::sleep(timeout) => {
                    break Err(SocketError::Subscribe(
                        format!("subscription validation timeout reached: {:?}", timeout)
                    ))
                },
                // Parse incoming messages and determine subscription outcomes
                message = ws_client.next() => {
                    let response = Rc::new(match message {
                        Some(response) => response,
                        None => break Err(SocketError::Subscribe("WebSocket stream terminated unexpectedly".to_string()))
                    });

                    match Self::Parser::parse::<Exchange::SubscriptionResponse>(response) {
                        Some(Ok(response)) => match response.validate() {
                            // Subscription success
                            Ok(response) => {
                                success_responses += 1;
                                debug!(
                                    exchange = %Exchange::ID,
                                    %success_responses,
                                    %expected_responses,
                                    payload = ?response,
                                    "received valid Ok subscription response",
                                );
                            }

                            // Subscription failure
                            Err(err) => {
                                error!(exchange = %Exchange::ID, %err, "received invalid subscription response");
                                break Err(err)
                            }
                        }
                        // NOTE: We need refactor this code.
                        Some(Err(SocketError::DeserializingJson { error, payload })) => {
                            // 이 부분은 꺼도 괜찮을 것 같음.
                            debug!(
                                exchange = %Exchange::ID,
                                ?error,
                                %success_responses,
                                %expected_responses,
                                %payload,
                                "failed to deserialize non SubResponse payload"
                            );
                            // Continue processing to handle potential late subscription messages
                            continue;
                        },
                        Some(Err(SocketError::Terminated(close_frame))) => {
                            break Err(SocketError::Subscribe(
                                format!("received WebSocket CloseFrame: {close_frame}")
                            ))
                        }
                        _ => {
                            // Pings, Pongs, Frames, etc.
                            continue
                        }
                    }
                }
            }
            info!(exchange = %Exchange::ID, "All subscriptions have been successfully registered.");
        }
    }
}
