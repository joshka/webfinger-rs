//! Axum integration for WebFinger request extraction and JRD responses.
//!
//! Enable the `axum` feature to:
//!
//! - extract [`WebFingerRequest`] from handlers mounted for `GET` requests to
//!   [`crate::WELL_KNOWN_PATH`]; and
//! - return [`WebFingerResponse`] directly from Axum handlers as `application/jrd+json` with the
//!   WebFinger CORS header.
//!
//! The extractor expects the standard WebFinger query shape from [RFC 7033 section 4.1]:
//!
//! - a required `resource` query parameter; and
//! - zero or more `rel` query parameters, encoded as repeated keys rather than a list.
//!
//! The `resource` value must be an absolute URI such as `acct:carol@example.com` or
//! `https://example.com/users/carol`; relative references are rejected as malformed requests.
//!
//! In practice, route handlers should usually be mounted like this:
//!
//! ```rust
//! use axum::{Router, routing::get};
//! use webfinger_rs::{WELL_KNOWN_PATH, WebFingerRequest, WebFingerResponse};
//!
//! async fn webfinger(_request: WebFingerRequest) -> WebFingerResponse {
//!     WebFingerResponse::new("acct:carol@example.com")
//! }
//!
//! let app = Router::<()>::new().route(WELL_KNOWN_PATH, get(webfinger));
//! # let _ = app;
//! ```
//!
//! The Axum router owns path and method matching. Mounting the handler with `get` at
//! [`crate::WELL_KNOWN_PATH`] rejects other paths and non-`GET` methods before this extractor runs.
//! The extractor itself validates the WebFinger request metadata available inside the handler:
//! host, query parameters, percent encoding, and the `resource` URI.
//!
//! RFC 7033 requires HTTPS for WebFinger. Axum request parts do not reliably identify the
//! externally visible scheme when the application runs behind TLS termination or a reverse proxy, so
//! this extractor does not enforce scheme. Configure TLS and forwarded-proto handling at your
//! server or proxy boundary.
//!
//! If extraction fails, Axum receives [`Rejection`], which returns `400 Bad Request` with a plain
//! text message for missing or duplicated `resource`, missing host values, invalid percent
//! encoding, relative resource references, or invalid resource URIs.
//!
//! See also [`WebFingerRequest`] for the extractor impl, [`WebFingerResponse`] for the responder
//! impl, and the [Axum example] for a runnable server.
//!
//! [RFC 7033 section 4.1]: https://www.rfc-editor.org/rfc/rfc7033.html#section-4.1
//! [Axum example]:
//!     https://github.com/joshka/webfinger-rs/blob/main/webfinger-rs/examples/axum.rs

use axum::Json;
use axum::extract::FromRequestParts;
use axum::response::{IntoResponse, Response as AxumResponse};
use http::header::{self, HOST};
use http::request::Parts;
use http::{HeaderValue, StatusCode};
use tracing::trace;

use crate::http::CORS_ALLOW_ORIGIN;
use crate::query::{RequestParams, RequestParamsError};
use crate::{Rel, ResourceError, WebFingerRequest, WebFingerResponse};

const JRD_CONTENT_TYPE: HeaderValue = HeaderValue::from_static("application/jrd+json");
const CORS_ALLOW_ORIGIN_HEADER: HeaderValue = HeaderValue::from_static(CORS_ALLOW_ORIGIN);

impl IntoResponse for WebFingerResponse {
    /// Converts a [`WebFingerResponse`] into an Axum response.
    ///
    /// This serializes the body as JSON, sets the `Content-Type` header to
    /// `application/jrd+json`, and allows cross-origin browser requests with
    /// `Access-Control-Allow-Origin: *` as recommended by RFC 7033 section 5.
    ///
    /// Handlers can therefore return [`WebFingerResponse`] directly without manually wrapping it in
    /// [`axum::Json`] or setting the response header themselves.
    ///
    /// Mount the route at [`crate::WELL_KNOWN_PATH`] so the handler matches the standard WebFinger
    /// endpoint path.
    ///
    /// See also the [`crate::axum`] module docs and the [Axum example].
    ///
    /// # Example
    ///
    /// ```rust
    /// use axum::{Router, routing::get};
    /// use http::StatusCode;
    /// use webfinger_rs::{Link, Rel, WELL_KNOWN_PATH, WebFingerRequest, WebFingerResponse};
    ///
    /// async fn webfinger(request: WebFingerRequest) -> axum::response::Result<WebFingerResponse> {
    ///     let subject = request.resource.to_string();
    ///     if subject != "acct:carol@example.com" {
    ///         return Err((StatusCode::NOT_FOUND, "not found").into());
    ///     }
    ///
    ///     let rel = Rel::new("http://webfinger.net/rel/profile-page");
    ///     let response = if request.rels.is_empty() || request.rels.contains(&rel) {
    ///         let link = Link::builder(rel).href("https://example.com/users/carol");
    ///         WebFingerResponse::builder(subject).link(link).build()
    ///     } else {
    ///         WebFingerResponse::builder(subject).build()
    ///     };
    ///     Ok(response)
    /// }
    ///
    /// let app = Router::<()>::new().route(WELL_KNOWN_PATH, get(webfinger));
    /// # let _ = app;
    /// ```
    ///
    /// [Axum example]:
    ///     https://github.com/joshka/webfinger-rs/blob/main/webfinger-rs/examples/axum.rs
    fn into_response(self) -> AxumResponse {
        (
            [
                (header::CONTENT_TYPE, JRD_CONTENT_TYPE),
                (
                    header::ACCESS_CONTROL_ALLOW_ORIGIN,
                    CORS_ALLOW_ORIGIN_HEADER,
                ),
            ],
            Json(self),
        )
            .into_response()
    }
}

/// Rejection type for WebFinger requests.
///
/// This represents errors that can occur while extracting [`WebFingerRequest`] from Axum request
/// parts.
///
/// Each variant maps to `400 Bad Request` when converted into an Axum response:
///
/// - [`Rejection::MissingHost`] when neither the request URI nor the `Host` header provides an
///   authority;
/// - [`Rejection::InvalidQueryString`] when the query string is missing `resource`, contains more
///   than one `resource`, or contains malformed percent encoding;
/// - [`Rejection::InvalidResource`] when the `resource` value is not an absolute URI.
#[derive(Debug)]
pub enum Rejection {
    /// The WebFinger query string is missing required data or is malformed.
    InvalidQueryString(String),

    /// The `resource` query parameter is not an absolute URI.
    InvalidResource(ResourceError),

    /// The `Host` header is missing.
    MissingHost,

    /// A `rel` query parameter is invalid.
    InvalidRel(crate::Error),
}

impl IntoResponse for Rejection {
    /// Converts the rejection into a `400 Bad Request` Axum response.
    ///
    /// The body is a plain text error message intended to make local debugging and simple server
    /// implementations straightforward.
    ///
    /// See also the [`crate::axum`] module docs.
    fn into_response(self) -> AxumResponse {
        let message = match self {
            Rejection::MissingHost => "missing host".to_string(),
            Rejection::InvalidQueryString(error) => error,
            Rejection::InvalidResource(error) => format!("invalid resource: {error}"),
            Rejection::InvalidRel(error) => error.to_string(),
        };
        (StatusCode::BAD_REQUEST, message).into_response()
    }
}

impl From<RequestParamsError> for Rejection {
    fn from(error: RequestParamsError) -> Self {
        match error {
            RequestParamsError::InvalidResource(error) => Rejection::InvalidResource(error),
            error => Rejection::InvalidQueryString(error.to_string()),
        }
    }
}

impl<S: Send + Sync> FromRequestParts<S> for WebFingerRequest {
    type Rejection = Rejection;

    /// Extracts a [`WebFingerRequest`] from Axum request parts.
    ///
    /// The extractor expects a request routed to [`crate::WELL_KNOWN_PATH`] with:
    ///
    /// - a `resource` query parameter containing the target resource URI; and
    /// - zero or more repeated `rel` query parameters.
    ///
    /// Host resolution follows this order:
    ///
    /// 1. Use the authority from `parts.uri` when the request URI is absolute.
    /// 1. Otherwise, fall back to the HTTP `Host` header.
    ///
    /// The extracted host, parsed resource, and collected relations are used to construct the
    /// resulting [`WebFingerRequest`].
    ///
    /// # Errors
    ///
    /// - If the request has neither a URI authority nor a `Host` header, extraction fails with
    ///   `Rejection::MissingHost`.
    /// - If the query string is missing `resource`, contains more than one `resource`, or contains
    ///   malformed percent encoding, extraction fails with `Rejection::InvalidQueryString`.
    /// - If `resource` is present but cannot be parsed as a URI, extraction fails with
    ///   `Rejection::InvalidResource`.
    ///
    /// See also the [`crate::axum`] module docs and the [Axum example].
    ///
    /// # Example
    ///
    /// ```rust
    /// use axum::{Router, routing::get};
    /// use webfinger_rs::{WELL_KNOWN_PATH, WebFingerRequest, WebFingerResponse};
    ///
    /// async fn webfinger(request: WebFingerRequest) -> WebFingerResponse {
    ///     WebFingerResponse::new(request.resource.to_string())
    /// }
    ///
    /// let app = Router::<()>::new().route(WELL_KNOWN_PATH, get(webfinger));
    /// # let _ = app;
    /// ```
    ///
    /// [Axum example]:
    ///     https://github.com/joshka/webfinger-rs/blob/main/webfinger-rs/examples/axum.rs
    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        trace!("request parts: {:?}", parts);

        let host = parts
            .uri
            .host()
            .or_else(|| parts.headers.get(HOST).and_then(|host| host.to_str().ok()))
            .map(str::to_string)
            .ok_or(Rejection::MissingHost)?;

        let query: RequestParams = parts.uri.query().unwrap_or("").parse()?;
        let rels = query
            .rel
            .into_iter()
            .map(Rel::try_new)
            .collect::<Result<Vec<_>, _>>()
            .map_err(Rejection::InvalidRel)?;

        Ok(WebFingerRequest {
            host,
            resource: query.resource,
            rels,
        })
    }
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::routing::get;
    use http::{Method, Request, Response};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use super::*;
    use crate::WELL_KNOWN_PATH;

    type Result<T = (), E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

    /// A small helper trait to convert a response body into a string.
    trait IntoText {
        /// Consumes the response body and decodes it as UTF-8 text.
        async fn into_text(self) -> Result<String>;
    }

    impl IntoText for Response<Body> {
        async fn into_text(self) -> Result<String> {
            let body = self.into_body().collect().await?.to_bytes();
            let string = String::from_utf8(body.to_vec())?;
            Ok(string)
        }
    }

    /// Builds a test router using the resource-echoing WebFinger handler.
    fn app() -> axum::Router {
        axum::Router::new().route(WELL_KNOWN_PATH, get(webfinger))
    }

    /// Builds a test router using the relation-echoing WebFinger handler.
    fn rels_app() -> axum::Router {
        axum::Router::new().route(WELL_KNOWN_PATH, get(webfinger_rels))
    }

    /// Returns a minimal JRD response so tests can assert resource extraction through Axum.
    async fn webfinger(request: WebFingerRequest) -> impl IntoResponse {
        WebFingerResponse::builder(&request.resource).build()
    }

    /// Returns extracted relation filters so tests can assert RFC 7033 repeated `rel` handling.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc7033.html#section-4.1>.
    async fn webfinger_rels(request: WebFingerRequest) -> impl IntoResponse {
        let rels = request
            .rels
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        Json(rels)
    }

    const VALID_RESOURCE: &str = "acct:carol@example.com";

    /// Accepts an ordinary `acct:` resource from an absolute request URI.
    ///
    /// This covers the common Axum path where the request URI already contains the authority, so host
    /// extraction should not depend on the `Host` header.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc7033.html#section-4>.
    #[tokio::test]
    async fn valid_request() -> Result {
        let uri = format!("https://example.com{WELL_KNOWN_PATH}?resource={VALID_RESOURCE}");
        let request = Request::builder().uri(uri).body(Body::empty())?;

        let response = app().oneshot(request).await?;

        assert_eq!(response.status(), StatusCode::OK, "{response:?}");
        let body = response.into_text().await?;
        assert_eq!(body, r#"{"subject":"acct:carol@example.com","links":[]}"#);
        Ok(())
    }

    /// Includes the RFC 7033 CORS header on successful JRD responses.
    ///
    /// WebFinger resources must be queryable from browsers, and RFC 7033 section 5 recommends the
    /// least restrictive `Access-Control-Allow-Origin` value for public WebFinger resources.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc7033.html#section-5>.
    #[tokio::test]
    async fn successful_response_sets_cors_header() -> Result {
        let uri = format!("https://example.com{WELL_KNOWN_PATH}?resource={VALID_RESOURCE}");
        let request = Request::builder().uri(uri).body(Body::empty())?;

        let response = app().oneshot(request).await?;

        assert_eq!(response.status(), StatusCode::OK, "{response:?}");
        assert_eq!(
            response.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN),
            Some(&CORS_ALLOW_ORIGIN_HEADER),
        );
        Ok(())
    }

    /// Accepts an ordinary `acct:` resource when only the `Host` header carries the authority.
    ///
    /// Axum tests usually build origin-form request URIs, so this catches regressions where the
    /// extractor ignores the fallback authority that HTTP/1.1 clients send in `Host`.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc7033.html#section-4>.
    #[tokio::test]
    async fn valid_request_with_host_header() -> Result {
        let request = Request::builder()
            .uri(format!("{WELL_KNOWN_PATH}?resource={VALID_RESOURCE}"))
            .header(HOST, "example.com")
            .body(Body::empty())?;

        let response = app().oneshot(request).await?;

        assert_eq!(response.status(), StatusCode::OK, "{response:?}");
        let body = response.into_text().await?;
        assert_eq!(body, r#"{"subject":"acct:carol@example.com","links":[]}"#);
        Ok(())
    }

    /// Relies on Axum routing to reject non-WebFinger paths before extraction.
    ///
    /// RFC 7033 sections 4 and 10.1 define `/.well-known/webfinger` as the WebFinger resource.
    /// Path matching stays in the router so applications get normal Axum `404 Not Found` behavior.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc7033.html#section-4> and
    /// <https://www.rfc-editor.org/rfc/rfc7033.html#section-10.1>.
    #[tokio::test]
    async fn wrong_path_is_not_routed() -> Result {
        let request = Request::builder()
            .uri(format!("/webfinger?resource={VALID_RESOURCE}"))
            .header(HOST, "example.com")
            .body(Body::empty())?;

        let response = app().oneshot(request).await?;

        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{response:?}");
        Ok(())
    }

    /// Relies on Axum routing to reject non-`GET` requests before extraction.
    ///
    /// RFC 7033 section 4.2 specifies a `GET` request. Method matching stays in the router so
    /// applications get normal Axum `405 Method Not Allowed` behavior.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc7033.html#section-4.2>.
    #[tokio::test]
    async fn wrong_method_is_not_routed() -> Result {
        let request = Request::builder()
            .method(Method::POST)
            .uri(format!("{WELL_KNOWN_PATH}?resource={VALID_RESOURCE}"))
            .header(HOST, "example.com")
            .body(Body::empty())?;

        let response = app().oneshot(request).await?;

        assert_eq!(
            response.status(),
            StatusCode::METHOD_NOT_ALLOWED,
            "{response:?}"
        );
        Ok(())
    }

    /// Rejects requests where neither the URI nor `Host` header provides an authority.
    ///
    /// The request host is significant to WebFinger query routing.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc7033.html#section-4>.
    #[tokio::test]
    async fn request_with_no_host() -> Result {
        let uri = format!("{WELL_KNOWN_PATH}?resource={VALID_RESOURCE}");
        let request = Request::builder().uri(uri).body(Body::empty())?;

        let response = app().oneshot(request).await?;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{response:?}");
        let body = response.into_text().await?;
        assert_eq!(body, "missing host");
        Ok(())
    }

    /// Rejects requests that omit the required `resource` parameter.
    ///
    /// RFC 7033 section 4.2 treats absent `resource` parameters as bad requests. This prevents the
    /// Axum adapter from relying on framework deserialization wording or accepting an empty target.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc7033.html#section-4.2>.
    #[tokio::test]
    async fn request_with_missing_resource() -> Result {
        let request = Request::builder()
            .uri(WELL_KNOWN_PATH)
            .header(HOST, "example.com")
            .body(Body::empty())?;

        let response = app().oneshot(request).await?;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{response:?}");
        let body = response.into_text().await?;
        assert_eq!(body, "missing resource parameter");
        Ok(())
    }

    /// Converts malformed resource values into Axum bad-request responses.
    ///
    /// RFC 7033 section 4.2 requires malformed `resource` parameters to be treated as bad requests
    /// instead of panicking inside extraction.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc7033.html#section-4.2>.
    #[tokio::test]
    async fn request_with_invalid_resource() -> Result {
        let uri = format!("https://example.com{WELL_KNOWN_PATH}?resource=http%3A%2F%2F%5B%3A%3A1");
        let request = Request::builder().uri(uri).body(Body::empty())?;

        let response = app().oneshot(request).await?;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{response:?}");
        let body = response.into_text().await?;
        assert_eq!(body, "invalid resource: invalid authority");
        Ok(())
    }

    /// Preserves the typed resource parse error until Axum renders the rejection.
    #[test]
    fn invalid_resource_rejection_preserves_resource_error() {
        let error = "resource=/relative".parse::<RequestParams>().unwrap_err();
        let rejection = Rejection::from(error);

        assert!(matches!(
            rejection,
            Rejection::InvalidResource(ResourceError::RelativeReference)
        ));
    }

    /// Rejects relative resource references at the Axum extractor boundary.
    ///
    /// RFC 7033 identifies the WebFinger query target as a URI, not a relative reference. Axum
    /// handlers should not receive ambiguous targets such as local paths or bare names.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc7033.html#section-4.1> and
    /// <https://www.rfc-editor.org/rfc/rfc3986.html#section-4.1>.
    #[tokio::test]
    async fn relative_resource_is_bad_request() -> Result {
        let uri = format!("https://example.com{WELL_KNOWN_PATH}?resource=/relative");
        let request = Request::builder().uri(uri).body(Body::empty())?;

        let response = app().oneshot(request).await?;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{response:?}");
        let body = response.into_text().await?;
        assert_eq!(body, "invalid resource: resource must be an absolute URI");
        Ok(())
    }

    /// Accepts a percent-encoded `acct:` resource without panicking.
    ///
    /// The resource query value is percent-encoded under RFC 7033 section 4.1, then parsed as a URI
    /// query target under RFC 7033 section 4.2.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc7033.html#section-4.1> and
    /// <https://www.rfc-editor.org/rfc/rfc7033.html#section-4.2>.
    #[tokio::test]
    async fn valid_percent_encoded_resource() -> Result {
        let uri = format!("https://example.com{WELL_KNOWN_PATH}?resource=acct%3Abad%40example.org");
        let request = Request::builder().uri(uri).body(Body::empty())?;

        let response = app().oneshot(request).await?;

        assert_eq!(response.status(), StatusCode::OK, "{response:?}");
        let body = response.into_text().await?;
        assert_eq!(body, r#"{"subject":"acct:bad@example.org","links":[]}"#);
        Ok(())
    }

    /// Preserves repeated `rel` parameters instead of collapsing them.
    ///
    /// WebFinger clients use repeated `rel` keys to request multiple relation filters. A generic
    /// map-shaped query parser can easily keep only one value, which would make handlers see an
    /// incomplete request.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc7033.html#section-4.1>.
    #[tokio::test]
    async fn valid_request_with_repeated_rel_params() -> Result {
        let resource = "acct%3Acarol%40example.org";
        let uri = format!(
            "https://example.com{WELL_KNOWN_PATH}?resource={resource}&rel=profile&rel=avatar"
        );
        let request = Request::builder().uri(uri).body(Body::empty())?;

        let response = rels_app().oneshot(request).await?;

        assert_eq!(response.status(), StatusCode::OK, "{response:?}");
        let body = response.into_text().await?;
        assert_eq!(body, r#"["profile","avatar"]"#);
        Ok(())
    }

    /// Exposes decoded relation URIs to Axum handlers.
    ///
    /// The shared parser owns the RFC 3986 percent-decoding rule; this adapter test proves Axum
    /// handlers receive decoded `Rel` values rather than raw query text.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc7033.html#section-4.1> and
    /// <https://www.rfc-editor.org/rfc/rfc3986.html#section-2.1>.
    #[tokio::test]
    async fn rel_params_are_percent_decoded() -> Result {
        let resource = "acct%3Acarol%40example.org";
        let rel = "http%3A%2F%2Fwebfinger.example%2Frel%2Fprofile-page";
        let uri = format!("https://example.com{WELL_KNOWN_PATH}?resource={resource}&rel={rel}");
        let request = Request::builder().uri(uri).body(Body::empty())?;

        let response = rels_app().oneshot(request).await?;

        assert_eq!(response.status(), StatusCode::OK, "{response:?}");
        let body = response.into_text().await?;
        assert_eq!(body, r#"["http://webfinger.example/rel/profile-page"]"#);
        Ok(())
    }

    /// Rejects relation values that are neither one registered relation type nor one URI.
    ///
    /// RFC 7033 section 4.4.4.1 allows one relation type per `rel` member. Multiple relation
    /// filters should be encoded as repeated `rel` parameters, not as whitespace-separated values.
    #[tokio::test]
    async fn invalid_rel_is_bad_request() -> Result {
        let resource = "acct%3Acarol%40example.org";
        let uri =
            format!("https://example.com{WELL_KNOWN_PATH}?resource={resource}&rel=author%20avatar");
        let request = Request::builder().uri(uri).body(Body::empty())?;

        let response = rels_app().oneshot(request).await?;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{response:?}");
        let body = response.into_text().await?;
        assert_eq!(body, "invalid relation type: author avatar");
        Ok(())
    }

    /// Converts invalid UTF-8 after percent decoding into an Axum bad-request response.
    ///
    /// The shared parser owns the byte-level validation; this adapter test proves malformed
    /// percent-encoded bytes do not reach an Axum handler as relation strings.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc3986.html#section-2.1>.
    #[tokio::test]
    async fn invalid_percent_encoded_rel_is_bad_request() -> Result {
        let resource = "acct%3Acarol%40example.org";
        let uri = format!("https://example.com{WELL_KNOWN_PATH}?resource={resource}&rel=%FF");
        let request = Request::builder().uri(uri).body(Body::empty())?;

        let response = rels_app().oneshot(request).await?;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{response:?}");
        let body = response.into_text().await?;
        assert_eq!(body, "invalid percent-encoded query parameter");
        Ok(())
    }

    /// Rejects malformed percent escape syntax instead of treating `%` literally.
    ///
    /// The shared query parser owns the RFC 3986 check; this Axum test proves that parser errors are
    /// converted into `400 Bad Request` responses instead of escaping the extractor boundary.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc3986.html#section-2.1>.
    #[tokio::test]
    async fn malformed_percent_escape_is_bad_request() -> Result {
        let resource = "acct%3Acarol%40example.org";
        let uri = format!("https://example.com{WELL_KNOWN_PATH}?resource={resource}&rel=%GG");
        let request = Request::builder().uri(uri).body(Body::empty())?;

        let response = rels_app().oneshot(request).await?;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{response:?}");
        let body = response.into_text().await?;
        assert_eq!(body, "invalid percent-encoded query parameter");
        Ok(())
    }

    /// Accepts `resource` in any query parameter position through the Axum extractor.
    ///
    /// RFC 7033 section 4.1 does not make parameter order significant. This adapter test proves
    /// Axum handlers still receive relation filters when `resource` appears after them.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc7033.html#section-4.1>.
    #[tokio::test]
    async fn resource_parameter_order_does_not_matter() -> Result {
        let resource = "acct%3Acarol%40example.org";
        let uri = format!("https://example.com{WELL_KNOWN_PATH}?rel=profile&resource={resource}");
        let request = Request::builder().uri(uri).body(Body::empty())?;

        let response = rels_app().oneshot(request).await?;

        assert_eq!(response.status(), StatusCode::OK, "{response:?}");
        let body = response.into_text().await?;
        assert_eq!(body, r#"["profile"]"#);
        Ok(())
    }

    /// Keeps encoded `=` and `&` inside handler-visible resource values.
    ///
    /// Resource URIs may contain query strings of their own. This adapter test proves Axum receives
    /// the decoded target resource without splitting encoded inner delimiters into WebFinger
    /// parameters.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc7033.html#section-4.1>.
    #[tokio::test]
    async fn encoded_delimiters_stay_inside_resource() -> Result {
        let resource = "https%3A%2F%2Fexample.org%2Fprofile%3Fa%3D1%26b%3D2";
        let uri = format!("https://example.com{WELL_KNOWN_PATH}?resource={resource}");
        let request = Request::builder().uri(uri).body(Body::empty())?;

        let response = app().oneshot(request).await?;

        assert_eq!(response.status(), StatusCode::OK, "{response:?}");
        let body = response.into_text().await?;
        assert_eq!(
            body,
            r#"{"subject":"https://example.org/profile?a=1&b=2","links":[]}"#,
        );
        Ok(())
    }

    /// Preserves literal `+` in Axum handler-visible resources.
    ///
    /// Framework form-query extractors are not used here because WebFinger follows RFC 3986 query
    /// semantics, where `+` remains data instead of becoming a space.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc3986.html#section-3.4>.
    #[tokio::test]
    async fn plus_is_not_decoded_as_space() -> Result {
        let uri =
            format!("https://example.com{WELL_KNOWN_PATH}?resource=acct%3Acarol+tag%40example.org");
        let request = Request::builder().uri(uri).body(Body::empty())?;

        let response = app().oneshot(request).await?;

        assert_eq!(response.status(), StatusCode::OK, "{response:?}");
        let body = response.into_text().await?;
        assert_eq!(
            body,
            r#"{"subject":"acct:carol+tag@example.org","links":[]}"#
        );
        Ok(())
    }

    /// Rejects duplicate `resource` parameters at the Axum extractor boundary.
    ///
    /// The parser owns the RFC 7033 section 4.2 rule that there is exactly one target. This adapter
    /// test proves ambiguous requests become `400 Bad Request` responses rather than arbitrary
    /// handler inputs.
    ///
    /// See <https://www.rfc-editor.org/rfc/rfc7033.html#section-4.2>.
    #[tokio::test]
    async fn request_with_multiple_resources() -> Result {
        let carol = "acct%3Acarol%40example.org";
        let alice = "acct%3Aalice%40example.org";
        let uri = format!("https://example.com{WELL_KNOWN_PATH}?resource={carol}&resource={alice}");
        let request = Request::builder().uri(uri).body(Body::empty())?;

        let response = app().oneshot(request).await?;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{response:?}");
        let body = response.into_text().await?;
        assert_eq!(body, "multiple resource parameters");
        Ok(())
    }
}
