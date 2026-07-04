//! Runtime-neutral viewer HTTP behavior.
//!
//! This module owns the route policy, htmx contract, shared response headers, and lookup form
//! handling for both the Cloudflare Worker and native Axum runtimes. Runtime adapters remain
//! responsible for extracting requests, building concrete responses, and installing logging.

use std::future::Future;

use http::Method;
use tracing::{error, info};
use url::{Url, form_urlencoded};

use crate::config::LookupConfig;
use crate::lookup::{LookupError, LookupPolicy, LookupRequest, LookupResult, log_lookup_error};
use crate::view;

pub const API_PATH: &str = "/api/lookup";
pub const MAX_LOOKUP_FORM_BYTES: usize = 16 * 1024;

// The page embeds htmx, app.js, and app.css into one response so path-mounted deployments do not
// need asset routes. That requires `unsafe-inline` for script and style execution. Keep this
// tradeoff visible when tightening CSP later: moving to nonce/hash CSP also means changing how
// `view::page` injects embedded assets. Even with inline allowances, this policy blocks remote
// script, object, frame, base-uri, form, and cross-origin connection surfaces.
const CONTENT_SECURITY_POLICY: &str = concat!(
    "default-src 'none'; ",
    "script-src 'unsafe-inline'; ",
    "style-src 'unsafe-inline'; ",
    "connect-src 'self'; ",
    "base-uri 'none'; ",
    "form-action 'self'; ",
    "frame-ancestors 'none'"
);

/// Runtime-neutral HTTP response produced by the viewer app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewerResponse {
    pub status: u16,
    pub headers: Vec<ViewerHeader>,
    pub body: Vec<u8>,
}

/// HTTP header returned by the shared viewer app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewerHeader {
    pub name: &'static str,
    pub value: String,
}

/// Raw lookup fields submitted by the htmx form.
///
/// The POST body is the only input that can trigger an outbound WebFinger fetch. These same fields
/// also produce the pushed viewer URL, keeping history and shareable URLs as form state without
/// making a plain GET page load perform network I/O against a target server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LookupForm {
    /// Resource string from the required form field.
    pub resource: Option<String>,

    /// Raw relation values from the optional text box and preset checkboxes.
    pub rels: Vec<String>,
}

/// Returns true when the URL path is the viewer htmx endpoint.
pub fn is_lookup_path(url: &Url) -> bool {
    url.path().ends_with(API_PATH)
}

/// Serves non-lookup viewer routes.
///
/// Other `GET` and `HEAD` requests serve the UI shell, which lets Cloudflare or Axum route
/// `/webfinger` and nested browser refreshes to the same Rust app.
pub fn serve_page_or_error(method: &Method, url: &Url) -> ViewerResponse {
    if *method == Method::GET || *method == Method::HEAD {
        log_viewer_request(method, url.path(), "page");
        return html_response(&view::page(url));
    }

    log_viewer_request(method, url.path(), "method_not_allowed");
    text_response("method not allowed", 405)
}

/// Applies the lookup endpoint guards that can be checked before reading a form body.
///
/// The order of checks is the endpoint policy: reject unsupported callers first, then reject
/// cross-site browser traffic, then parse viewer input only for the supported same-page htmx path.
pub fn lookup_preflight(
    method: &Method,
    url: &Url,
    is_htmx_request: bool,
    is_cross_site_request: bool,
) -> Result<(), ViewerResponse> {
    if !is_htmx_request {
        log_viewer_request(method, url.path(), "lookup_not_htmx");
        return Err(text_response("not found", 404));
    }
    if is_cross_site_request {
        log_viewer_request(method, url.path(), "lookup_cross_site");
        return Err(text_response("forbidden", 403));
    }
    if *method != Method::POST {
        log_viewer_request(method, url.path(), "lookup_method_not_allowed");
        return Err(text_response("method not allowed", 405));
    }

    Ok(())
}

/// Serves a validated htmx lookup request.
///
/// htmx fragments use `200` for viewer-level errors so the browser swaps the failure UI normally.
/// Target endpoint failures are represented inside the fragment so the user can inspect the target
/// status, headers, and body as the debugging result.
pub async fn serve_lookup<F, Fut>(url: &Url, form: LookupForm, fetch: F) -> ViewerResponse
where
    F: FnOnce(LookupRequest) -> Fut,
    Fut: Future<Output = Result<LookupResult, LookupError>>,
{
    serve_lookup_with_config(url, form, &LookupConfig::default(), fetch).await
}

/// Serves a validated htmx lookup request with runtime-neutral configuration.
pub async fn serve_lookup_with_config<F, Fut>(
    url: &Url,
    form: LookupForm,
    config: &LookupConfig,
    fetch: F,
) -> ViewerResponse
where
    F: FnOnce(LookupRequest) -> Fut,
    Fut: Future<Output = Result<LookupResult, LookupError>>,
{
    let history_url = viewer_history_url(url, &form);
    let policy = LookupPolicy::from_viewer_url_and_config(url, config);
    let lookup_request =
        LookupRequest::from_form_values(form.resource.clone(), form.rels.clone(), &policy);
    let request = match lookup_request {
        Ok(request) => request,
        Err(error) => {
            error!(%error, "webfinger lookup input error");
            let html = view::lookup_error(&error, None);
            return html_fragment_response(&html, Some(&history_url));
        }
    };

    let target_url = request.target_url().to_string();
    let result = match fetch(request).await {
        Ok(result) => result,
        Err(error) => {
            log_lookup_error(&error);
            error!(%error, "webfinger lookup runtime error");
            let html = view::lookup_error(&error, Some(&target_url));
            return html_fragment_response(&html, Some(&history_url));
        }
    };
    html_fragment_response(&view::lookup_result(&result), Some(&history_url))
}

fn html_response(html: &str) -> ViewerResponse {
    response(200, standard_headers("text/html; charset=utf-8"), html)
}

fn text_response(body: &str, status: u16) -> ViewerResponse {
    response(status, standard_headers("text/plain; charset=utf-8"), body)
}

fn html_fragment_response(html: &str, history_url: Option<&str>) -> ViewerResponse {
    let mut headers = standard_headers("text/html; charset=utf-8");
    if let Some(history_url) = history_url {
        headers.push(ViewerHeader {
            name: "hx-push-url",
            value: history_url.to_string(),
        });
    }
    response(200, headers, html)
}

fn response(status: u16, headers: Vec<ViewerHeader>, body: &str) -> ViewerResponse {
    ViewerResponse {
        status,
        headers,
        body: body.as_bytes().to_vec(),
    }
}

fn standard_headers(content_type: &str) -> Vec<ViewerHeader> {
    vec![
        ViewerHeader {
            name: "content-type",
            value: content_type.to_string(),
        },
        ViewerHeader {
            name: "cache-control",
            value: "no-store".to_string(),
        },
        ViewerHeader {
            name: "content-security-policy",
            value: CONTENT_SECURITY_POLICY.to_string(),
        },
        ViewerHeader {
            name: "x-content-type-options",
            value: "nosniff".to_string(),
        },
        ViewerHeader {
            name: "referrer-policy",
            value: "no-referrer".to_string(),
        },
        ViewerHeader {
            name: "permissions-policy",
            value: "clipboard-write=(self)".to_string(),
        },
    ]
}

/// Returns the viewer URL that represents a lookup in browser history.
///
/// htmx submits to `/api/lookup`, but browser history should stay on the user-facing viewer path
/// such as `/webfinger?resource=acct%3Aalice%40example.com`. The query is rebuilt from supported
/// form fields so empty optional `rel` controls and unrelated routing parameters do not pollute
/// shareable URLs.
fn viewer_history_url(lookup_url: &Url, form: &LookupForm) -> String {
    let viewer_path = lookup_url
        .path()
        .strip_suffix(API_PATH)
        .filter(|path| !path.is_empty())
        .unwrap_or("/");
    let query = viewer_history_query(form);
    if query.is_empty() {
        viewer_path.to_string()
    } else {
        format!("{viewer_path}?{query}")
    }
}

/// Builds the canonical viewer query string from submitted form values.
///
/// Relation filters are accepted either as repeated `rel` parameters or comma/newline-separated
/// text box values. Splitting them here makes the address bar match the POST body that
/// `LookupRequest` sends to the target endpoint.
fn viewer_history_query(form: &LookupForm) -> String {
    let mut query = form_urlencoded::Serializer::new(String::new());
    if let Some(resource) = form.resource.as_deref().filter(|value| !value.is_empty()) {
        query.append_pair("resource", resource);
    }
    for value in &form.rels {
        for rel in value.split([',', '\n']).map(str::trim) {
            if !rel.is_empty() {
                query.append_pair("rel", rel);
            }
        }
    }
    query.finish()
}

/// Logs viewer request handling decisions.
///
/// Runtime entrypoints install their own tracing subscribers. Keep the outcome vocabulary stable so
/// Worker dashboard filters and native logs can answer "was the viewer hit" and "which guard
/// rejected this request" without exposing headers or other browser metadata.
fn log_viewer_request(method: &Method, path: &str, outcome: &str) {
    info!(method = %method, path, outcome, "webfinger viewer request");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_url_uses_viewer_path_not_api_path() {
        let url = Url::parse("https://example.com/webfinger/api/lookup").unwrap();
        let form = LookupForm {
            resource: Some("acct:alice@example.com".to_string()),
            rels: vec!["self".to_string()],
        };

        assert_eq!(
            viewer_history_url(&url, &form),
            "/webfinger?resource=acct%3Aalice%40example.com&rel=self"
        );
    }

    #[test]
    fn history_url_splits_and_omits_empty_relation_filters() {
        let url = Url::parse("https://example.com/webfinger/api/lookup").unwrap();
        let form = LookupForm {
            resource: Some("acct:alice@example.com".to_string()),
            rels: vec!["".to_string(), "self, issuer".to_string()],
        };

        assert_eq!(
            viewer_history_url(&url, &form),
            "/webfinger?resource=acct%3Aalice%40example.com&rel=self&rel=issuer"
        );
    }

    #[test]
    fn history_url_without_form_values_keeps_viewer_path() {
        let url = Url::parse("https://example.com/webfinger/api/lookup").unwrap();
        let form = LookupForm {
            resource: None,
            rels: Vec::new(),
        };

        assert_eq!(viewer_history_url(&url, &form), "/webfinger");
    }

    #[test]
    fn lookup_preflight_rejects_non_htmx_before_method() {
        let url = Url::parse("https://example.com/webfinger/api/lookup").unwrap();
        let response = lookup_preflight(&Method::GET, &url, false, false).unwrap_err();

        assert_eq!(response.status, 404);
    }
}
