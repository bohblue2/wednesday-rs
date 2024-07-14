use wednesday_model::error::SocketError;

use super::builder::HttpRequestBuilder;

/// [`RestRequest`](super::RestRequest) [`BuildStrategy`] that builds a non-authenticated Http request with no headers.
#[derive(Debug, Copy, Clone)]
pub struct PublicNoHeaders;

impl HttpRequestBuilder for PublicNoHeaders {
    fn build<Request>(&self, _: Request, builder: reqwest::RequestBuilder) -> Result<reqwest::Request, SocketError> {
        builder.build().map_err(SocketError::from)
    }
}
