use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{fmt::Debug, rc::Rc};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{
        client::IntoClientRequest,
        error::ProtocolError,
        protocol::{frame::Frame, CloseFrame},
    },
    MaybeTlsStream,
};
use tracing::debug;
use wednesday_model::error::SocketError;

use crate::stream::parser::StreamParser;

pub type WsClient = tokio_tungstenite::WebSocketStream<MaybeTlsStream<TcpStream>>;
pub type WsMessage = tokio_tungstenite::tungstenite::Message;
pub type WsError = tokio_tungstenite::tungstenite::Error;
pub type WsSink = futures::stream::SplitSink<WsClient, WsMessage>;
pub type WsStream = futures::stream::SplitStream<WsClient>;

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WsParser;

impl StreamParser for WsParser {
    type Stream = WsClient;
    type Message = WsMessage;
    type Error = WsError;

    fn parse<Output>(input: Rc<Result<Self::Message, Self::Error>>) -> Option<Result<Output, SocketError>>
    where
        Output: DeserializeOwned,
    {
        match &*input {
            Ok(ws_message) => match ws_message {
                WsMessage::Text(text) => process_text(text),
                WsMessage::Binary(binary) => process_binary(binary),
                WsMessage::Ping(ping) => process_ping(ping),
                WsMessage::Pong(pong) => process_pong(pong),
                WsMessage::Close(close_frame) => process_close_frame(close_frame),
                WsMessage::Frame(frame) => process_frame(frame),
            },
            // NOTE_0002: NOTE_0001 구현을 위해서 StreamTransforemr::Pong 를 추가의 사이드 이펙트로 SocketError::Websocket 에서.
            // SocketError:WebSocketConnection(String) 으로 에러 처리가 변경됨.
            // 아 뭔가 맘에 들지 않은데 일단 PONG 처리 방법은 저렇게 해야되서 그냥 둠(SocketError:Websocket 이걸로 처리하는게 더 맞음).
            Err(ws_err) => Some(Err(SocketError::WebSocketConnection(ws_err.to_string()))),
        }
    }
}

/// Process a payload of `String` by deserialising into an `ExchangeMessage`.
pub fn process_text<ExchangeMessage>(payload: &String) -> Option<Result<ExchangeMessage, SocketError>>
where
    ExchangeMessage: DeserializeOwned,
{
    Some(serde_json::from_str::<ExchangeMessage>(&payload).map_err(|error| {
        debug!(
            ?error,
            ?payload,
            action = "returning Some(Err(err))",
            "failed to deserialize WebSocket Message into domain specific Message"
        );
        // 에러는 더 상위 레이어에서 처리, 일단 에러 로깅은 디버깅으로 함.
        SocketError::DeserializingJson {
            error: error.to_string(),
            payload: payload.clone(),
        }
    }))
}

/// Process a payload of `Vec<u8>` bytes by deserialising into an `ExchangeMessage`.
pub fn process_binary<ExchangeMessage>(payload: &Vec<u8>) -> Option<Result<ExchangeMessage, SocketError>>
where
    ExchangeMessage: DeserializeOwned,
{
    Some(serde_json::from_slice::<ExchangeMessage>(&payload).map_err(|error| {
        debug!(
            ?error,
            ?payload,
            action = "returning Some(Err(err))",
            "failed to deserialize WebSocket Message into domain specific Message"
        );
        SocketError::DeserializingBinary {
            error: error,
            payload: payload.to_vec(),
        }
    }))
}

/// Basic process for a [`WebSocket`] ping message. Logs the payload at `trace` level.
pub fn process_ping<ExchangeMessage>(ping: &Vec<u8>) -> Option<Result<ExchangeMessage, SocketError>> {
    debug!(payload = ?ping, "received Ping WebSocket message");
    None
}

/// Basic process for a [`WebSocket`] pong message. Logs the payload at `trace` level.
pub fn process_pong<ExchangeMessage>(pong: &Vec<u8>) -> Option<Result<ExchangeMessage, SocketError>> {
    debug!(payload = ?pong, "received Pong WebSocket message");
    None
}

/// Basic process for a [`WebSocket`] CloseFrame message. Logs the payload at `trace` level.
pub fn process_close_frame<ExchangeMessage>(close_frame: &Option<CloseFrame<'_>>) -> Option<Result<ExchangeMessage, SocketError>> {
    let close_frame = format!("{:?}", close_frame);
    debug!(payload = %close_frame, "received CloseFrame WebSocket message");
    Some(Err(SocketError::Terminated(close_frame)))
}

/// Basic process for a [`WebSocket`] Frame message. Logs the payload at `trace` level.
pub fn process_frame<ExchangeMessage>(frame: &Frame) -> Option<Result<ExchangeMessage, SocketError>> {
    let frame = format!("{:?}", frame);
    debug!(payload = %frame, "received unexpected Frame WebSocket message");
    None
}

/// Connect asynchronously to a [`WebSocket`] server.
pub async fn connect<Request>(request: Request) -> Result<WsClient, SocketError>
where
    Request: IntoClientRequest + Unpin + Debug,
{
    debug!(?request, "attempting to establish WebSocket connection");
    connect_async(request)
        .await
        .map(|(websocket, _)| websocket)
        .map_err(SocketError::WebSocket)
}

/// Determine whether a [`WsError`] indicates the [`WebSocket`] has disconnected.
pub fn is_ws_disconnected(error: &WsError) -> bool {
    matches!(
        error,
        WsError::ConnectionClosed | WsError::AlreadyClosed | WsError::Io(_) | WsError::Protocol(ProtocolError::SendAfterClosing)
    )
}

#[derive(Debug)]
pub struct PingInterval {
    pub interval: tokio::time::Interval,
    pub ping: fn() -> WsMessage,
}
