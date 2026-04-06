use http::Uri;
use tracing::trace;

use crate::error::Error;
use crate::{WebFingerRequest, WebFingerResponse};

struct EmptyBody;

impl From<EmptyBody> for reqwest::Body {
    fn from(_: EmptyBody) -> reqwest::Body {
        reqwest::Body::default()
    }
}

impl TryFrom<&WebFingerRequest> for http::Request<EmptyBody> {
    type Error = http::Error;

    fn try_from(query: &WebFingerRequest) -> Result<http::Request<EmptyBody>, http::Error> {
        let uri = Uri::try_from(query)?;
        http::Request::builder()
            .method("GET")
            .uri(uri)
            .body(EmptyBody)
    }
}

impl TryFrom<&WebFingerRequest> for reqwest::Request {
    type Error = crate::Error;

    fn try_from(query: &WebFingerRequest) -> Result<reqwest::Request, crate::Error> {
        let request = http::Request::try_from(query)?;
        let request = reqwest::Request::try_from(request)?;
        Ok(request)
    }
}

impl WebFingerRequest {
    /// Executes the WebFinger request with a fresh [`reqwest::Client`].
    ///
    /// This is the shortest path from a [`WebFingerRequest`] to a parsed [`WebFingerResponse`].
    /// The method:
    ///
    /// 1. Converts the WebFinger query into a `GET` [`reqwest::Request`].
    /// 1. Creates a new default [`reqwest::Client`].
    /// 1. Sends the request with that client.
    /// 1. Rejects non-success HTTP statuses with [`reqwest::Response::error_for_status`].
    /// 1. Deserializes the response body as JSON into [`WebFingerResponse`].
    ///
    /// Use this when the default Reqwest client configuration is sufficient. If you need shared
    /// connection pooling, custom headers, middleware, proxies, timeouts, or TLS settings, prefer
    /// [`Self::execute_reqwest_with_client`] instead.
    ///
    /// Errors are returned as [`crate::Error`]:
    ///
    /// - Request-construction failures surface as [`crate::Error::Http`] or
    ///   [`crate::Error::InvalidUri`].
    /// - Reqwest transport failures, non-success HTTP statuses, and JSON decoding failures surface
    ///   as [`crate::Error::Reqwest`].
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use webfinger_rs::WebFingerRequest;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let request = WebFingerRequest::builder("acct:carol@example.com")?
    ///     .host("example.com")
    ///     .rel("http://webfinger.net/rel/profile-page")
    ///     .build();
    ///
    /// let response = request.execute_reqwest().await?;
    /// println!("{response:#?}");
    /// # Ok(())
    /// # }
    /// ```
    #[tracing::instrument]
    pub async fn execute_reqwest(&self) -> Result<WebFingerResponse, Error> {
        let client = reqwest::Client::new();
        self.execute_reqwest_with_client(&client).await
    }

    /// Executes the WebFinger request with a caller-provided [`reqwest::Client`].
    ///
    /// This follows the same conversion, status handling, and JSON decoding path as
    /// [`Self::execute_reqwest`], but reuses the client you provide instead of constructing a new
    /// default one for each call.
    ///
    /// Use this when your application already owns a configured client, for example to:
    ///
    /// - reuse connection pools across multiple requests;
    /// - set default headers, user agents, or auth;
    /// - configure timeouts, proxies, redirects, or TLS behavior; or
    /// - integrate with Reqwest middleware or client-wide instrumentation.
    ///
    /// Non-success HTTP statuses and JSON decoding failures still surface as
    /// [`crate::Error::Reqwest`], because they originate from Reqwest's response handling.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use std::time::Duration;
    ///
    /// use reqwest::Client;
    /// use webfinger_rs::WebFingerRequest;
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::builder()
    ///     .timeout(Duration::from_secs(10))
    ///     .user_agent("webfinger-rs docs example")
    ///     .build()?;
    ///
    /// let request = WebFingerRequest::builder("acct:carol@example.com")?
    ///     .host("example.com")
    ///     .build();
    ///
    /// let response = request.execute_reqwest_with_client(&client).await?;
    /// println!("{response:#?}");
    /// # Ok(())
    /// # }
    /// ```
    #[tracing::instrument]
    pub async fn execute_reqwest_with_client(
        &self,
        client: &reqwest::Client,
    ) -> Result<WebFingerResponse, Error> {
        let request = self.try_into()?;
        trace!("request: {:?}", request);
        let response = client.execute(request).await?;
        trace!("response: {:?}", response);
        async_convert::TryFrom::try_from(response).await
    }

    /// Converts this WebFinger query into a [`reqwest::Request`] without executing it.
    ///
    /// This is useful when you want to inspect or modify the outgoing request before sending it,
    /// or when another part of your application is responsible for execution.
    ///
    /// The resulting request is an HTTPS `GET` to the WebFinger well-known endpoint with the
    /// current `resource`, `host`, and `rel` values encoded into the URL.
    ///
    /// This only performs request construction. It does not send anything over the network.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use webfinger_rs::WebFingerRequest;
    ///
    /// let request = WebFingerRequest::builder("acct:carol@example.com")?
    ///     .host("example.com")
    ///     .rel("http://webfinger.net/rel/profile-page")
    ///     .build();
    ///
    /// let reqwest_request = request.try_into_reqwest()?;
    /// assert_eq!(reqwest_request.method(), reqwest::Method::GET);
    /// assert_eq!(
    ///     reqwest_request.url().as_str(),
    ///     "https://example.com/.well-known/webfinger?resource=acct:carol@example.com&rel=http://webfinger.net/rel/profile-page"
    /// );
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn try_into_reqwest(&self) -> Result<reqwest::Request, Error> {
        self.try_into()
    }
}

impl WebFingerResponse {
    /// Converts a completed [`reqwest::Response`] into a [`WebFingerResponse`].
    ///
    /// This is useful when you execute the HTTP request yourself, but still want this crate's
    /// WebFinger response parsing behavior.
    ///
    /// The conversion:
    ///
    /// 1. Rejects non-success HTTP statuses with [`reqwest::Response::error_for_status`].
    /// 1. Deserializes the response body as JSON into [`WebFingerResponse`].
    ///
    /// Both status failures and JSON decoding failures surface as [`crate::Error::Reqwest`].
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use reqwest::Client;
    /// use webfinger_rs::{WebFingerRequest, WebFingerResponse};
    ///
    /// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::new();
    /// let request = WebFingerRequest::builder("acct:carol@example.com")?
    ///     .host("example.com")
    ///     .build()
    ///     .try_into_reqwest()?;
    ///
    /// let response = client.execute(request).await?;
    /// let webfinger = WebFingerResponse::try_from_reqwest(response).await?;
    /// println!("{webfinger:#?}");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn try_from_reqwest(response: reqwest::Response) -> Result<WebFingerResponse, Error> {
        async_convert::TryFrom::try_from(response).await
    }
}

#[async_convert::async_trait]
impl async_convert::TryFrom<reqwest::Response> for WebFingerResponse {
    type Error = crate::Error;

    async fn try_from(response: reqwest::Response) -> Result<WebFingerResponse, crate::Error> {
        let response = response.error_for_status()?;
        let response = response.json().await?;
        Ok(response)
    }
}
