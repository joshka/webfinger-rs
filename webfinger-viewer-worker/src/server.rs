//! HTTP boundary for the viewer Worker.
//!
//! This module knows about Cloudflare Worker request and response types, static UI delivery, and
//! the htmx fragment contract used by the browser. It intentionally does not know how WebFinger
//! target URLs are derived or how JRD fields are displayed; those belong to `lookup` and `view` so
//! protocol behavior and rendering can be changed without reading response-header plumbing.
//!
//! `/api/lookup` is not a general JSON API. It is a same-page htmx POST endpoint for the bundled
//! form: callers must send `HX-Request`, and obvious cross-site browser requests are rejected using
//! Fetch Metadata. This is not authentication, but it avoids advertising the Worker as a public
//! server-side lookup API and prevents normal cross-origin browser reads because no CORS headers are
//! emitted. Malformed viewer input and Worker-fetch failures still render HTML fragments with
//! status `200` so htmx swaps them into `#results`. Target WebFinger status is displayed inside the
//! fragment rather than encoded as the Worker response status.

use ::worker::{Context, Env, FormEntry, Method, Request, Response};
use tracing::{error, info};
use url::{Url, form_urlencoded};

use crate::lookup::{LookupPolicy, LookupRequest, fetch_webfinger, log_lookup_error};
use crate::view;

const API_PATH: &str = "/api/lookup";

// The page embeds htmx, app.js, and app.css into one Worker response so path-mounted deployments do
// not need asset routes. That requires `unsafe-inline` for script and style execution. Keep this
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

/// Serves one Cloudflare Worker request.
///
/// Requests ending in `/api/lookup` are treated as same-page htmx fragment requests so the viewer
/// can be deployed under a path prefix such as `/webfinger`. Other `GET` and `HEAD` requests serve
/// the UI shell, which lets Cloudflare route `/webfinger` and nested browser refreshes to the same
/// Worker script.
pub async fn serve(request: Request, _env: Env, _ctx: Context) -> worker::Result<Response> {
    let method = request.method();
    let url = request.url()?;
    let path = url.path().to_string();

    if path.ends_with(API_PATH) {
        return serve_lookup(request).await;
    }

    if method == Method::Get || method == Method::Head {
        log_viewer_request(&method, url.path(), "page");
        return html_response(&view::page(&url));
    }

    log_viewer_request(&method, url.path(), "method_not_allowed");
    text_response("method not allowed", 405)
}

/// Serves the htmx lookup endpoint used by the bundled form.
///
/// Keep method and URL extraction inside this helper so the request remains the single source of
/// truth. This makes the handler easier to validate: tests or manual curls only need to vary the
/// actual Worker request, not a parallel set of derived arguments. The order of checks is also the
/// endpoint policy: reject unsupported callers first, then reject cross-site browser traffic, then
/// parse viewer input only for the supported same-page htmx path.
async fn serve_lookup(mut request: Request) -> worker::Result<Response> {
    let method = request.method();
    let url = request.url()?;

    if !is_htmx_request(&request)? {
        log_viewer_request(&method, url.path(), "lookup_not_htmx");
        return text_response("not found", 404);
    }
    if is_cross_site_request(&request)? {
        log_viewer_request(&method, url.path(), "lookup_cross_site");
        return text_response("forbidden", 403);
    }

    if method != Method::Post {
        log_viewer_request(&method, url.path(), "lookup_method_not_allowed");
        return text_response("method not allowed", 405);
    }

    let form = LookupForm::from_request(&mut request).await?;
    let history_url = viewer_history_url(&url, &form);
    let policy = LookupPolicy::from_viewer_url(&url);
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

    let result = match fetch_webfinger(&request).await {
        Ok(result) => result,
        Err(error) => {
            log_lookup_error(&error);
            error!(%error, "webfinger lookup worker error");
            let target_url = request.target_url().as_str();
            let html = view::lookup_error(&error, Some(target_url));
            return html_fragment_response(&html, Some(&history_url));
        }
    };
    html_fragment_response(&view::lookup_result(&result), Some(&history_url))
}

/// Returns the full viewer page shell.
///
/// The page is rendered by `view` rather than loaded from a static HTML file so the source can stay
/// split across Askama templates, CSS, and JavaScript while still deploying as a single Worker
/// response.
fn html_response(html: &str) -> worker::Result<Response> {
    Response::builder()
        .with_header("cache-control", "no-store")?
        .with_header("content-security-policy", CONTENT_SECURITY_POLICY)?
        .with_header("x-content-type-options", "nosniff")?
        .with_header("referrer-policy", "no-referrer")?
        .with_header("permissions-policy", "clipboard-write=(self)")?
        .from_html(html)
}

/// Returns a small plain-text response for non-UI routing errors.
///
/// The viewer shell handles normal browser traffic, and `/api/lookup` has its own htmx fragment
/// contract. This helper is intentionally reserved for outer routing failures, rejected non-htmx
/// lookup calls, and unsupported methods that should not swap into the result panel.
fn text_response(body: &str, status: u16) -> worker::Result<Response> {
    let response = Response::builder()
        .with_status(status)
        .with_header("content-type", "text/plain; charset=utf-8")?
        .with_header("cache-control", "no-store")?
        .with_header("content-security-policy", CONTENT_SECURITY_POLICY)?
        .with_header("x-content-type-options", "nosniff")?
        .with_header("referrer-policy", "no-referrer")?
        .with_header("permissions-policy", "clipboard-write=(self)")?
        .fixed(body.as_bytes().to_vec());
    Ok(response)
}

/// Returns a browser-swappable htmx fragment.
///
/// htmx fragments use `200` for viewer-level errors so the browser swaps the failure UI normally.
/// Target endpoint failures should be represented inside the fragment so the user can inspect the
/// target status, headers, and body as the debugging result.
fn html_fragment_response(html: &str, history_url: Option<&str>) -> worker::Result<Response> {
    let mut builder = Response::builder()
        .with_status(200)
        .with_header("content-type", "text/html; charset=utf-8")?
        .with_header("cache-control", "no-store")?
        .with_header("content-security-policy", CONTENT_SECURITY_POLICY)?
        .with_header("x-content-type-options", "nosniff")?
        .with_header("referrer-policy", "no-referrer")?
        .with_header("permissions-policy", "clipboard-write=(self)")?;
    if let Some(history_url) = history_url {
        builder = builder.with_header("hx-push-url", history_url)?;
    }
    let response = builder.fixed(html.as_bytes().to_vec());
    Ok(response)
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

/// Raw lookup fields submitted by the htmx form.
///
/// The POST body is the only input that can trigger an outbound WebFinger fetch. These same fields
/// also produce the pushed viewer URL, keeping history and shareable URLs as form state without
/// making a plain GET page load perform network I/O against a target server.
struct LookupForm {
    /// Resource string from the required form field.
    resource: Option<String>,

    /// Raw relation values from the optional text box and preset checkboxes.
    rels: Vec<String>,
}

impl LookupForm {
    /// Reads the htmx form body from the Worker request.
    ///
    /// `Request::form_data` handles the browser's `application/x-www-form-urlencoded` submission.
    /// File values are ignored because the viewer form has no file controls; treating them as
    /// absent keeps malformed programmatic posts on the normal validation path.
    async fn from_request(request: &mut Request) -> worker::Result<Self> {
        let form = request.form_data().await?;
        let resource = form.get_field("resource");
        let rels = form
            .get_all("rel")
            .unwrap_or_default()
            .into_iter()
            .filter_map(|entry| match entry {
                FormEntry::Field(value) => Some(value),
                FormEntry::File(_) => None,
            })
            .collect();

        Ok(Self { resource, rels })
    }
}

/// Returns true when the request came from htmx.
///
/// `HX-Request` is not a secret; direct clients can spoof it. The point is to keep the endpoint's
/// supported contract aligned with the page form and reject accidental or generic JSON-style use.
fn is_htmx_request(request: &Request) -> worker::Result<bool> {
    Ok(request.headers().get("hx-request")?.is_some())
}

/// Returns true for obvious cross-site browser requests.
///
/// Fetch Metadata is browser-provided defense in depth. It is not authentication and non-browser
/// clients can omit or spoof it, but it blocks the normal "other site embeds this endpoint as a
/// cross-origin htmx/fetch target" path without requiring a CSRF token for an unauthenticated
/// read-only tool.
fn is_cross_site_request(request: &Request) -> worker::Result<bool> {
    Ok(matches!(
        request.headers().get("sec-fetch-site")?.as_deref(),
        Some("cross-site")
    ))
}

/// Logs Worker request handling decisions.
///
/// The wasm entrypoint installs a console-backed tracing subscriber, so these events appear in
/// Wrangler tail and Cloudflare dashboard logs. Keep the outcome vocabulary stable so dashboard
/// filters can answer "was the Worker hit" and "which guard rejected this request" without
/// exposing headers or other browser metadata.
fn log_viewer_request(method: &Method, path: &str, outcome: &str) {
    info!(method = ?method, path, outcome, "webfinger viewer request");
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
}
