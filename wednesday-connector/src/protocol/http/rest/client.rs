use bytes::Bytes;
use chrono::Utc;
use tracing::debug;
use wednesday_model::error::SocketError;
use std::borrow::Cow;

use crate::protocol::{http::{builder::HttpRequestBuilder, parser::HttpParser}, metric::{Field, Metric, Tag}};

use super::request::RestRequest;
use crate::protocol::http::rest::request::AsUrlParams;


/// Configurable REST client capable of executing signed [`RestRequest`]s. Use this when
/// integrating APIs that require Http in order to interact with resources. Each API will require
/// a specific combination of [`Signer`](super::super::private::Signer), [`Mac`](hmac::Mac),
/// signature [`Encoder`](super::super::private::encoder::Encoder), and
/// [`HttpParser`].
#[derive(Debug)]
pub struct RestClient<'a, Strategy, Parser> {
    /// HTTP [`reqwest::Client`] for executing signed [`reqwest::Request`]s.
    pub http_client: reqwest::Client,

    /// Base Url of the API being interacted with.
    pub base_url: Cow<'a, str>,

    /// [`RestRequest`] build strategy for the API being interacted with that implements
    /// [`BuildStrategy`].
    ///
    /// An authenticated [`RestClient`] will utilise API specific
    /// [`Signer`](super::super::private::Signer) logic, a hashable [`Mac`](hmac::Mac), and a
    /// signature [`Encoder`](super::super::private::encoder::Encoder). Where as a non authorised
    /// [`RestRequest`] may add any mandatory `reqwest` headers that are required.
    pub strategy: Strategy,

    /// [`HttpParser`] that deserialises [`RestRequest::Response`]s, and upon failure parses
    /// API errors returned from the server.
    pub parser: Parser,
}

impl<'a, Strategy, Parser> RestClient<'a, Strategy, Parser>
where
    Strategy: HttpRequestBuilder,
    Parser: HttpParser,
{
    /// Execute the provided [`RestRequest`].
    pub async fn execute<Request>(
        &self,
        request: Request,
    ) -> Result<(Request::Response, Metric), Parser::OutputError>
    where
        Request: RestRequest,
    {
        // Use provided Request to construct a signed reqwest::Request
        let request = self.build(request)?;

        // Measure request execution
        let (status, payload, latency) = self.measured_execution::<Request>(request).await?;

        // Log the response payload as JSON for debugging
        if let Ok(json_payload) = serde_json::from_slice::<serde_json::Value>(&payload) {
            debug!("Response Payload: {}", json_payload);
        } else if let Ok(string_payload) = std::str::from_utf8(&payload) {
            debug!("Response Payload: {}", string_payload);
            if string_payload.is_empty() {
                debug!("Response payload is empty: Probably a wrong endpoint.");
            }
        } else {
            debug!("Failed to parse response payload as JSON or String");
        }

        // Attempt to parse API Success or Error response
        self.parser
            .parse::<Request::Response>(status, &payload)
            .map(|response| (response, latency))
    }

    /// Use the provided [`RestRequest`] to construct a signed Http [`reqwest::Request`].
    pub fn build<Request>(&self, request: Request) -> Result<reqwest::Request, SocketError>
    where
        Request: RestRequest,
    {
        // Construct url
        let url = format!("{}{}", self.base_url, request.path());

        // Construct RequestBuilder with method & url
        let mut builder = self
            .http_client
            .request(Request::method(), url)
            .timeout(Request::timeout());


            
        // // Add optional query parameters
        // if let Some(query_params) = request.query_params() {
        //     builder = builder.query(query_params);
        // }

        // // Add optional Body
        // if let Some(body) = request.body() {
        //     // builder = builder.json(body);
        //     let string_ = body.to_url_params();
        //     debug!("Body: {}", string_);
        //     builder = builder.body(body.to_url_params());
        // }

        // Use RequestBuilder (public or private strategy) to build reqwest::Request
        self.strategy.build(request, builder)

    }

    /// Execute the built [`reqwest::Request`] using the [`reqwest::Client`].
    ///
    /// Measures and returns the Http request round trip duration.
    pub async fn measured_execution<Request>(
        &self,
        request: reqwest::Request,
    ) -> Result<(reqwest::StatusCode, Bytes, Metric), SocketError>
    where
        Request: RestRequest,
    {
        // Construct Http request duration Metric
        let mut latency = Metric {
            name: "http_request_duration",
            time: Utc::now().timestamp_millis() as u64,
            tags: vec![
                Tag::new("http_method", Request::method().as_str()),
                Tag::new("base_url", self.base_url.as_ref()),
                Tag::new("path", request.url().path()),
            ],
            fields: Vec::with_capacity(1),
        };

        // Measure the HTTP request round trip duration
        let start = std::time::Instant::now();
        let response = self.http_client.execute(request).await?;
        let duration = start.elapsed().as_millis() as u64;

        // Update Metric with response status and request duration
        latency
            .tags
            .push(Tag::new("status_code", response.status().as_str()));
        latency.fields.push(Field::new("duration", duration));

        // Extract Status Code & reqwest::Response Bytes
        let status_code = response.status();
        let payload = response.bytes().await?;

        Ok((status_code, payload, latency))
    }
}

impl<'a, Strategy, Parser> RestClient<'a, Strategy, Parser> {
    /// Construct a new [`Self`] using the provided configuration.
    pub fn new<Url: Into<Cow<'a, str>>>(base_url: Url, strategy: Strategy, parser: Parser) -> Self {
        Self {
            http_client: reqwest::Client::new(),
            base_url: base_url.into(),
            strategy,
            parser,
        }
    }
}