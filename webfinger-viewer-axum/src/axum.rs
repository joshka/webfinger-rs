//! Axum HTTP adapter for the viewer.
//!
//! This module owns native request extraction, response construction, and reqwest-backed target
//! fetches. Shared route policy, htmx behavior, and rendering live in `app` so the native runtime
//! stays aligned with the Cloudflare Worker deployment.

use std::convert::Infallible;

use ::axum::Router;
use ::axum::body::{Body, Bytes, to_bytes};
use ::axum::extract::State;
use ::axum::http::{HeaderMap, Request, StatusCode, Uri};
use ::axum::response::Response;
use ::axum::routing::any;
use futures_util::TryStreamExt;
use tower_http::trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tracing::{Level, info, instrument};
use url::{Url, form_urlencoded};
use webfinger_viewer::app::{self, LookupForm, MAX_LOOKUP_FORM_BYTES, ViewerResponse};
use webfinger_viewer::config::LookupConfig;
use webfinger_viewer::lookup::{
    ACCEPT_HEADER, CappedBody, LookupError, LookupRequest, LookupResult, LookupResultParts,
    MAX_BODY_BYTES, cap_body_bytes, log_lookup_result,
};

/// Builds the native Axum viewer application.
pub fn router(client: reqwest::Client, lookup_config: LookupConfig) -> Router {
    Router::new()
        .fallback(any(handle))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_request(DefaultOnRequest::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        .with_state(AppState {
            client,
            lookup_config,
        })
}

#[derive(Clone)]
struct AppState {
    client: reqwest::Client,
    lookup_config: LookupConfig,
}

async fn handle(
    State(state): State<AppState>,
    request: Request<Body>,
) -> Result<Response, Infallible> {
    let (parts, body) = request.into_parts();
    let url = request_url(&parts.headers, &parts.uri);
    let response = match url {
        Ok(url) if app::is_lookup_path(&url) => {
            handle_lookup(state, parts.method, parts.headers, body, url).await
        }
        Ok(url) => app::serve_page_or_error(&parts.method, &url),
        Err(error) => ViewerResponse {
            status: 400,
            headers: Vec::new(),
            body: error.into_bytes(),
        },
    };

    Ok(axum_response(response))
}

async fn handle_lookup(
    state: AppState,
    method: http::Method,
    headers: HeaderMap,
    body: Body,
    url: Url,
) -> ViewerResponse {
    let is_htmx_request = headers.contains_key("hx-request");
    let is_cross_site_request = headers
        .get("sec-fetch-site")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value == "cross-site");
    if let Err(response) =
        app::lookup_preflight(&method, &url, is_htmx_request, is_cross_site_request)
    {
        return response;
    }

    let form = match axum_lookup_form(body).await {
        Ok(form) => form,
        Err(response) => return response,
    };
    app::serve_lookup_with_config(&url, form, &state.lookup_config, |request| async move {
        fetch_webfinger_reqwest(&state.client, &request).await
    })
    .await
}

/// Fetches the target WebFinger endpoint with reqwest for the native Axum runtime.
///
/// The native runtime mirrors the Worker fetch policy: redirects are not followed, the same Accept
/// preference is sent, and the response body is capped before rendering.
#[instrument(skip(client, request), fields(target_url = %request.target_url()))]
async fn fetch_webfinger_reqwest(
    client: &reqwest::Client,
    request: &LookupRequest,
) -> Result<LookupResult, LookupError> {
    info!("fetching webfinger resource");
    let response = client
        .get(request.target_url().clone())
        .header(reqwest::header::ACCEPT, ACCEPT_HEADER)
        .send()
        .await
        .map_err(|error| LookupError::transport("Native fetch", error))?;
    let status = response.status().as_u16();
    let headers = response.headers().clone();
    let content_type = header_value(&headers, reqwest::header::CONTENT_TYPE);
    let redirect_location = header_value(&headers, reqwest::header::LOCATION);
    let body = read_capped_reqwest_body(response).await?;
    log_lookup_result(request, status, content_type.as_deref(), body.truncated);

    Ok(LookupResult::new(LookupResultParts {
        request_url: request.target_url().to_string(),
        redirect_location,
        resource: request.resource().to_string(),
        rels: request.rels().to_vec(),
        status,
        content_type,
        body: body.text,
        truncated: body.truncated,
    }))
}

fn header_value(
    headers: &reqwest::header::HeaderMap,
    name: reqwest::header::HeaderName,
) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
}

async fn read_capped_reqwest_body(response: reqwest::Response) -> Result<CappedBody, LookupError> {
    let mut stream = response.bytes_stream();
    let mut bytes = Vec::new();
    let read_limit = MAX_BODY_BYTES + 1;

    while bytes.len() < read_limit {
        let Some(chunk) = stream
            .try_next()
            .await
            .map_err(|error| LookupError::transport("Native fetch", error))?
        else {
            break;
        };
        let remaining = read_limit - bytes.len();
        if chunk.len() > remaining {
            bytes.extend_from_slice(&chunk[..remaining]);
            break;
        }
        bytes.extend_from_slice(&chunk);
    }

    Ok(cap_body_bytes(bytes))
}

async fn axum_lookup_form(body: Body) -> Result<LookupForm, ViewerResponse> {
    let bytes = match to_bytes(body, MAX_LOOKUP_FORM_BYTES).await {
        Ok(bytes) => bytes,
        Err(error) => {
            return Err(ViewerResponse {
                status: 400,
                headers: Vec::new(),
                body: format!("invalid form body: {error}").into_bytes(),
            });
        }
    };

    Ok(parse_lookup_form(&bytes))
}

fn parse_lookup_form(bytes: &Bytes) -> LookupForm {
    let mut resource = None;
    let mut rels = Vec::new();
    for (key, value) in form_urlencoded::parse(bytes) {
        if key == "resource" {
            resource = Some(value.into_owned());
        } else if key == "rel" {
            rels.push(value.into_owned());
        }
    }

    LookupForm { resource, rels }
}

fn request_url(headers: &HeaderMap, uri: &Uri) -> Result<Url, String> {
    if uri.scheme_str().is_some() {
        return Url::parse(&uri.to_string()).map_err(|error| error.to_string());
    }

    let scheme = headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("http");
    let host = headers
        .get("host")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("127.0.0.1:8788");
    Url::parse(&format!("{scheme}://{host}{uri}")).map_err(|error| error.to_string())
}

fn axum_response(response: ViewerResponse) -> Response {
    let status = StatusCode::from_u16(response.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let mut builder = Response::builder().status(status);
    for header in response.headers {
        builder = builder.header(header.name, header.value);
    }
    builder
        .body(Body::from(response.body))
        .unwrap_or_else(|error| {
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from(format!("failed to build response: {error}")))
                .expect("static response is valid")
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_repeated_relation_fields() {
        let form = parse_lookup_form(&Bytes::from_static(
            b"resource=acct%3Aalice%40example.com&rel=self&rel=issuer",
        ));

        assert_eq!(form.resource, Some("acct:alice@example.com".to_string()));
        assert_eq!(form.rels, vec!["self", "issuer"]);
    }

    #[test]
    fn builds_local_request_url_from_host_header() {
        let mut headers = HeaderMap::new();
        headers.insert("host", "localhost:8788".parse().unwrap());
        let url = request_url(&headers, &"/webfinger/api/lookup".parse().unwrap()).unwrap();

        assert_eq!(url.as_str(), "http://localhost:8788/webfinger/api/lookup");
    }
}
