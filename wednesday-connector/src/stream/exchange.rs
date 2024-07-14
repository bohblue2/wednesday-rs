use std::fmt::Debug;
use std::pin;
use std::rc::Rc;
use std::task::Poll;
use std::{collections::VecDeque, marker::PhantomData};

use futures::Stream;
use pin_project::pin_project;
use tracing::debug;
use wednesday_model::error::SocketError;

use crate::transformer::Transformer;

use super::parser::StreamParser;

#[pin_project]
pub struct ExchangeStream<Protocol, InnerStream, StreamTransformer>
where
    Protocol: StreamParser, // NOTE: Protocol 이라는 이름이 적절한지 확인 필요
    InnerStream: Stream,
    StreamTransformer: Transformer,
{
    #[pin]
    pub stream: InnerStream,
    pub transformer: StreamTransformer,
    pub buffer: VecDeque<Result<StreamTransformer::Output, StreamTransformer::Error>>,
    pub protocol_marker: PhantomData<Protocol>,
}

impl<Protocol, InnerStream, StreamTransformer> Stream for ExchangeStream<Protocol, InnerStream, StreamTransformer>
where
    Protocol: StreamParser,
    InnerStream: Stream<Item = Result<Protocol::Message, Protocol::Error>> + Unpin,
    StreamTransformer: Transformer,
    StreamTransformer::Error: From<SocketError>,
    StreamTransformer::Pong: Debug,
{
    type Item = Result<StreamTransformer::Output, StreamTransformer::Error>;

    fn poll_next(mut self: pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Self::Item>> {
        loop {
            if let Some(output) = self.buffer.pop_front() {
                return Poll::Ready(Some(output));
            }

            let input = Rc::new(match self.as_mut().project().stream.poll_next(cx) {
                Poll::Ready(Some(input)) => input,
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Pending => return Poll::Pending,
            });

            // NOTE_0002: NOTE_0001 구현을 위해서 StreamTransforemr::Pong 를 추가하였음.
            let parsed_input = Protocol::parse::<StreamTransformer::Input>(Rc::clone(&input));

            let exchange_message = match parsed_input {
                // `StreamParser` successfully deserialized the `ExchagneMessage`
                Some(Ok(exchange_message)) => exchange_message,
                // if `StreamParser` return an Err pass it downstream
                Some(Err(err)) => {
                    // NOTE_0001: 원래 이 에러 처리는 Bybit 에서 PONG 메시지가 오는 것을 처리하기 위한 것임.
                    // KOSCOM 이나 다른 국내 증권사 WS API 를 사용할 때는 이 부분이 필요가 없을 수도 있음.
                    // ExchangeWsStream 이나 Protocol::parse layer 에서 처리하는게 맞는데 이쪽 레이어에서 처리하게함. 일단 보이는게 여기밖에 없었음.

                    let _message = match Protocol::parse::<StreamTransformer::Pong>(Rc::clone(&input)) {
                        Some(Ok(message)) => {
                            debug!("Received PONG message from exchange");
                            debug!(?message);
                            continue;
                        },
                        Some(Err(err)) => err,
                        None => {
                            debug!("StreamParser returned None!!");
                            continue;
                        },
                    };

                    return Poll::Ready(Some(Err(err.into())));
                },
                // if `StreamParser` returns None it's a safe-to-skip message
                None => {
                    debug!("StreamParser returned None!!");
                    continue;
                },
            };

            // Transform `ExchangeMessage` into `Transformer::OutputIter`
            // ie/ IntoIterator<Item = Result<Output, SocketError>>
            self.transformer.transform(exchange_message).into_iter().for_each(
                |output_result: Result<StreamTransformer::Output, StreamTransformer::Error>| {
                    self.buffer.push_back(output_result);
                },
            );
        }
    }
}

impl<Protocol, InnerStream, StreamTransformer> ExchangeStream<Protocol, InnerStream, StreamTransformer>
where
    Protocol: StreamParser,
    InnerStream: Stream,
    StreamTransformer: Transformer,
{
    pub fn new(stream: InnerStream, transformer: StreamTransformer) -> Self {
        ExchangeStream {
            stream,
            transformer,
            buffer: VecDeque::with_capacity(6),
            protocol_marker: PhantomData,
        }
    }
}
