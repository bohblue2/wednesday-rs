use async_trait::async_trait;
use futures::Stream;
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use tracing::{info, warn, error};
use wednesday_model::{error::DataError, events::MarketEvent, identifiers::Identifier};

use crate::{exchange::connector::Connector, subscriber::subscription::{Subscription, SubscriptionKind}};

use super::selector::StreamSelector;


#[async_trait]
pub trait MarketStream<Exchange, Kind>
where
    Self: Stream<Item = Result<MarketEvent<Kind::Event>, DataError>> + Send + Sized + Unpin,
    Exchange: Connector,
    Kind: SubscriptionKind,
{
    async fn init(subscriptions: &[Subscription<Exchange, Kind>]) -> Result<Self, DataError>
    where
        Subscription<Exchange, Kind>: Identifier<Exchange::Channel> + Identifier<Exchange::Market>;
}

pub const STARTING_RECONNECT_BACKOFF_MS: u64 = 1000;

pub async fn consume<Exchange, Kind>(
    subscriptions: Vec<Subscription<Exchange, Kind>>,
    exchange_tx: mpsc::UnboundedSender<MarketEvent<Kind::Event>>,
) -> DataError
where
    Exchange: StreamSelector<Kind>,
    Kind: SubscriptionKind,
    Subscription<Exchange, Kind>: Identifier<Exchange::Channel> + Identifier<Exchange::Market>,
{
    let exchange = Exchange::ID;

    info!(%exchange, ?subscriptions,
        policy = "retry connection with exponential backoff",
        "MarketStream consumer loop running",
    );

    let mut attempt: u32 = 0;
    let mut backoff_ms: u64 = STARTING_RECONNECT_BACKOFF_MS;

    loop {
        attempt += 1;
        backoff_ms *= 2;
        info!(%exchange, attempt,"attempting to initialize MarketStream");
        
        let mut stream = match Exchange::Stream::init(&subscriptions).await {
            Ok(stream) => {
                info!(%exchange, attempt, "successfully initialized MarketStream");
                
                backoff_ms = STARTING_RECONNECT_BACKOFF_MS;
                stream
            }
            Err(error) => {
                info!(%exchange, attempt, %error, "failed to initialize MarketStream");
                
                // If MarketStream Occurred DataError, retry connection after backoff_ms
                warn!(
                    %exchange,
                    attempt,
                    action = "attempting re-connection after backoff",
                    "exchange MarketStream Occurred DataError when Initialization.",
                );
                tokio::time::sleep(
                    std::time::Duration::from_millis(backoff_ms)
                ).await;

                if attempt == 5 {
                    return error;
                } else {
                    continue;
                }
            }
        };

        while let Some(event) = stream.next().await {
            match event {
                Ok(market_event) => {
                    let _ = exchange_tx
                        .send(market_event)
                        .map_err(|error| {
                            error!(
                                payload = ?error.0,
                                why = "receiver dropped",
                                "failed to send Event<MarketData> to Exchange Receiver"
                            );
                        }
                    );
                }
                Err(error) if error.is_terminal() => {
                    error!(%exchange, %error,
                        action = "re-initializing Stream",
                        "consumed DataError from MarketStream",
                    );
                    break;
                }
                Err(error) => {
                    warn!(%exchange, %error,
                        action = "skipping message",
                        "consumed DataError from MarketStream",
                    );
                    continue;
                }
            }
        }

        // If MarketStream ends unexpectedly, retry connection after backoff_ms
        warn!(
            %exchange,
            attempt,
            action = "attempting re-connection after backoff",
            "exchange MarketStream unexpectedly ended",
        );
        tokio::time::sleep(
            std::time::Duration::from_millis(backoff_ms)
        ).await;
    }
}
