// original file path: intergration-rs/src/protocol/mod.rs

use std::{rc::Rc, sync::Arc};

/// Contains `StreamParser` implementations for transforming communication protocol specific
/// messages into a generic output data structure.


use futures::Stream;
use serde::de::DeserializeOwned;
use wednesday_model::error::SocketError;


/// Contains useful `WebSocket` type aliases and a default `WebSocket` implementation of a
/// [`StreamParser`].
// pub mod websocket;

/// Contains HTTP client capable of executing signed & unsigned requests, as well as an associated
/// exchange oriented HTTP request.
// pub mod http;

/// `StreamParser`s are capable of parsing the input messages from a given stream protocol
/// (eg/ WebSocket, Financial Information eXchange (FIX), etc.) and deserialising into an `Output`.
pub trait StreamParser {
    type Stream: Stream;
    type Message;
    type Error;


    // NOTE_0002: NOTE_0001 구현을 위해서 StreamTransforemr::Pong 처리를 위해 input이 RC로 감싸짐.
    fn parse<Output>(
        input: Rc<Result<Self::Message, Self::Error>>,
    ) -> Option<Result<Output, SocketError>>
    where
        Output: DeserializeOwned;
}