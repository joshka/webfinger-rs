//! Axum integration for WebFinger request extraction and JRD responses.
//!
//! Enable the `axum` feature to:
//!
//! - extract [`WebFingerRequest`] from `GET` requests to [`crate::WELL_KNOWN_PATH`]; and
//! - return [`WebFingerResponse`] directly from Axum handlers as `application/jrd+json`.
//!
//! The extractor expects the standard WebFinger query shape from [RFC 7033 section 4.1]:
//!
//! - a required `resource` query parameter; and
//! - zero or more `rel` query parameters, encoded as repeated keys rather than a list.
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
//! If extraction fails, Axum receives [`Rejection`], which returns `400 Bad Request` with a plain
//! text message for missing hosts, malformed query strings, or invalid resource URIs.
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
use axum_extra::extract::{Query, QueryRejection};
use http::header::{self, HOST};
use http::request::Parts;
use http::uri::InvalidUri;
use http::{HeaderValue, StatusCode};
use tracing::trace;

use crate::{Rel, WebFingerRequest, WebFingerResponse};

const JRD_CONTENT_TYPE: HeaderValue = HeaderValue::from_static("application/jrd+json");

impl IntoResponse for WebFingerResponse {
    /// Converts a [`WebFingerResponse`] into an Axum response.
    ///
    /// This serializes the body as JSON and sets the `Content-Type` header to
    /// `application/jrd+json`, which is the JRD media type used by WebFinger.
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
        ([(header::CONTENT_TYPE, JRD_CONTENT_TYPE)], Json(self)).into_response()
    }
}

/// The query parameters for a WebFinger request.
#[derive(Debug, serde::Deserialize)]
struct RequestParams {
    resource: String,

    #[serde(default)]
    rel: Vec<String>,
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
/// - [`Rejection::InvalidQueryString`] when the query string is missing `resource` or otherwise
///   fails deserialization by [`axum_extra::extract::Query`]; and
/// - [`Rejection::InvalidResource`] when `resource` is present but cannot be parsed as an
///   [`http::Uri`].
pub enum Rejection {
    /// The `resource` query parameter is missing or invalid.
    InvalidQueryString(String),

    /// The `Host` header is missing.
    MissingHost,

    /// The `resource` query parameter is invalid.
    InvalidResource(InvalidUri),
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
            Rejection::InvalidQueryString(e) => format!("{e}"),
            Rejection::InvalidResource(e) => format!("invalid resource: {e}"),
        };
        (StatusCode::BAD_REQUEST, message).into_response()
    }
}

impl From<QueryRejection> for Rejection {
    fn from(rejection: QueryRejection) -> Self {
        Rejection::InvalidQueryString(rejection.to_string())
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
    /// - If the query string is missing `resource` or otherwise fails Axum query deserialization,
    ///   extraction fails with `Rejection::InvalidQueryString`.
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
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        trace!("request parts: {:?}", parts);

        let host = parts
            .uri
            .host()
            .or_else(|| parts.headers.get(HOST).and_then(|host| host.to_str().ok()))
            .map(str::to_string)
            .ok_or(Rejection::MissingHost)?;

        // use axum::extract::Query instead of axum::extract::Query, so that we can accept multiple
        // rel query parameters rather than this being provided as a sequence (`rel=[a,b,c]`).
        let query = Query::<RequestParams>::from_request_parts(parts, state).await?;
        let resource = query.resource.parse().map_err(Rejection::InvalidResource)?;
        let rels = query.rel.clone().into_iter().map(Rel::from).collect();

        Ok(WebFingerRequest {
            host,
            resource,
            rels,
        })
    }
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::routing::get;
    use http::{Request, Response};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use super::*;
    use crate::WELL_KNOWN_PATH;

    type Result<T = (), E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

    /// A small helper trait to convert a response body into a string.
    trait IntoText {
        async fn into_text(self) -> Result<String>;
    }

    impl IntoText for Response<Body> {
        async fn into_text(self) -> Result<String> {
            let body = self.into_body().collect().await?.to_bytes();
            let string = String::from_utf8(body.to_vec())?;
            Ok(string)
        }
    }

    fn app() -> axum::Router {
        axum::Router::new().route(WELL_KNOWN_PATH, get(webfinger))
    }

    async fn webfinger(request: WebFingerRequest) -> impl IntoResponse {
        WebFingerResponse::builder(request.resource.to_string()).build()
    }

    const VALID_RESOURCE: &str = "acct:carol@example.com";

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

    #[tokio::test]
    async fn request_with_missing_resource() -> Result {
        let request = Request::builder()
            .uri(WELL_KNOWN_PATH)
            .header(HOST, "example.com")
            .body(Body::empty())?;

        let response = app().oneshot(request).await?;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{response:?}");
        let body = response.into_text().await?;
        assert_eq!(
            body,
            "Failed to deserialize query string: missing field `resource`",
        );
        Ok(())
    }

    #[tokio::test]
    async fn request_with_invalid_resource() -> Result {
        let uri = format!("https://example.com{WELL_KNOWN_PATH}?resource=%");
        let request = Request::builder().uri(uri).body(Body::empty())?;

        let response = app().oneshot(request).await?;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{response:?}");
        let body = response.into_text().await?;
        assert_eq!(body, "invalid resource: invalid authority");
        Ok(())
    }
}
