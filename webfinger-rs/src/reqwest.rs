use std::sync::Once;

use http::Uri;
use tracing::trace;

use crate::error::Error;
use crate::{WebFingerRequest, WebFingerResponse};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EmptyBody;

static DEFAULT_CRYPTO_PROVIDER: Once = Once::new();

fn install_default_crypto_provider() {
    DEFAULT_CRYPTO_PROVIDER.call_once(|| {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    });
}

fn webfinger_reqwest_client() -> Result<reqwest::Client, reqwest::Error> {
    install_default_crypto_provider();
    reqwest::Client::builder().https_only(true).build()
}

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
    /// 1. Creates a new [`reqwest::Client`] that only sends HTTPS requests, including redirects.
    /// 1. Sends the request with that client.
    /// 1. Rejects non-success HTTP statuses with [`reqwest::Response::error_for_status`].
    /// 1. Deserializes the response body as JSON into [`WebFingerResponse`].
    ///
    /// Use this when the first-party WebFinger client configuration is sufficient. This path
    /// follows RFC 7033's HTTPS-only transport requirements by rejecting redirects to non-HTTPS
    /// targets. If you need shared connection pooling, custom headers, middleware, proxies,
    /// timeouts, or TLS settings, prefer [`Self::execute_reqwest_with_client`] instead.
    ///
    /// Errors are returned as [`crate::Error`]:
    ///
    /// - Request-construction failures surface as [`crate::Error::Http`] or
    ///   [`crate::Error::InvalidUri`].
    /// - Reqwest client-construction failures, transport failures, non-success HTTP statuses, and
    ///   JSON decoding failures surface as [`crate::Error::Reqwest`].
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
        let client = webfinger_reqwest_client()?;
        self.execute_reqwest_with_client(&client).await
    }

    /// Executes the WebFinger request with a caller-provided [`reqwest::Client`].
    ///
    /// This follows the same conversion, status handling, and JSON decoding path as
    /// [`Self::execute_reqwest`], but reuses the client you provide instead of constructing a new
    /// WebFinger-specific one for each call.
    ///
    /// RFC 7033 requires clients to query WebFinger resources using HTTPS only and allows redirects
    /// only to HTTPS URIs. Caller-provided clients are used as-is, so configure them to reject
    /// non-HTTPS requests and redirect targets when you need RFC-compliant WebFinger execution.
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
    ///     .https_only(true)
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
    ///     "https://example.com/.well-known/webfinger?resource=acct%3Acarol%40example.com&rel=http%3A%2F%2Fwebfinger.net%2Frel%2Fprofile-page"
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

#[cfg(test)]
mod tests {
    use http::StatusCode;
    use http::header::CONTENT_TYPE;
    use reqwest::Method;

    use super::*;

    fn reqwest_response(status: StatusCode, body: &'static str) -> reqwest::Response {
        http::Response::builder()
            .status(status)
            .header(CONTENT_TYPE, "application/jrd+json")
            .body(body)
            .unwrap()
            .into()
    }

    /// Converts the typed request into the exact Reqwest method and URL shape sent on the wire.
    ///
    /// This is the low-level inspection path callers can use before execution, and it should match
    /// the RFC 7033 query encoding used by the `http::Uri` conversion.
    #[test]
    fn try_into_reqwest_builds_get_request() {
        let request = WebFingerRequest::builder("acct:carol@example.org")
            .unwrap()
            .host("example.org")
            .rel("avatar")
            .build();

        let reqwest_request = request.try_into_reqwest().unwrap();

        assert_eq!(reqwest_request.method(), Method::GET);
        assert_eq!(
            reqwest_request.url().as_str(),
            "https://example.org/.well-known/webfinger?resource=acct%3Acarol%40example.org&rel=avatar",
        );
    }

    /// Surfaces invalid endpoint hosts as request-construction failures.
    ///
    /// Host validation happens when the typed WebFinger request is lowered into an HTTP request, so
    /// this guards the error variant callers see before any transport is involved.
    #[test]
    fn try_into_reqwest_rejects_invalid_host() {
        let request = WebFingerRequest::builder("acct:carol@example.org")
            .unwrap()
            .host("exa mple.org")
            .build();

        let error = request.try_into_reqwest().expect_err("invalid host");

        assert!(matches!(error, Error::Http(_)));
    }

    /// RFC 7033 sections 4.2 and 9.1 require WebFinger clients to use HTTPS-only transport. The
    /// first-party client should reject an HTTP URL before attempting network I/O; the same Reqwest
    /// setting also applies to redirect targets.
    #[tokio::test]
    async fn default_webfinger_client_rejects_non_https_requests() {
        let client = webfinger_reqwest_client().unwrap();
        let url = "http://127.0.0.1:9/.well-known/webfinger?resource=acct:carol@example.org"
            .parse()
            .unwrap();
        let request = reqwest::Request::new(Method::GET, url);

        let error = client.execute(request).await.unwrap_err();

        assert!(error.is_builder());
        assert_eq!(error.url().map(reqwest::Url::scheme), Some("http"));
    }

    /// Parses a successful HTTP response through the public response-conversion helper.
    ///
    /// Applications may execute requests themselves and still rely on this crate for JRD status
    /// handling and JSON decoding, so the helper needs direct coverage independent of networking.
    #[tokio::test]
    async fn try_from_reqwest_parses_success_response() {
        let body = r#"{"subject":"acct:carol@example.org","links":[{"rel":"avatar"}]}"#;
        let response = reqwest_response(StatusCode::OK, body);

        let response = WebFingerResponse::try_from_reqwest(response).await.unwrap();

        assert_eq!(
            response,
            WebFingerResponse::builder("acct:carol@example.org")
                .link(crate::Link::builder("avatar"))
                .build()
        );
    }

    /// Rejects non-success statuses before attempting to parse the JRD body.
    ///
    /// Reqwest owns HTTP status classification here, and callers should receive the same error
    /// family for status failures as for other response-handling failures.
    #[tokio::test]
    async fn try_from_reqwest_rejects_error_status() {
        let response = reqwest_response(
            StatusCode::NOT_FOUND,
            r#"{"subject":"acct:carol@example.org","links":[]}"#,
        );

        let error = WebFingerResponse::try_from_reqwest(response)
            .await
            .expect_err("error status");

        assert!(matches!(error, Error::Reqwest(_)));
    }

    /// Rejects malformed JSON bodies after a successful status.
    ///
    /// This keeps the response conversion contract narrow: success requires both a successful HTTP
    /// status and a valid WebFinger JRD JSON document.
    #[tokio::test]
    async fn try_from_reqwest_rejects_invalid_json() {
        let response = reqwest_response(StatusCode::OK, "not json");

        let error = WebFingerResponse::try_from_reqwest(response)
            .await
            .expect_err("invalid json");

        assert!(matches!(error, Error::Reqwest(_)));
    }
}
