//! Native Axum adapter for `webfinger-service`.
//!
//! This crate turns a [`webfinger_service::StaticConfigProvider`] into an Axum router that serves
//! the WebFinger endpoint at `/.well-known/webfinger`. It is intended for local development,
//! simple native deployments, and tests that need the same HTTP mapping as the Cloudflare Worker
//! without running Wrangler.
//!
//! The router accepts only `GET /.well-known/webfinger`, plus `GET /health` for local health
//! checks. It maps malformed WebFinger queries to `400`, unknown resources to `404`, unsupported
//! methods to `405`, and successful responses to `application/jrd+json`.

use std::convert::Infallible;

use axum::body::Body;
use axum::extract::FromRequestParts;
use axum::http::{Method, Request, StatusCode, header};
use axum::response::{IntoResponse, Response};
use tower::service_fn;
use tower_http::trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tracing::{Level, info};
use webfinger_rs::{WELL_KNOWN_PATH, WebFingerRequest};
use webfinger_service::StaticConfigProvider;

/// Builds a native Axum router for a static configuration provider.
///
/// The returned router uses a fallback service so it can make method and path decisions in one
/// place. It also installs a Tower HTTP trace layer; configure `tracing-subscriber` in the binary
/// or test harness to see request and response logs.
pub fn axum_router(provider: StaticConfigProvider) -> axum::Router {
    axum::Router::new()
        .fallback_service(service_fn(move |request: Request<Body>| {
            let provider = provider.clone();
            async move {
                let response = serve_static_http(provider, request).await;
                Ok::<_, Infallible>(response)
            }
        }))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_request(DefaultOnRequest::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
}

async fn serve_static_http<B>(provider: StaticConfigProvider, request: Request<B>) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    if method != Method::GET {
        log_webfinger_request(&method, &path, "method_not_allowed");
        return (
            StatusCode::METHOD_NOT_ALLOWED,
            [(header::ALLOW, Method::GET.as_str())],
            "method not allowed",
        )
            .into_response();
    }
    if path == "/health" {
        log_webfinger_request(&method, &path, "health");
        return "OK".into_response();
    }
    if path != WELL_KNOWN_PATH {
        log_webfinger_request(&method, &path, "not_found");
        return (StatusCode::NOT_FOUND, "not found").into_response();
    }

    let (mut parts, _body) = request.into_parts();
    let request = match WebFingerRequest::from_request_parts(&mut parts, &()).await {
        Ok(request) => {
            log_webfinger_request(&method, &path, "lookup");
            request
        }
        Err(rejection) => {
            log_webfinger_request(&method, &path, "bad_request");
            return rejection.into_response();
        }
    };
    match provider.resolve_config(&request) {
        Some(response) => response.into_response(),
        None => (StatusCode::NOT_FOUND, "resource not found").into_response(),
    }
}

fn log_webfinger_request(method: &Method, path: &str, outcome: &str) {
    info!(method = %method, path, outcome, "webfinger service request");
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use axum::http::header;
    use http::Request;
    use webfinger_rs::WebFingerResponse;
    use webfinger_service::EXAMPLE_CONFIG;

    #[tokio::test]
    async fn maps_unknown_resource_to_not_found() {
        let response = call("/.well-known/webfinger?resource=acct:bob@example.com").await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn maps_unsupported_method_to_method_not_allowed() {
        let provider = StaticConfigProvider::from_toml(EXAMPLE_CONFIG).unwrap();
        let request = Request::builder()
            .method(Method::POST)
            .uri("/.well-known/webfinger?resource=acct:alice@example.com")
            .header("host", "example.com")
            .body(Body::empty())
            .unwrap();

        let response = serve_static_http(provider, request).await;

        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
        assert_eq!(response.headers().get(header::ALLOW).unwrap(), "GET");
    }

    #[tokio::test]
    async fn maps_malformed_query_to_bad_request() {
        let response = call("/.well-known/webfinger").await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn successful_response_uses_jrd_content_type() {
        let response = call("/.well-known/webfinger?resource=acct:alice@example.com").await;
        let content_type = response.headers().get(http::header::CONTENT_TYPE).unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(content_type, "application/jrd+json");

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let response: WebFingerResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(response.subject.as_ref(), "acct:alice@example.com");
    }

    async fn call(uri: &str) -> axum::response::Response {
        let provider = StaticConfigProvider::from_toml(EXAMPLE_CONFIG).unwrap();
        let request = Request::builder()
            .uri(uri)
            .header("host", "example.com")
            .body(Body::empty())
            .unwrap();

        serve_static_http(provider, request).await
    }
}
