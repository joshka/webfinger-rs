use axum::{
    async_trait, body::Body, extract::FromRequestParts, http::Response as AxumResponse,
    response::IntoResponse, Json,
};
use axum_extra::extract::{Query, QueryRejection};
use http::{
    header::{self, HOST},
    request::Parts,
    uri::InvalidUri,
    HeaderValue, StatusCode,
};
use tracing::trace;

use crate::{Rel, Request, Response};

const JRD_CONTENT_TYPE: HeaderValue = HeaderValue::from_static("application/jrd+json");

impl IntoResponse for Response {
    /// Converts a WebFinger response into an axum response.
    ///
    /// This is used to convert a WebFinger [`Response`] into an axum response in an axum route
    /// handler. The response will be serialized as JSON and the `Content-Type` header will be set
    /// to `application/jrd+json`.
    ///
    /// See the [axum example] for more information.
    ///
    /// # Example
    ///
    /// ```rust
    /// use axum::response::IntoResponse;
    /// use webfinger_rs::{Link, Request, Response};
    ///
    /// async fn handler(request: Request) -> impl IntoResponse {
    ///     // ... your code to handle the webfinger request ...
    ///     let subject = request.resource.to_string();
    ///     let link = Link::builder("http://webfinger.net/rel/profile-page")
    ///         .href(format!("https://example.com/profile/{subject}"));
    ///     Response::builder(subject).link(link).build()
    /// }
    /// ```
    ///
    /// [axum example]: http://github.com/joshka/webfinger-rs/blob/main/webfinger-rs/examples/axum.rs
    fn into_response(self) -> AxumResponse<Body> {
        let mut response = Json(self).into_response();
        let headers = response.headers_mut();
        headers.insert(header::CONTENT_TYPE, JRD_CONTENT_TYPE);
        response
    }
}

#[derive(Debug, serde::Deserialize)]
struct RequestParams {
    resource: String,
    #[serde(default)]
    rel: Vec<String>,
}

/// Rejection type for WebFinger requests.
///
/// This is used to represent errors that can occur when extracting a WebFinger request from the
/// request parts in an axum route handler.
pub enum Rejection {
    /// The `resource` query parameter is missing or invalid.
    InvalidQueryString(String),

    /// The `Host` header is missing.
    MissingHost,

    /// The `resource` query parameter is invalid.
    InvalidResource(InvalidUri),
}

impl IntoResponse for Rejection {
    /// Converts a WebFinger rejection into an axum response.
    fn into_response(self) -> AxumResponse<Body> {
        let (status, body) = match self {
            Rejection::MissingHost => (StatusCode::BAD_REQUEST, "missing host".to_string()),
            Rejection::InvalidQueryString(e) => (
                StatusCode::BAD_REQUEST,
                format!("invalid query string: {e}"),
            ),
            Rejection::InvalidResource(e) => {
                (StatusCode::BAD_REQUEST, format!("invalid resource: {e}"))
            }
        };
        AxumResponse::builder()
            .status(status)
            .body(Body::from(body))
            .unwrap()
    }
}

impl From<QueryRejection> for Rejection {
    fn from(rejection: QueryRejection) -> Self {
        Rejection::InvalidQueryString(rejection.to_string())
    }
}

#[async_trait]
impl<S: Send + Sync> FromRequestParts<S> for Request {
    type Rejection = Rejection;

    /// Extracts a `Request` from the request parts.
    ///
    /// This is used to extract a WebFinger [`Request`] from the request parts in an axum route
    /// handler.
    ///
    /// # Errors
    ///
    /// - If the request is missing the `Host` header, it will return a Bad Request response with
    /// the message "missing host".
    ///
    /// - If the `resource` query parameter is missing or invalid, it will return a Bad Request
    /// response with the message "invalid resource: {error}".
    ///
    /// - If the `rel` query parameter is invalid, it will return a Bad Request response with the
    /// message "invalid query string: {error}".
    ///
    /// See the [axum example] for more information.
    ///
    /// # Example
    ///
    /// ```rust
    /// use axum::response::IntoResponse;
    /// use webfinger_rs::Request;
    ///
    /// async fn handler(request: Request) -> impl IntoResponse {
    ///     let Request {
    ///         host,
    ///         resource,
    ///         rels,
    ///     } = request;
    ///     // ... your code to handle the webfinger request ...
    /// # webfinger_rs::Response::new(resource.to_string())
    /// }
    /// ```
    ///
    /// [axum example]:
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

        Ok(Request {
            host,
            resource,
            rels,
        })
    }
}

#[cfg(test)]
mod tests {
    use axum::routing::get;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use crate::WELL_KNOWN_PATH;

    use super::*;

    fn app() -> axum::Router {
        axum::Router::new().route(WELL_KNOWN_PATH, get(webfinger))
    }

    async fn webfinger(request: Request) -> impl IntoResponse {
        Response::builder(request.resource.to_string()).build()
    }

    #[tokio::test]
    async fn valid_request() {
        let uri = format!("https://example.com{WELL_KNOWN_PATH}?resource=acct:carol@example.com");
        let request = http::Request::builder()
            .uri(uri)
            .body(Body::empty())
            .unwrap();
        let response = app().oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK, "{response:?}");
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(
            &body[..],
            br#"{"subject":"acct:carol@example.com","links":[]}"#,
            "{body:?}"
        );
    }

    #[tokio::test]
    async fn valid_request_with_host_header() {
        let uri = format!("{WELL_KNOWN_PATH}?resource=acct:carol@example.com");
        let request = http::Request::builder()
            .uri(uri)
            .header(HOST, "example.com")
            .body(Body::empty())
            .unwrap();
        let response = app().oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK, "{response:?}");
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(
            &body[..],
            br#"{"subject":"acct:carol@example.com","links":[]}"#,
            "{body:?}"
        );
    }

    #[tokio::test]
    async fn request_with_no_host() {
        let uri = format!("{WELL_KNOWN_PATH}?resource=acct:carol@example.com");
        let request = http::Request::builder()
            .uri(uri)
            .body(Body::empty())
            .unwrap();
        let response = app().oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{response:?}");
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(&body[..], b"missing host", "{body:?}");
    }

    #[tokio::test]
    async fn request_with_missing_resource() {
        let request = http::Request::builder()
            .uri(WELL_KNOWN_PATH)
            .header(HOST, "example.com")
            .body(Body::empty())
            .unwrap();
        let response = app().oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{response:?}");
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(
            &body[..],
            b"invalid query string: missing field `resource`",
            "{body:?}"
        );
    }

    #[tokio::test]
    async fn request_with_invalid_resource() {
        let uri = format!("https://example.com{WELL_KNOWN_PATH}?resource=%");
        let request = http::Request::builder()
            .uri(uri)
            .body(Body::empty())
            .unwrap();
        let response = app().oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{response:?}");
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(
            &body[..],
            b"invalid resource: invalid authority",
            "{body:?}"
        );
    }
}
