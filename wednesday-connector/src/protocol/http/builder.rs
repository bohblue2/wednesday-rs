use wednesday_model::error::SocketError;

use super::rest::request::RestRequest;

/// [`RestRequest`] build strategy for the API being interacted with.
///
/// An API that requires authenticated [`RestRequest`]s will likely utilise the configurable
/// [`RequestSigner`](private::RequestSigner) to sign the requests before building.
///
/// An API that requires no authentication may just add mandatory `reqwest` headers to the
/// [`RestRequest`] before building.
pub trait HttpRequestBuilder {
    /// Use a [`RestRequest`] and [`reqwest::RequestBuilder`] to construct a [`reqwest::Request`]
    /// that is ready for executing.
    ///
    /// It is expected that any signing or performed during this method, or the addition of any
    /// `reqwest` headers.
    fn build<Request>(
        &self,
        request: Request,
        builder: reqwest::RequestBuilder,
    ) -> Result<reqwest::Request, SocketError>
    where
        Request: RestRequest;
}