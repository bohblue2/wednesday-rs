use reqwest::Error;
use thiserror::Error;

use crate::identifiers::SubscriptionId;

pub type Packet = u8;



#[derive(Error, Debug)]
pub enum SocketError {
    #[error("Sink Error")]
    Sink,

    #[error("Deserializing JSON error: {error} for payload: {payload}")]
    DeserializingJson { 
        error: String, 
        payload: String 
    },

    #[error("Deserializing Binary error: {error} for payload: {payload:?}")]
    DeserializingBinary { 
        error: serde_json::Error,
        payload: Vec<u8> 
    },

    #[error("Error unwrapping value: {0}")]
    UnwrapError(String),

    #[error("Serializing JSON error: {error} for payload: {payload}")]
    SerializingJson { error: String, payload: String },

    #[error("SerDe Query String serialisation error: {0}")]
    QueryParams(#[from] serde_qs::Error),

    #[error("error parsing Url: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("error subscribing to resources over the socket: {0}")]
    Subscribe(String),

    #[error("ExchangeStream terminated with closing frame: {0}")]
    Terminated(String),

    #[error("{entity} does not support: {item}")]
    Unsupported { entity: &'static str, item: String },

    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("WebSocket Connection error: {0}")]
    WebSocketConnection(String),

    #[error("HTTP error: {0}")]
    Http(reqwest::Error),

    #[error("HTTP request timed out")]
    HttpTimeout(reqwest::Error),

    /// REST http response error
    #[error("HTTP response (status={0}) error: {1}")]
    HttpResponse(reqwest::StatusCode, String),

    #[error("consumed unidentifiable message: {0}")]
    Unidentifiable(SubscriptionId),

    #[error("consumed error message from exchange: {0}")]
    Exchange(String),
}

impl From<reqwest::Error> for SocketError {
    fn from(error: Error) -> Self {
        match error {
            error if error.is_timeout() => SocketError::HttpTimeout(error),
            error => SocketError::Http(error),
        }
    }
}

#[derive(Error, Debug, Clone)]
pub enum ParserError {
    #[error("fail to parse packet: data({0})")] 
    PacketParse(Packet),

    #[error("invalid aggressor side: {0}")]
    AggressorSideParse(String),
}

#[derive(Error, Debug)]
pub enum DataError {
    #[error("Socket error: {0}")]
    Socket(#[from] SocketError),

    #[error(
        "InvalidSequence: first_update_id {first_update_id} does not follow on from the \
        prev_last_update_id {prev_last_update_id}"
    )]
    InvalidSequence {
        prev_last_update_id: u64,
        first_update_id: u64,
    }
}

impl DataError {
    /// Determine if an error requires a [`MarketStream`](super::MarketStream) to re-initialise.
    pub fn is_terminal(&self) -> bool {
        match self {
            DataError::InvalidSequence { .. } => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_error_packet_parse() {
        let packet = 0x01;
        let parser_error = ParserError::PacketParse(packet);
        assert!(matches!(parser_error, ParserError::PacketParse(_)));
    }

    #[test]
    fn test_parser_error_aggressor_side_parse() {
        let aggressor_side = "invalid";
        let parser_error = ParserError::AggressorSideParse(aggressor_side.to_string());
        assert!(matches!(parser_error, ParserError::AggressorSideParse(_)));
    }

    #[test]
    fn test_data_error_invalid_sequence() {
        let prev_last_update_id = 100;
        let first_update_id = 200;
        let data_error = DataError::InvalidSequence {
            prev_last_update_id,
            first_update_id,
        };
        assert!(matches!(data_error, DataError::InvalidSequence { .. }));
    }
}
