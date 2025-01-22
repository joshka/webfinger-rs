use axum::{
    extract::FromRequestParts,
    response::{IntoResponse, Response as AxumResponse},
    Json,
};
use axum_extra::extract::{Query, QueryRejection};
use http::{
    header::{self, HOST},
    request::Parts,
    uri::InvalidUri,
    HeaderValue, StatusCode,
};
use tracing::trace;

use crate::{Rel, WebFingerRequest, WebFingerResponse};

const JRD_CONTENT_TYPE: HeaderValue = HeaderValue::from_static("application/jrd+json");

impl IntoResponse for WebFingerResponse {
    /// Converts a WebFinger response into an axum response.
    ///
    /// This is used to convert a [`WebFingerResponse`] into an axum response in an axum route
    /// handler. The response will be serialized as JSON and the `Content-Type` header will be set
    /// to `application/jrd+json`.
    ///
    /// See the [axum example] for more information.
    ///
    /// # Example
    ///
    /// ```rust
    /// use axum::response::IntoResponse;
    /// use webfinger_rs::{Link, WebFingerRequest, WebFingerResponse};
    ///
    /// async fn handler(request: WebFingerRequest) -> impl IntoResponse {
    ///     // ... your code to handle the webfinger request ...
    ///     let subject = request.resource.to_string();
    ///     let link = Link::builder("http://webfinger.net/rel/profile-page")
    ///         .href(format!("https://example.com/profile/{subject}"));
    ///     WebFingerResponse::builder(subject).link(link).build()
    /// }
    /// ```
    ///
    /// [axum example]:
    ///     http://github.com/joshka/webfinger-rs/blob/main/webfinger-rs/examples/axum.rs
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

    /// Extracts a [`WebFingerRequest`] from the request parts.
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
    /// use webfinger_rs::WebFingerRequest;
    ///
    /// async fn handler(request: WebFingerRequest) -> impl IntoResponse {
    ///     let WebFingerRequest {
    ///         host,
    ///         resource,
    ///         rels,
    ///     } = request;
    ///     // ... your code to handle the webfinger request ...
    /// # webfinger_rs::WebFingerResponse::new(resource.to_string())
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

        Ok(WebFingerRequest {
            host,
            resource,
            rels,
        })
    }
}

#[cfg(test)]
mod tests {
    use axum::{body::Body, routing::get};
    use http::{Request, Response};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use crate::WELL_KNOWN_PATH;

    use super::*;

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
