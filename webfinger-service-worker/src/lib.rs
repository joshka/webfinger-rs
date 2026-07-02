//! Cloudflare Worker runtime for serving WebFinger responses from editable KV configuration.
//!
//! Shared TOML parsing, provider traits, and relation filtering live in `webfinger-service`.
//! This crate owns the Cloudflare Worker boundary: KV configuration, HTTP mapping, wasm logging,
//! and the `fetch` entrypoint.
//!
//! The default Worker entrypoint reads TOML from Workers KV binding `WEBFINGER_CONFIG` and key
//! `webfinger.toml`. Custom Workers can reuse the same HTTP mapping by constructing
//! [`Worker::new`] with any [`webfinger_service::WebFingerProvider`] implementation or by calling
//! [`serve_with_provider`].
//!
//! Public HTTP error bodies intentionally avoid detailed provider/configuration failures, except
//! for the missing setup key message. Detailed failures are logged through `tracing` for Wrangler
//! tail and Cloudflare Worker logs.

mod kv;
mod observability;

use axum::extract::FromRequestParts;
use axum::http::{Method, StatusCode, header};
use axum::response::{IntoResponse, Response};
use thiserror::Error;
use tracing::{error, info, instrument};
use webfinger_rs::{WELL_KNOWN_PATH, WebFingerRequest, WebFingerResponse};
use webfinger_service::{ProviderError, WebFingerProvider};
use worker::{Context, Env, HttpRequest};

pub use crate::kv::{KvConfigProvider, WEBFINGER_CONFIG_BINDING};

/// A WebFinger Worker backed by a caller-provided provider.
///
/// Use this type when the data source is not the default Workers KV key. The provider owns
/// resource lookup and relation filtering; the Worker owns method/path checks, WebFinger request
/// extraction, response headers, status codes, and logging.
#[derive(Debug, Clone)]
pub struct Worker<P> {
    provider: P,
}

impl<P> Worker<P> {
    /// Creates a Worker from a provider.
    pub fn new(provider: P) -> Self {
        Self { provider }
    }
}

impl<P> Worker<P>
where
    P: WebFingerProvider,
{
    /// Serves one HTTP request with this Worker's provider.
    ///
    /// This method is useful from custom `#[worker::event(fetch)]` functions after the caller has
    /// constructed a provider from bindings or other Worker state.
    pub async fn serve(&self, request: HttpRequest) -> Response {
        serve_with_provider(&self.provider, request).await
    }
}

/// Serves one Cloudflare Worker HTTP request using the default KV provider.
///
/// This is the function used by the bundled `fetch` entrypoint. It expects the Worker environment
/// to contain KV binding [`WEBFINGER_CONFIG_BINDING`] and reads configuration from
/// [`webfinger_service::WEBFINGER_CONFIG_KEY`].
///
/// # Errors
///
/// Returns a Worker error if the required KV binding is missing. Provider failures that happen
/// after the binding is available are mapped into HTTP responses and logged.
pub async fn serve(request: HttpRequest, env: Env, _ctx: Context) -> worker::Result<Response> {
    let provider = KvConfigProvider::from_env(&env)
        .map_err(|error| worker::Error::RustError(error.to_string()))?;
    Ok(Worker::new(provider).serve(request).await)
}

/// Serves one HTTP request with a caller-provided WebFinger provider.
///
/// This is the lowest-level reusable HTTP mapping in the Worker crate. It accepts only
/// `GET /.well-known/webfinger`, plus `GET /health`, and maps provider results into WebFinger HTTP
/// responses. Use [`Worker`] when you want to hold a provider value and serve multiple requests
/// through the same wrapper.
pub async fn serve_with_provider<P>(provider: &P, request: HttpRequest) -> Response
where
    P: WebFingerProvider,
{
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
    webfinger(provider, request).await.into_response()
}

#[instrument(skip(provider, request), fields(resource = %request.resource))]
async fn webfinger<P>(
    provider: &P,
    request: WebFingerRequest,
) -> Result<WebFingerResponse, HttpError>
where
    P: WebFingerProvider,
{
    match provider.resolve(&request).await? {
        Some(response) => {
            info!("resolved webfinger response");
            Ok(response)
        }
        None => Err(HttpError::NotFound),
    }
}

fn log_webfinger_request(method: &Method, path: &str, outcome: &str) {
    info!(method = %method, path, outcome, "webfinger service request");
}

#[derive(Debug, Error)]
enum HttpError {
    #[error("resource not found")]
    NotFound,

    #[error(transparent)]
    Provider(#[from] ProviderError),
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        match self {
            HttpError::NotFound => (StatusCode::NOT_FOUND, "resource not found").into_response(),
            HttpError::Provider(error) => {
                error!(?error, "webfinger provider failed");
                let message = match error {
                    ProviderError::MissingConfig { key } => format!(
                        "WebFinger is not configured. Add TOML configuration to key `{key}`."
                    ),
                    _ => {
                        "WebFinger provider failed. Check the Worker logs for details.".to_string()
                    }
                };
                (StatusCode::INTERNAL_SERVER_ERROR, message).into_response()
            }
        }
    }
}

/// Cloudflare Worker lifecycle entrypoint for logging setup.
#[worker::event(start)]
fn start() {
    observability::init();
}

/// Cloudflare Worker fetch entrypoint.
#[worker::event(fetch)]
async fn fetch(request: HttpRequest, env: Env, ctx: Context) -> worker::Result<Response> {
    observability::init();
    serve(request, env, ctx).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use axum::http::{Request, header};
    use webfinger_service::{EXAMPLE_CONFIG, StaticConfigProvider, WEBFINGER_CONFIG_KEY};
    use worker::Body;

    #[tokio::test]
    async fn maps_unknown_resource_to_not_found() {
        let response = call("/.well-known/webfinger?resource=acct:bob@example.com").await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn maps_unknown_path_to_not_found() {
        let response = call("/").await;

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

        let response = Worker::new(provider).serve(request).await;

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
        let content_type = response.headers().get(header::CONTENT_TYPE).unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(content_type, "application/jrd+json");

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let response: WebFingerResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(response.subject.as_ref(), "acct:alice@example.com");
    }

    #[tokio::test]
    async fn missing_config_returns_setup_error() {
        let request = Request::builder()
            .uri("/.well-known/webfinger?resource=acct:alice@example.com")
            .header("host", "example.com")
            .body(Body::empty())
            .unwrap();

        let response = Worker::new(MissingConfigProvider).serve(request).await;

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body = String::from_utf8(body.to_vec()).unwrap();
        assert!(body.contains(WEBFINGER_CONFIG_KEY));
        assert!(body.contains("Add TOML configuration"));
    }

    #[tokio::test]
    async fn provider_errors_do_not_expose_config_details() {
        let request = Request::builder()
            .uri("/.well-known/webfinger?resource=acct:alice@example.com")
            .header("host", "example.com")
            .body(Body::empty())
            .unwrap();

        let response = Worker::new(InvalidConfigProvider).serve(request).await;

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body = String::from_utf8(body.to_vec()).unwrap();
        assert!(body.contains("Check the Worker logs"));
        assert!(!body.contains("acct:alice@example.com"));
    }

    async fn call(uri: &str) -> Response {
        let provider = StaticConfigProvider::from_toml(EXAMPLE_CONFIG).unwrap();
        let request = Request::builder()
            .uri(uri)
            .header("host", "example.com")
            .body(Body::empty())
            .unwrap();

        Worker::new(provider).serve(request).await
    }

    struct MissingConfigProvider;

    impl WebFingerProvider for MissingConfigProvider {
        async fn resolve<'a>(
            &'a self,
            _request: &'a WebFingerRequest,
        ) -> Result<Option<WebFingerResponse>, ProviderError> {
            Err(ProviderError::MissingConfig {
                key: WEBFINGER_CONFIG_KEY.to_string(),
            })
        }
    }

    struct InvalidConfigProvider;

    impl WebFingerProvider for InvalidConfigProvider {
        async fn resolve<'a>(
            &'a self,
            _request: &'a WebFingerRequest,
        ) -> Result<Option<WebFingerResponse>, ProviderError> {
            Err(webfinger_service::Config::from_toml(
                r#"
[[resources]]
resource = "acct:alice@example.com"

[[resources]]
resource = "acct:alice@example.com"
"#,
            )
            .unwrap_err()
            .into())
        }
    }
}
