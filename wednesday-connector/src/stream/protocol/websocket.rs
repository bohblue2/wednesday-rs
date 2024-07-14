use async_trait::async_trait;
use futures::StreamExt;
use futures::SinkExt;
use tokio::sync::mpsc;
use tracing::{info, error, debug};
use wednesday_model::error::DataError;
use wednesday_model::identifiers::ExchangeId;
use wednesday_model::identifiers::Identifier;

use crate::exchange::connector::Connector;
use crate::protocol::http::websocket::is_ws_disconnected;
use crate::protocol::http::websocket::PingInterval;
use crate::protocol::http::websocket::WsMessage;
use crate::protocol::http::websocket::WsParser;
use crate::protocol::http::websocket::WsSink;
use crate::protocol::http::websocket::WsStream;
use crate::stream::exchange::ExchangeStream;
use crate::stream::market::MarketStream;
use crate::subscriber::subscription::Subscription;
use crate::subscriber::subscription::SubscriptionKind;
use crate::subscriber::Subscriber;
use crate::transformer::ExchangeTransformer;

pub type ExchangeWsStream<Transformer> = ExchangeStream<WsParser, WsStream, Transformer>;
use std::fmt::Debug;

#[async_trait]
impl<Exchange, Kind, Transformer> MarketStream<Exchange, Kind> for ExchangeWsStream<Transformer>
where
    Exchange: Connector + Send + Sync,
    Kind: SubscriptionKind + Send + Sync,
    Transformer: ExchangeTransformer<Exchange, Kind> + Send, 
    Transformer::Pong: Debug,
    Kind::Event: Send,
{
    async fn init(subscriptions: &[Subscription<Exchange, Kind>]) -> Result<Self, DataError>
    where
        Subscription<Exchange, Kind>: Identifier<Exchange::Channel> + Identifier<Exchange::Market>
    {
        let (ws, map) = Exchange::Subscriber::subscribe(subscriptions).await?;
        let (ws_sink, ws_stream) = ws.split();
        let(ws_sink_tx, ws_sink_rx) = mpsc::unbounded_channel();

        tokio::spawn(distribute_messages_to_exchange(
            Exchange::ID,
            ws_sink,
            ws_sink_rx
        ));
        debug!(exchange=%Exchange::ID, "Spawned task to distribute messages to exchange with WebSocket sink and receiver");

        if let Some(ping_interval) = Exchange::ping_interval() {
            debug!(exchange=%Exchange::ID,
                "Spawned task to schedule pings to exchange with specified ping interval");
            tokio::spawn(schedule_pings_to_exchange(
                Exchange::ID,
                ws_sink_tx.clone(),
                ping_interval
            ));
        } else {
            debug!(exchange=%Exchange::ID, "No ping interval specified for exchange, skipping ping scheduling");
        }

        let transformer = Transformer::new(ws_sink_tx, map).await?;

        Ok(ExchangeWsStream::new(ws_stream, transformer))
    }
}

pub async fn distribute_messages_to_exchange(
    exchange: ExchangeId, 
    mut ws_sink: WsSink, 
    mut ws_sink_rx: mpsc::UnboundedReceiver<WsMessage>)
{
    while let Some(message) = ws_sink_rx.recv().await {
        if let Err(error) = ws_sink.send(message).await {
            if is_ws_disconnected(&error) {
                info!(%exchange, "WebSocket disconnected");
                break;
            }

            error!(%exchange, %error, "failed to send output message to the exchange via WsSink");
        }
    }
}

pub async fn schedule_pings_to_exchange(
    exchange_id: ExchangeId, 
    ws_sink_tx: mpsc::UnboundedSender<WsMessage>, 
    PingInterval { mut interval, ping}: PingInterval)
{
    loop {
        interval.tick().await;

        let payload = ping();
        debug!(%exchange_id, %payload, "sending custom application-level ping to exchange");

        if ws_sink_tx.send(payload).is_err() {
            break;
        }
    }
}